"""Ask WorldPulse: grounded answers from locally stored records.

1 parse the question → countries/regions, categories, time range, keywords
2 search events and articles (SQLite FTS5 + filters)
3 rank by relevance and freshness
4 answer with templates (or optional local LLM) and attach references
5 say what is missing
"""
from __future__ import annotations

import math
import re
import sqlite3
from datetime import datetime, timedelta, timezone

from .db import uj
from .gazetteer import COUNTRIES, REGION_GROUPS, find_countries
from .processing.classify import LABELS
from .processing.normalize import parse_iso, to_iso

CATEGORY_WORDS = {
    "economics": ["economy", "economic", "inflation", "gdp", "recession", "growth"],
    "financial_markets": ["market", "markets", "stocks", "shares", "currency", "crypto", "bitcoin"],
    "energy": ["oil", "gas", "energy", "opec", "crude", "lng"],
    "trade": ["trade", "tariff", "tariffs", "exports", "imports", "shipping"],
    "natural_disasters": ["earthquake", "earthquakes", "quake", "flood", "floods", "cyclone", "hurricane", "disaster", "disasters", "wildfire", "volcano"],
    "military_security": ["war", "military", "conflict", "attack", "attacks", "security", "missile", "strike", "strikes"],
    "diplomacy": ["diplomacy", "diplomatic", "talks", "summit", "relations", "treaty"],
    "elections": ["election", "elections", "vote", "voting", "poll", "polls"],
    "cybersecurity": ["cyber", "hack", "ransomware", "breach"],
    "public_health": ["health", "outbreak", "disease", "virus", "epidemic"],
    "environment": ["climate", "environment", "emissions", "pollution"],
    "technology": ["technology", "tech", "ai", "chips", "semiconductor"],
    "geopolitics": ["geopolitical", "geopolitics", "sanctions", "coup", "protest", "protests"],
}
HAZARDS = [(r"earthquake|quake|tremor", r"earthquake|quake|tremor|seismic"), (r"flood", r"flood"),
           (r"cyclone|hurricane|typhoon|storm", r"cyclone|hurricane|typhoon|storm"), (r"wildfire|forest fire|bushfire", r"fire"),
           (r"volcan|eruption", r"volcan|eruption"), (r"tsunami", r"tsunami"), (r"drought", r"drought")]
QUESTION_STOP = set("""what which who when where how why is are was were did do does happened happening happen show me tell list give
the a an in on of for about during last past this that these those today yesterday week weeks month months day days hours hour
recent recently latest major significant important any all and or with involving between affecting summarize summarise developments
news events countries country experienced""".split())


def parse_question(q: str, now: datetime | None = None) -> dict:
    now = now or datetime.now(timezone.utc)
    ql = q.lower()
    hours = 168
    m = re.search(r"(?:last|past)\s+(\d+)\s*(hour|hours|day|days|week|weeks|month|months)", ql)
    if m:
        n, unit = int(m.group(1)), m.group(2)
        hours = n * {"h": 1, "d": 24, "w": 168, "m": 720}[unit[0]]
    elif re.search(r"\btoday\b|24 hours|last day", ql):
        hours = 24
    elif "yesterday" in ql:
        hours = 48
    elif re.search(r"this week|last week|past week", ql):
        hours = 168
    elif re.search(r"this month|last month|past month", ql):
        hours = 720
    hours = min(hours, 24 * 45)
    countries = set(find_countries(q).keys())
    for name, members in REGION_GROUPS.items():
        if name in ql:
            countries |= set(members)
            regions = True
    cats = {c for c, words in CATEGORY_WORDS.items() if any(re.search(rf"\b{w}\b", ql) for w in words)}
    min_mag = None
    mm = re.search(r"magnitude\s*(?:above|over|>=?|of at least)?\s*(\d(?:\.\d)?)", ql)
    if mm:
        min_mag = float(mm.group(1))
    elif "significant" in ql and "natural_disasters" in cats:
        min_mag = 5.5
    names = {w.lower() for iso in countries for w in [COUNTRIES[iso].name, *COUNTRIES[iso].aliases]} if countries else set()
    words = [w for w in re.findall(r"[a-zA-Z][a-zA-Z\-]{2,}", q) if w.lower() not in QUESTION_STOP and w.lower() not in names
             and not any(w.lower() in ws for ws in CATEGORY_WORDS.values()) and w.lower() not in REGION_GROUPS]
    return {"hours": hours, "since": to_iso(now - timedelta(hours=hours)), "countries": sorted(countries), "categories": sorted(cats),
            "keywords": words[:8], "min_magnitude": min_mag, "multi_country": len(countries) > 1 and len(countries) <= 4}


def _score(text: str, keywords: list[str], when: str | None, hours: int) -> float:
    s = 1.0
    tl = text.lower()
    for k in keywords:
        if k.lower() in tl:
            s += 2
    t = parse_iso(when)
    if t:
        age_h = (datetime.now(timezone.utc) - t).total_seconds() / 3600
        s *= math.exp(-max(age_h, 0) / max(hours, 24))
    return s


def answer(conn: sqlite3.Connection, question: str, limit: int = 12) -> dict:
    p = parse_question(question)
    where, params = ["last_updated >= ?"], [p["since"]]
    if p["countries"]:
        cs = p["countries"]
        where.append("(" + " OR ".join(["country=?"] * len(cs) + ["related_countries LIKE ?"] * len(cs)) + ")")
        params += cs + [f'%"{c}"%' for c in cs]
    if p["categories"]:
        where.append(f"category IN ({','.join('?' * len(p['categories']))})")
        params += p["categories"]
    if p["min_magnitude"] is not None:
        where.append("magnitude >= ?")
        params.append(p["min_magnitude"])
    evs = [dict(r) for r in conn.execute(f"SELECT * FROM events WHERE {' AND '.join(where)} LIMIT 2000", params)]
    if p["multi_country"]:
        # "China and Taiwan": prefer events that involve all named countries
        both = [e for e in evs if set(p["countries"]) <= set(uj(e["related_countries"], []))]
        if both:
            evs = both
    if p["keywords"] and not p["countries"] and not p["categories"]:
        evs = [e for e in evs if any(k.lower() in e["title"].lower() for k in p["keywords"])]
    hazards = [t for q_rx, t in HAZARDS if re.search(q_rx, question.lower())]
    if hazards:  # "earthquakes" should not return fires
        evs = [e for e in evs if any(re.search(t, e["title"], re.I) for t in hazards)]
    for e in evs:
        bonus = {"instrument_observation": 3, "official_alert": 2.5, "official_statement": 2, "multi_source_reporting": 1.5}.get(e["verification"], 1)
        e["_s"] = _score(e["title"], p["keywords"], e["last_updated"], p["hours"]) * bonus * (1 + len(uj(e["source_refs"], [])) / 6)
    evs.sort(key=lambda e: -e["_s"])
    top = evs[:limit]

    sigs = []
    if p["countries"]:
        sigs = [dict(r) for r in conn.execute(
            f"SELECT * FROM signals WHERE detected_at >= ? AND country IN ({','.join('?' * len(p['countries']))}) ORDER BY score DESC LIMIT 5",
            [p["since"], *p["countries"]])]
    refs, lines = [], []
    for e in top:
        ids = []
        for r in uj(e["source_refs"], [])[:2]:
            refs.append({"id": len(refs) + 1, "title": r["title"], "url": r.get("url"), "publisher": r.get("publisher"), "date": r.get("published_at")})
            ids.append(len(refs))
        lines.append({"text": e["title"], "category": LABELS.get(e["category"], e["category"]), "verification": e["verification"],
                      "country": e["country"], "date": e["started_at"] or e["first_detected"], "refs": ids, "event_id": e["id"]})
    scope = ", ".join(COUNTRIES[c].name for c in p["countries"][:5]) + ("…" if len(p["countries"]) > 5 else "") if p["countries"] else "worldwide"
    period = f"the last {p['hours'] // 24} days" if p["hours"] >= 48 else f"the last {p['hours']} hours"
    topic = ", ".join(LABELS[c].lower() for c in p["categories"]) or "all topics"
    if top:
        summary = f"Found {len(evs)} recorded developments ({scope}; {topic}; {period}). The {len(top)} most relevant are listed with their sources."
    else:
        summary = f"No recorded developments matched ({scope}; {topic}; {period})."
    gaps = []
    if not top:
        gaps.append("Nothing in the local index matched. Try a longer period or fewer filters. Coverage is limited to configured sources.")
    if p["categories"] == ["financial_markets"] or "energy" in p["categories"]:
        gaps.append("For price levels, see the Markets view; a list of news next to price moves does not show that one caused the other.")
    if top and all(e["verification"] == "single_source_report" for e in top):
        gaps.append("All matches are single-source reports and remain unverified.")
    return {"question": question, "interpretation": p, "summary": summary, "items": lines,
            "signals": [{"id": s["id"], "title": s["title"], "confidence": s["confidence"]} for s in sigs],
            "references": refs, "gaps": gaps, "mode": "basic"}
