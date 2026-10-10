"""Derived analysis: events, relationships and OpenSignals.

Everything here reads raw tables and (re)writes derived tables, so it can be
re-run at any time without fetching. Nothing here claims causation: links
between events are labelled observed (a shared fact) or hypothesis.
"""
from __future__ import annotations

import hashlib
import json
import math
import re
import sqlite3
import statistics
from collections import Counter, defaultdict
from datetime import datetime, timedelta, timezone

from .db import j, tx, uj
from .gazetteer import COUNTRIES
from .processing.classify import LABELS
from .processing.dedup import jaccard, tokens
from .processing.normalize import now_iso, parse_iso, to_iso

VERIFICATION = {
    "instrument_observation": "Observed by a scientific instrument network",
    "official_alert": "Official alert from a public monitoring body",
    "official_statement": "Published by a government or international institution",
    "multi_source_reporting": "Reported by two or more different outlets (not independently verified)",
    "single_source_report": "Reported by one outlet so far (unverified claim)",
}


def _hid(*parts: str) -> str:
    return hashlib.sha1("|".join(parts).encode()).hexdigest()[:14]


def _utcnow() -> datetime:
    return datetime.now(timezone.utc)


# ====================================================================== events

def build_events(conn: sqlite3.Connection, days: int = 7) -> int:
    cutoff = to_iso(_utcnow() - timedelta(days=days))
    rows = conn.execute(
        """SELECT a.*, s.source_type FROM articles a JOIN sources s ON s.id=a.source_id
           WHERE a.collected_at >= ? AND a.cluster_id IS NOT NULL""", (cutoff,)).fetchall()
    clusters: dict[str, list[sqlite3.Row]] = defaultdict(list)
    for r in rows:
        clusters[r["cluster_id"]].append(r)
    n = 0
    with tx(conn):
        conn.execute("DELETE FROM article_clusters WHERE last_seen < ?", (to_iso(_utcnow() - timedelta(days=60)),))
        for cid, arts in clusters.items():
            uniq = [a for a in arts if not a["duplicate_of"]] or arts
            publishers = {(a["publisher"] or a["source_id"]).lower() for a in arts}
            times = sorted(a["published_at"] or a["collected_at"] for a in arts)
            cats = Counter(a["category"] for a in arts if a["category"] != "general")
            category = cats.most_common(1)[0][0] if cats else "general"
            cscore: Counter = Counter()
            for a in arts:
                for iso2, conf in uj(a["countries"], {}).items():
                    cscore[iso2] += conf
            countries = [c for c, s in cscore.most_common(5) if s >= 0.9]
            # headline closest to the others
            toks = [tokens(a["norm_title"]) for a in uniq]
            best = max(range(len(uniq)), key=lambda i: sum(jaccard(toks[i], t) for t in toks))
            title = uniq[best]["title"]
            conn.execute("""INSERT OR REPLACE INTO article_clusters(id,title,first_seen,last_seen,article_count,publisher_count,category,countries)
                            VALUES (?,?,?,?,?,?,?,?)""", (cid, title, times[0], times[-1], len(arts), len(publishers), category, j(countries)))
            official = any(a["is_official"] for a in arts)
            if not (len(publishers) >= 2 or official or (category != "general" and countries)):
                continue
            verification = ("official_statement" if official else
                            "multi_source_reporting" if len(publishers) >= 2 else "single_source_report")
            conf = 0.5 + 0.1 * min(len(publishers), 3) + (0.1 if countries else 0) + (0.05 if category != "general" else 0)
            refs = sorted(({"title": a["title"], "url": a["url"], "publisher": a["publisher"], "published_at": a["published_at"],
                            "article_id": a["id"]} for a in uniq), key=lambda x: x["published_at"] or "", reverse=True)[:12]
            _upsert_event(conn, {
                "id": "e_" + cid[2:], "title": title, "category": category,
                "description": next((a["excerpt"] for a in uniq if a["excerpt"]), None),
                "country": countries[0] if countries else None, "related_countries": countries,
                "lat": None, "lon": None, "started_at": times[0], "first_detected": min(a["collected_at"] for a in arts),
                "last_updated": max(a["collected_at"] for a in arts), "origin": "news_cluster", "cluster_id": cid,
                "source_refs": refs, "datasets": [], "verification": verification,
                "extraction_confidence": round(min(conf, 0.95), 2), "magnitude": None,
            })
            n += 1

        # earthquakes M5+ (instrument observations)
        for q in conn.execute("SELECT * FROM quakes WHERE mag >= 5.0 AND time >= ?", (to_iso(_utcnow() - timedelta(days=30)),)):
            title = f"M{q['mag']:.1f} earthquake — {q['place']}"
            _upsert_event(conn, {
                "id": "e_q_" + q["id"], "title": title, "category": "natural_disasters",
                "description": (f"Depth {q['depth']:.0f} km." if q["depth"] is not None else "") + (" USGS tsunami flag set (check official tsunami centres)." if q["tsunami"] else "")
                               + (f" USGS PAGER alert: {q['alert']}." if q["alert"] else ""),
                "country": q["country"], "related_countries": [q["country"]] if q["country"] else [],
                "lat": q["lat"], "lon": q["lon"], "started_at": q["time"], "first_detected": q["retrieved_at"],
                "last_updated": q["updated"] or q["retrieved_at"], "origin": "usgs", "cluster_id": None,
                "source_refs": [{"title": f"USGS event page {q['id']}", "url": q["url"], "publisher": "USGS", "published_at": q["time"]}],
                "datasets": ["usgs-quakes"], "verification": "instrument_observation", "extraction_confidence": 0.99,
                "magnitude": q["mag"],
            })
            n += 1

        # GDACS current alerts
        for o in conn.execute("SELECT * FROM signal_observations WHERE series LIKE 'gdacs:%' AND ts >= ?",
                              (to_iso(_utcnow() - timedelta(days=14)),)):
            rec = uj(o["source"], {})
            if not rec or not rec.get("is_current"):
                continue
            cs = rec.get("countries") or []
            _upsert_event(conn, {
                "id": "e_g_" + _hid(o["series"]), "title": rec["title"], "category": "natural_disasters",
                "description": f"GDACS alert level: {rec.get('alert_level') or 'n/a'}. {rec.get('summary') or ''}"[:400],
                "country": cs[0] if cs else None, "related_countries": cs, "lat": rec.get("lat"), "lon": rec.get("lon"),
                "started_at": rec.get("published"), "first_detected": o["retrieved_at"], "last_updated": o["retrieved_at"],
                "origin": "gdacs", "cluster_id": None,
                "source_refs": [{"title": rec["title"], "url": rec["link"], "publisher": "GDACS", "published_at": rec.get("published")}],
                "datasets": ["gdacs"], "verification": "official_alert", "extraction_confidence": 0.9, "magnitude": None,
            })
            n += 1
    return n


def _upsert_event(conn: sqlite3.Connection, e: dict) -> None:
    old = conn.execute("SELECT * FROM events WHERE id=?", (e["id"],)).fetchone()
    vals = {**e, "related_countries": j(e["related_countries"]), "source_refs": j(e["source_refs"]), "datasets": j(e["datasets"])}
    if old:
        changed = any(str(old[k]) != str(vals[k]) for k in ("title", "verification", "magnitude", "country", "description"))
        rev = old["revision"] + 1 if changed else old["revision"]
        if changed:
            conn.execute("INSERT OR IGNORE INTO event_revisions(event_id, revision, changed_at, snapshot) VALUES (?,?,?,?)",
                         (e["id"], old["revision"], now_iso(), j(dict(old))))
        vals["revision"] = rev
        vals["first_detected"] = old["first_detected"]
    else:
        vals["revision"] = 1
    cols = list(vals)
    conn.execute(f"INSERT OR REPLACE INTO events({','.join(cols)}) VALUES ({','.join('?' * len(cols))})", [vals[c] for c in cols])


# ============================================================== relationships

ORGS = ["NATO", "OPEC", "United Nations", "UN Security Council", "European Union", "EU", "WHO", "IMF", "World Bank", "WTO",
        "G7", "G20", "BRICS", "ASEAN", "African Union", "IAEA", "Hamas", "Hezbollah", "Houthi", "Fed", "ECB", "Kremlin"]
_ORG_RX = {o: re.compile(r"(?<!\w)" + re.escape(o) + r"(?!\w)") for o in ORGS}
CHOKEPOINTS = ["Red Sea", "Strait of Hormuz", "Hormuz", "Suez", "Panama Canal", "Black Sea", "Bab el-Mandeb", "Taiwan Strait", "Malacca"]
# category pairs where a shared chokepoint makes a contextual link plausible (hypothesis only)
CONTEXT_PAIRS = {frozenset(p) for p in [("trade", "energy"), ("military_security", "trade"), ("military_security", "energy"),
                                        ("trade", "financial_markets"), ("energy", "financial_markets"), ("natural_disasters", "energy"),
                                        ("natural_disasters", "trade")]}


def build_relationships(conn: sqlite3.Connection, days: int = 7) -> int:
    evs = [dict(r) for r in conn.execute("SELECT * FROM events WHERE last_updated >= ?", (to_iso(_utcnow() - timedelta(days=days)),))]
    for e in evs:
        e["_tok"] = tokens(e["title"].lower())
        e["_t"] = parse_iso(e["started_at"]) or parse_iso(e["first_detected"])
        e["_orgs"] = {o for o, rx in _ORG_RX.items() if rx.search(e["title"])}
        e["_choke"] = {c for c in CHOKEPOINTS if c.lower() in e["title"].lower()}
        e["_cs"] = set(uj(e["related_countries"], []))
    rels = []
    by_country: dict[str, list[dict]] = defaultdict(list)
    for e in evs:
        for c in e["_cs"]:
            by_country[c].append(e)
    seen = set()

    def add(a, b, typ, evidence, method, conf, observed):
        key = (a["id"], b["id"], typ)
        if key in seen or a["id"] == b["id"]:
            return
        seen.add(key)
        rels.append((a["id"], b["id"], typ, evidence, method, round(conf, 2), int(observed)))

    quakes = [e for e in evs if e["origin"] == "usgs"]
    news = [e for e in evs if e["origin"] == "news_cluster"]
    for q in quakes:
        for e in news:
            if e["category"] != "natural_disasters" or not re.search(r"earthquake|quake|tremor|tsunami", e["title"], re.I):
                continue
            if not (e["_t"] and q["_t"] and timedelta(0) <= e["_t"] - q["_t"] <= timedelta(hours=72)):
                continue
            place_words = {w for w in re.findall(r"[A-Z][a-z]{3,}", q["title"])} - {"Earthquake"}
            share_place = any(w in e["title"] for w in place_words)
            if (q["country"] and q["country"] in e["_cs"]) or share_place:
                add(q, e, "same_event", f"News report about an earthquake in {'the same place' if share_place else COUNTRIES.get(q['country']).name if q['country'] in COUNTRIES else 'the same country'} within 72 h after the USGS event",
                    "time window + location match", 0.75 if share_place else 0.55, True)

    for c, group in by_country.items():
        group.sort(key=lambda e: e["_t"] or _utcnow())
        for i, a in enumerate(group):
            for b in group[i + 1:i + 40]:
                if not (a["_t"] and b["_t"]) or b["_t"] - a["_t"] > timedelta(hours=96):
                    continue
                sim = jaccard(a["_tok"], b["_tok"])
                cname = COUNTRIES[c].name if c in COUNTRIES else c
                if sim >= 0.3 and b["_t"] - a["_t"] >= timedelta(hours=12) and a["category"] == b["category"]:
                    add(a, b, "follow_up", f"Similar headline wording ({sim:.0%} shared terms) about {cname}, {int((b['_t'] - a['_t']).total_seconds() // 3600)} h later",
                        "headline similarity + time order", 0.4 + sim / 2, False)
                elif a["category"] == b["category"] and a["category"] != "general":
                    add(a, b, "shared_country", f"Both concern {cname} and are classed as {LABELS.get(a['category'], a['category'])}",
                        "shared country + category", 0.5, True)
                shared_orgs = a["_orgs"] & b["_orgs"]
                if shared_orgs:
                    add(a, b, "shared_organization", f"Both mention {', '.join(sorted(shared_orgs))}", "named organization match", 0.5, True)
    for i, a in enumerate(evs):
        if not a["_choke"]:
            continue
        for b in evs[i + 1:]:
            shared = a["_choke"] & b["_choke"]
            if shared and frozenset((a["category"], b["category"])) in CONTEXT_PAIRS:
                add(a, b, "possible_context", f"Both mention {', '.join(sorted(shared))}. A contextual link is possible; it is not established that one affects the other.",
                    "shared chokepoint + related categories", 0.3, False)
    with tx(conn):
        conn.execute("DELETE FROM event_relationships")
        conn.executemany("INSERT OR REPLACE INTO event_relationships VALUES (?,?,?,?,?,?,?)", rels)
    return len(rels)


# ==================================================================== signals

THRESHOLDS = {"index": 3.0, "commodity": 4.0, "fx": 2.0, "crypto": 8.0}


def _series(conn, series: str) -> list[tuple[str, float]]:
    return [(r["ts"], r["value"]) for r in conn.execute("SELECT ts, value FROM signal_observations WHERE series=? ORDER BY ts", (series,))]


def _save_signal(conn, s: dict) -> None:
    s = {"region": None, "baseline": None, "baseline_std": None, "deviation": None, "pct_change": None, "score": None,
         "unit": None, "data_freshness": None, "country": None, **s}
    s["evidence"] = j(s["evidence"])
    cols = list(s)
    # keep the first detection time when the same signal is seen again
    updates = ",".join(f"{c}=excluded.{c}" for c in cols if c not in ("id", "detected_at"))
    conn.execute(f"INSERT INTO signals({','.join(cols)}) VALUES ({','.join('?' * len(cols))}) ON CONFLICT(id) DO UPDATE SET {updates}",
                 [s[c] for c in cols])


def detect_signals(conn: sqlite3.Connection, registry: dict) -> int:
    now = _utcnow()
    detected = now_iso()
    n = 0
    with tx(conn):
        n += _volume_signals(conn, detected)
        n += _local_volume_signals(conn, detected, now)
        n += _quake_signals(conn, detected, now)
        n += _market_signals(conn, registry, detected, now)
        n += _gdacs_signals(conn, detected, now)
        n += _ioda_signals(conn, detected, now)
    return n


def _zstats(values: list[float]) -> tuple[float, float]:
    mean = statistics.fmean(values)
    std = statistics.pstdev(values) if len(values) > 1 else 0.0
    return mean, std


def _volume_signals(conn, detected) -> int:
    n = 0
    yesterday = (_utcnow() - timedelta(days=1)).date().isoformat()
    for r in conn.execute("SELECT DISTINCT series FROM signal_observations WHERE series LIKE 'vol:%'").fetchall():
        series = r["series"]
        pts = [p for p in _series(conn, series) if p[0] <= yesterday]
        if not pts or pts[-1][0] != yesterday:
            continue
        base = [v for _, v in pts[-29:-1]]
        obs = pts[-1][1]
        if len(base) < 14:
            continue
        mean, std = _zstats(base)
        if mean <= 0:
            continue
        z = (obs - mean) / std if std > 0 else 0
        pct = (obs - mean) / mean * 100
        if not (z >= 2.5 and pct >= 50 and obs >= 30):
            continue
        _, kind, key = series.split(":", 2)
        is_country = kind == "country"
        name = COUNTRIES[key].name if is_country and key in COUNTRIES else key.replace("_", " ")
        src = conn.execute("SELECT source FROM signal_observations WHERE series=? ORDER BY ts DESC LIMIT 1", (series,)).fetchone()["source"]
        evidence = [{"type": "dataset", "title": f"GDELT daily article count for {name}", "detail": src,
                     "url": "https://api.gdeltproject.org/api/v2/doc/doc?query=" + src.split("query=", 1)[-1].replace(" ", "%20") + "&mode=timelinevolraw&timespan=30d"}]
        evidence += _top_articles(conn, key if is_country else None, None if is_country else key, yesterday)
        _save_signal(conn, {
            "id": "s_" + _hid(series, yesterday), "signal_type": "news_volume" if is_country else "topic_volume",
            "category": "news_coverage" if is_country else "geopolitical_reporting",
            "country": key if is_country else None,
            "title": f"Unusual rise in news coverage of {name}" if is_country else f"Unusual rise in reporting on {name}",
            "detected_at": detected, "observed": obs, "baseline": round(mean, 1), "baseline_std": round(std, 1),
            "deviation": round(z, 2), "pct_change": round(pct, 1), "score": round(z, 2), "unit": "articles per day (GDELT-monitored)",
            "evidence": evidence, "data_freshness": yesterday,
            "confidence": "high" if len(base) >= 21 and std > 0 else "medium",
            "limitations": "Measures how much monitored media wrote, not how much happened. Counts articles that mention the name, "
                           "so unrelated uses of the word and syndicated copies inflate it. GDELT coverage can change when its sources change.",
            "status": "statistical_anomaly"})
        n += 1
    return n


def _top_articles(conn, iso2: str | None, topic: str | None, day: str, k: int = 5) -> list[dict]:
    rows = conn.execute("""SELECT title,url,publisher,published_at,countries,category FROM articles
                           WHERE duplicate_of IS NULL AND substr(coalesce(published_at,collected_at),1,10)=? ORDER BY published_at DESC LIMIT 2000""",
                        (day,)).fetchall()
    out = []
    for r in rows:
        if iso2 and iso2 not in uj(r["countries"], {}):
            continue
        if topic and topic.split("_")[0][:6] not in r["title"].lower():
            continue
        out.append({"type": "article", "title": r["title"], "url": r["url"], "publisher": r["publisher"], "published_at": r["published_at"]})
        if len(out) >= k:
            break
    return out


def _local_volume_signals(conn, detected, now) -> int:
    """Story-cluster counts per country from WorldPulse's own collection.
    Requires 7+ days with successful collection; otherwise silent."""
    days_ok = [r[0] for r in conn.execute(
        "SELECT substr(started_at,1,10) d FROM fetch_logs WHERE status='ok' GROUP BY d HAVING count(*) >= 5 ORDER BY d")]
    yesterday = (now - timedelta(days=1)).date().isoformat()
    if yesterday not in days_ok:
        return 0
    base_days = [d for d in days_ok if d < yesterday][-28:]
    if len(base_days) < 7:
        return 0
    counts: dict[str, Counter] = defaultdict(Counter)
    for r in conn.execute("""SELECT substr(coalesce(published_at,collected_at),1,10) d, countries, cluster_id FROM articles
                             WHERE duplicate_of IS NULL AND collected_at >= ?""", (base_days[0],)):
        for iso2, conf in uj(r["countries"], {}).items():
            if conf >= 0.9:
                counts[iso2][(r["d"], r["cluster_id"])] = 1
    n = 0
    for iso2, c in counts.items():
        per_day = Counter(d for d, _ in c)
        obs = per_day.get(yesterday, 0)
        base = [per_day.get(d, 0) for d in base_days]
        mean, std = _zstats(base)
        if obs < 8 or mean <= 0:
            continue
        z = (obs - mean) / std if std > 0 else 0
        pct = (obs - mean) / mean * 100
        if z >= 3 and pct >= 100:
            name = COUNTRIES[iso2].name
            _save_signal(conn, {
                "id": "s_" + _hid("local", iso2, yesterday), "signal_type": "news_volume", "category": "news_coverage", "country": iso2,
                "title": f"More distinct stories about {name} than usual (WorldPulse feeds)", "detected_at": detected,
                "observed": obs, "baseline": round(mean, 1), "baseline_std": round(std, 1), "deviation": round(z, 2), "pct_change": round(pct, 1),
                "score": round(z, 2), "unit": "distinct story clusters per day", "data_freshness": yesterday,
                "evidence": _top_articles(conn, iso2, None, yesterday, 8), "confidence": "medium" if len(base_days) >= 14 else "low",
                "limitations": "Based only on WorldPulse's configured feeds; adding or losing a feed changes the count. Duplicates are merged into story clusters first.",
                "status": "statistical_anomaly"})
            n += 1
    return n


def _quake_signals(conn, detected, now) -> int:
    n = 0
    for q in conn.execute("SELECT * FROM quakes WHERE time >= ? AND (mag >= 6.0 OR (mag >= 5.0 AND (tsunami=1 OR alert IN ('yellow','orange','red'))))",
                          (to_iso(now - timedelta(hours=72)),)):
        extra = []
        if q["alert"]:
            extra.append(f"USGS PAGER alert level '{q['alert']}' (estimated impact)")
        if q["tsunami"]:
            extra.append("USGS tsunami flag set — this flag means the region is one where tsunami information is issued, not that a tsunami occurred")
        _save_signal(conn, {
            "id": "s_q_" + q["id"], "signal_type": "earthquake", "category": "natural_disasters", "country": q["country"],
            "region": q["place"], "title": f"M{q['mag']:.1f} earthquake — {q['place']}", "detected_at": detected,
            "observed": q["mag"], "unit": "magnitude", "data_freshness": q["updated"] or q["time"],
            "evidence": [{"type": "dataset", "title": f"USGS event {q['id']}", "url": q["url"], "published_at": q["time"],
                          "detail": (f"Depth {q['depth']:.0f} km" if q["depth"] is not None else "Depth n/a") + ("; " + "; ".join(extra) if extra else "")}],
            "confidence": "high", "status": "observed_measurement",
            "limitations": "Magnitude and location can be revised by USGS in the hours after an event. Damage is not implied by magnitude alone."})
        n += 1
    # clusters: 2°x2° cells, last 24h vs the previous 6 days
    since = now - timedelta(days=7)
    cells: dict[tuple[int, int], list] = defaultdict(list)
    for q in conn.execute("SELECT * FROM quakes WHERE time >= ? AND mag >= 2.5", (to_iso(since),)):
        if q["lat"] is None:
            continue
        cells[(math.floor(q["lat"] / 2), math.floor(q["lon"] / 2))].append(q)
    day_ago = now - timedelta(hours=24)
    for cell, qs in cells.items():
        recent = [q for q in qs if parse_iso(q["time"]) >= day_ago]
        older = len(qs) - len(recent)
        mean = older / 6
        if len(recent) >= 6 and len(recent) >= 3 * (mean + 1):
            top = max(recent, key=lambda q: q["mag"] or 0)
            _save_signal(conn, {
                "id": "s_qc_" + _hid(str(cell), now.date().isoformat()), "signal_type": "quake_cluster", "category": "natural_disasters",
                "country": top["country"], "region": top["place"], "title": f"Cluster of {len(recent)} earthquakes near {top['place']}",
                "detected_at": detected, "observed": len(recent), "baseline": round(mean, 1), "deviation": round(len(recent) / (mean + 1), 1),
                "pct_change": round((len(recent) - mean) / mean * 100, 1) if mean else None, "score": round(len(recent) / (mean + 1), 1),
                "unit": "M2.5+ events in 24 h within a 2°×2° cell", "data_freshness": max(q["time"] for q in recent),
                "evidence": [{"type": "dataset", "title": f"USGS M{q['mag']:.1f} — {q['place']}", "url": q["url"], "published_at": q["time"]}
                             for q in sorted(recent, key=lambda q: -(q["mag"] or 0))[:6]],
                "confidence": "medium" if mean >= 1 else "low", "status": "statistical_anomaly",
                "limitations": "Six-day baseline only. USGS catalogue completeness for small quakes varies by region (much better inside the US), "
                               "so some regions look quieter than they are. Aftershock sequences after a large event are expected, not unusual."})
            n += 1
    return n


def _returns(points: list[tuple[str, float]]) -> list[float]:
    return [(points[i][1] / points[i - 1][1] - 1) * 100 for i in range(1, len(points)) if points[i - 1][1]]


def _market_signals(conn, registry, detected, now) -> int:
    n = 0
    meta = {f"mkt:{m['symbol']}": (m["name"], m["kind"], m.get("country"), m["unit"]) for m in registry.get("markets", [])}
    meta.update({f"fx:USD{c}": (f"US dollar to {c}", "fx", None, f"{c} per USD") for c in registry.get("fx_quotes", [])})
    meta.update({f"crypto:{c}": (c.title(), "crypto", None, "USD") for c in registry.get("crypto", [])})
    for series, (name, kind, country, unit) in meta.items():
        pts = _series(conn, series)
        if len(pts) < 40:
            continue
        last_day = parse_iso(pts[-1][0] + "T00:00:00Z")
        if last_day and now - last_day > timedelta(days=5):
            continue  # stale: do not raise signals on old data
        rets = _returns(pts)
        last = rets[-1]
        mean, std = _zstats(rets[-61:-1])
        z = (last - mean) / std if std > 0 else 0
        five = (pts[-1][1] / pts[-6][1] - 1) * 100 if len(pts) >= 6 and pts[-6][1] else 0
        five_r = [(pts[i][1] / pts[i - 5][1] - 1) * 100 for i in range(len(pts) - 65, len(pts) - 1) if i >= 5 and pts[i - 5][1]]
        _, std5 = _zstats(five_r) if len(five_r) > 10 else (0, 0)
        z5 = five / std5 if std5 > 0 else 0
        thr = THRESHOLDS[kind]
        hit_day = abs(z) >= 3 or abs(last) >= thr
        hit_week = abs(z5) >= 3 and abs(five) >= thr
        if not (hit_day or hit_week):
            continue
        src = conn.execute("SELECT source FROM signal_observations WHERE series=? ORDER BY ts DESC LIMIT 1", (series,)).fetchone()["source"]
        window = "1-day" if hit_day else "5-day"
        chg = last if hit_day else five
        _save_signal(conn, {
            "id": "s_m_" + _hid(series, pts[-1][0], window), "signal_type": "fx" if kind == "fx" else "market",
            "category": "financial_markets", "country": country,
            "title": f"{name}: {'+' if chg > 0 else ''}{chg:.1f}% {window} move", "detected_at": detected,
            "observed": round(pts[-1][1], 4), "baseline": round(pts[-2][1] if hit_day else pts[-6][1], 4),
            "baseline_std": round(std if hit_day else std5, 3), "deviation": round(z if hit_day else z5, 2), "pct_change": round(chg, 2),
            "score": round(abs(z if hit_day else z5), 2), "unit": unit, "data_freshness": pts[-1][0],
            "evidence": [{"type": "dataset", "title": f"{name} daily closes", "detail": src, "published_at": pts[-1][0]}],
            "confidence": "high" if len(rets) >= 60 else "medium", "status": "statistical_anomaly",
            "limitations": ("Delayed end-of-day data from a free, unofficial endpoint; not an exchange feed. Futures roll dates can create artificial jumps."
                            if kind in ("index", "commodity") else
                            "ECB reference rates are published once per business day; intraday moves are not visible." if kind == "fx" else
                            "Aggregated spot price from CoinGecko; crypto trades 24/7 and prices differ between venues."),
        })
        n += 1
    return n


def _gdacs_signals(conn, detected, now) -> int:
    n = 0
    for o in conn.execute("SELECT * FROM signal_observations WHERE series LIKE 'gdacs:%' AND value >= 2 AND ts >= ?",
                          (to_iso(now - timedelta(days=7)),)):
        rec = uj(o["source"], {})
        if not rec.get("is_current"):
            continue
        cs = rec.get("countries") or []
        _save_signal(conn, {
            "id": "s_g_" + _hid(o["series"]), "signal_type": "disaster_alert", "category": "natural_disasters",
            "country": cs[0] if cs else None, "title": rec["title"], "detected_at": detected, "observed": o["value"],
            "unit": "GDACS alert level (1 green, 2 orange, 3 red)", "data_freshness": o["ts"],
            "evidence": [{"type": "dataset", "title": rec["title"], "url": rec["link"], "published_at": rec.get("published"),
                          "detail": f"Alert level {rec.get('alert_level')}; severity: {rec.get('severity') or 'n/a'}"}],
            "confidence": "high", "status": "official_alert",
            "limitations": "GDACS alert levels are model-based estimates of humanitarian impact issued soon after an event; they are revised as information arrives."})
        n += 1
    return n


def _ioda_signals(conn, detected, now) -> int:
    by_country: dict[str, list[dict]] = defaultdict(list)
    for o in conn.execute("SELECT * FROM signal_observations WHERE series LIKE 'ioda:%' AND ts >= ?", (to_iso(now - timedelta(hours=24)),)):
        rec = uj(o["source"], {})
        if rec.get("level") == "critical":
            by_country[rec["country"]].append(rec)
    n = 0
    for iso2, alerts in by_country.items():
        sources = sorted({a["datasource"] for a in alerts if a.get("datasource")})
        name = COUNTRIES[iso2].name if iso2 in COUNTRIES else iso2
        _save_signal(conn, {
            "id": "s_i_" + _hid(iso2, now.date().isoformat()), "signal_type": "internet", "category": "internet_disruption",
            "country": iso2, "title": f"Internet connectivity drop measured in {name}", "detected_at": detected,
            "observed": len(alerts), "unit": "IODA critical alerts in 24 h", "data_freshness": max(a["time"] for a in alerts),
            "evidence": [{"type": "dataset", "title": f"IODA {a['datasource']} alert", "published_at": a["time"],
                          "url": f"https://ioda.inetintel.cc.gatech.edu/country/{iso2}",
                          "detail": f"value {a.get('value')} vs history {a.get('history_value')}"} for a in alerts[:6]],
            "confidence": "high" if len(sources) >= 2 else "low",
            "status": "measurement_alert",
            "limitations": f"Measured by {', '.join(sources) or 'IODA'}. "
                           + ("Several independent measurements agree. " if len(sources) >= 2 else "Only one measurement method fired, which can be a measurement artefact. ")
                           + "A measured drop is not a confirmed outage or shutdown until an operator or authority reports it."})
        n += 1
    return n


def prune(conn: sqlite3.Connection, keep_days: int = 45) -> None:
    """Bounded retention so storage cannot grow without limit."""
    cut = to_iso(_utcnow() - timedelta(days=keep_days))
    with tx(conn):
        conn.execute("DELETE FROM articles WHERE collected_at < ?", (cut,))
        conn.execute("DELETE FROM events WHERE last_updated < ?", (cut,))
        conn.execute("DELETE FROM event_revisions WHERE changed_at < ?", (cut,))
        conn.execute("DELETE FROM signals WHERE detected_at < ?", (to_iso(_utcnow() - timedelta(days=120)),))
        conn.execute("DELETE FROM quakes WHERE time < ?", (to_iso(_utcnow() - timedelta(days=120)),))
        conn.execute("DELETE FROM fetch_logs WHERE started_at < ?", (cut,))
        conn.execute("DELETE FROM processing_errors WHERE at < ?", (cut,))
        conn.execute("DELETE FROM signal_observations WHERE ts < ? AND series NOT LIKE 'mkt:%' AND series NOT LIKE 'fx:%' AND series NOT LIKE 'crypto:%'",
                     (to_iso(_utcnow() - timedelta(days=400)),))
        conn.execute("DELETE FROM signal_observations WHERE ts < ?", (to_iso(_utcnow() - timedelta(days=800)),))
