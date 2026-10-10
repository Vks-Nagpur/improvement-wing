"""Two-stage briefings.

Stage 1 (extract): pick stored events, signals, indicators and observations
for the scope and period. Each becomes an evidence item with a reference.

Stage 2 (write): turn evidence into sentences with deterministic templates.
Every sentence carries the reference ids it rests on. Optionally a local
Ollama model writes a short summary from the same evidence; its output is
accepted only if every citation exists and every number it writes appears in
the evidence. Otherwise it is discarded and the template text stands.
"""
from __future__ import annotations

import json
import os
import re
import sqlite3
from collections import Counter
from datetime import datetime, timedelta, timezone

from .analyze import VERIFICATION
from .db import j, tx, uj
from .gazetteer import COUNTRIES
from .processing.classify import LABELS
from .processing.normalize import now_iso, to_iso

ECON = {"economics", "financial_markets", "trade", "energy"}
DIPLO = {"diplomacy", "geopolitics", "elections"}
SECURITY = {"military_security", "cybersecurity"}

VERIF_RANK = {"instrument_observation": 5, "official_alert": 4, "official_statement": 3, "multi_source_reporting": 2, "single_source_report": 1}


class Refs:
    def __init__(self) -> None:
        self.items: list[dict] = []
        self._ids: dict[str, int] = {}

    def add(self, title: str, url: str | None, publisher: str | None, date: str | None, kind: str = "article") -> int:
        key = url or title
        if key not in self._ids:
            self.items.append({"id": len(self.items) + 1, "title": title, "url": url, "publisher": publisher, "date": date, "kind": kind})
            self._ids[key] = len(self.items)
        return self._ids[key]

    def for_event(self, e: dict, k: int = 3) -> list[int]:
        return [self.add(r["title"], r.get("url"), r.get("publisher"), r.get("published_at"), "dataset" if e["origin"] != "news_cluster" else "article")
                for r in uj(e["source_refs"], [])[:k]]


def _since(hours: int) -> str:
    return to_iso(datetime.now(timezone.utc) - timedelta(hours=hours))


def _events(conn, hours: int, where: str = "", params: tuple = ()) -> list[dict]:
    rows = conn.execute(f"SELECT * FROM events WHERE last_updated >= ? {where}", (_since(hours), *params)).fetchall()
    evs = [dict(r) for r in rows]
    for e in evs:
        e["_pubs"] = len({r.get("publisher") for r in uj(e["source_refs"], [])})
        e["_rank"] = VERIF_RANK.get(e["verification"], 0) * 2 + e["_pubs"] + (e["magnitude"] or 0)
    evs.sort(key=lambda e: e["_rank"], reverse=True)
    return evs


def _event_line(e: dict) -> str:
    v = {"instrument_observation": "observed", "official_alert": "official alert", "official_statement": "official source",
         "multi_source_reporting": f"reported by {e['_pubs']} outlets", "single_source_report": "single-source report"}[e["verification"]]
    return f"{e['title']} ({v})"


def _limitations(conn, hours: int) -> list[str]:
    out = []
    failing = conn.execute("SELECT name, last_error FROM sources WHERE enabled=1 AND consecutive_failures > 0").fetchall()
    if failing:
        out.append(f"{len(failing)} source(s) failed on their latest attempt: " + ", ".join(r["name"] for r in failing[:6]) + ("…" if len(failing) > 6 else ""))
    out.append("News items are what monitored outlets published; WorldPulse does not verify claims. "
               "'Reported by N outlets' counts outlets, which may repeat the same original report.")
    out.append("Coverage is English-language and weighted toward outlets with free RSS feeds.")
    return out


def _section(heading: str, items: list[dict], empty: str) -> dict:
    return {"heading": heading, "items": items or [{"text": empty, "refs": []}]}


def country_briefing(conn: sqlite3.Connection, iso2: str, hours: int) -> dict:
    c = COUNTRIES[iso2]
    refs = Refs()
    evs = _events(conn, hours, "AND (country=? OR related_countries LIKE ?)", (iso2, f'%"{iso2}"%'))
    primary = [e for e in evs if e["country"] == iso2]
    related = [e for e in evs if e["country"] != iso2]
    sigs = [dict(r) for r in conn.execute("SELECT * FROM signals WHERE country=? AND detected_at >= ? ORDER BY score DESC", (iso2, _since(hours)))]
    period = {24: "the last 24 hours", 168: "the last 7 days", 720: "the last 30 days"}.get(hours, f"the last {hours} hours")
    cats = Counter(e["category"] for e in primary if e["category"] != "general")

    def items(group, k=5):
        return [{"text": _event_line(e), "refs": refs.for_event(e), "event_id": e["id"], "verification": e["verification"]} for e in group[:k]]

    exec_items = []
    if primary:
        top = ", ".join(f"{LABELS[k].lower()} ({v})" for k, v in cats.most_common(3)) or "uncategorised topics"
        exec_items.append({"text": f"In {period}, WorldPulse recorded {len(primary)} developments mainly about {c.name}; most frequent topics: {top}.",
                           "refs": []})
        exec_items.append({"text": f"Most prominent: {_event_line(primary[0])}.", "refs": refs.for_event(primary[0])})
    else:
        exec_items.append({"text": f"No developments mainly about {c.name} were recorded in {period} in monitored sources. "
                                   "This reflects source coverage, not necessarily an absence of events.", "refs": []})
    for s in sigs[:2]:
        exec_items.append({"text": f"Signal: {s['title']} — {_signal_measure(s)}.", "refs": [_sig_ref(refs, s)], "signal_id": s["id"]})

    ind = conn.execute("SELECT * FROM country_indicators WHERE iso2=? ORDER BY indicator, period DESC", (iso2,)).fetchall()
    latest = {}
    for r in ind:
        latest.setdefault(r["indicator"], r)
    econ_items = items([e for e in primary if e["category"] in ECON], 4)
    for key in ("NY.GDP.MKTP.KD.ZG", "FP.CPI.TOTL.ZG", "SL.UEM.TOTL.ZS"):
        r = latest.get(key)
        if r:
            econ_items.append({"text": f"{r['label']}: {r['value']:.1f} {r['unit']} in {r['period']} (annual figure, not a live measurement).",
                               "refs": [refs.add(f"World Bank WDI — {r['label']}", r["source_url"], "World Bank", r["retrieved_at"][:10], "dataset")]})

    sections = [
        _section("Executive summary", exec_items, ""),
        _section("Major recent developments", items(primary, 6), "None recorded in this period."),
        _section("Economic developments", econ_items, "No economic reporting recorded in this period."),
        _section("Diplomatic & political developments", items([e for e in primary if e["category"] in DIPLO]), "None recorded in this period."),
        _section("Security-related reporting", items([e for e in primary if e["category"] in SECURITY]), "None recorded in this period."),
        _section("Natural hazards", items([e for e in primary if e["category"] == "natural_disasters"]), "None recorded in this period."),
        _section("Relevant global developments", items(related, 5), f"No other-country developments mentioning {c.name} were recorded."),
    ]
    return _finish("country", iso2, hours, sections, refs, _limitations(conn, hours) +
                   (["Economic indicators are annual World Bank figures and can be one or more years old."] if latest else
                    ["No World Bank indicators were available for this country."]))


def _signal_measure(s: dict) -> str:
    if s["signal_type"] in ("news_volume", "topic_volume"):
        return f"{s['observed']:.0f} vs a typical {s['baseline']:.0f} {s['unit']} ({s['pct_change']:+.0f}%)"
    if s["signal_type"] in ("market", "fx"):
        return f"{s['pct_change']:+.2f}% (z = {s['deviation']:.1f}), data as of {s['data_freshness']}"
    if s["signal_type"] == "earthquake":
        return f"magnitude {s['observed']:.1f}"
    return f"{s['observed']} {s['unit'] or ''}".strip()


def _sig_ref(refs: Refs, s: dict) -> int:
    ev = uj(s["evidence"], [])
    first = ev[0] if ev else {"title": s["title"]}
    return refs.add(first.get("title", s["title"]), first.get("url"), first.get("publisher"), first.get("published_at") or s["data_freshness"], "dataset")


def global_briefing(conn: sqlite3.Connection, hours: int = 24) -> dict:
    refs = Refs()
    evs = _events(conn, hours)
    sections = []
    top = [e for e in evs if e["verification"] != "single_source_report"][:8]
    sections.append(_section("Top developments", [{"text": _event_line(e), "refs": refs.for_event(e), "event_id": e["id"],
                                                    "verification": e["verification"]} for e in top], "No multi-source developments recorded."))
    for heading, cats in (("Geopolitics, diplomacy & elections", DIPLO), ("Security", SECURITY), ("Economy, markets, trade & energy", ECON),
                          ("Natural hazards", {"natural_disasters"}), ("Health, environment & technology", {"public_health", "environment", "technology"})):
        group = [e for e in evs if e["category"] in cats][:5]
        sections.append(_section(heading, [{"text": _event_line(e), "refs": refs.for_event(e), "event_id": e["id"],
                                            "verification": e["verification"]} for e in group], "None recorded in this period."))
    sigs = [dict(r) for r in conn.execute("SELECT * FROM signals WHERE detected_at >= ? ORDER BY score DESC LIMIT 8", (_since(hours),))]
    sections.append(_section("Unusual changes (OpenSignals)", [{"text": f"{s['title']} — {_signal_measure(s)}.", "refs": [_sig_ref(refs, s)],
                                                               "signal_id": s["id"]} for s in sigs], "No signals crossed their thresholds."))
    return _finish("global", None, hours, sections, refs, _limitations(conn, hours))


def geopolitical_briefing(conn: sqlite3.Connection, hours: int = 24) -> dict:
    refs = Refs()
    evs = _events(conn, hours)
    sections = []
    for heading, cats in (("Diplomacy", {"diplomacy"}), ("Geopolitics & sanctions", {"geopolitics"}), ("Elections", {"elections"}),
                          ("Military & security", {"military_security"}), ("Cybersecurity", {"cybersecurity"})):
        group = [e for e in evs if e["category"] in cats][:6]
        sections.append(_section(heading, [{"text": _event_line(e), "refs": refs.for_event(e), "event_id": e["id"],
                                            "verification": e["verification"]} for e in group], "None recorded in this period."))
    sigs = [dict(r) for r in conn.execute("SELECT * FROM signals WHERE signal_type='topic_volume' AND detected_at >= ? ORDER BY score DESC", (_since(hours),))]
    sections.append(_section("Reporting trends", [{"text": f"{s['title']} — {_signal_measure(s)}.", "refs": [_sig_ref(refs, s)]} for s in sigs],
                             "No unusual change in monitored reporting topics."))
    return _finish("geopolitical", None, hours, sections, refs, _limitations(conn, hours) +
                   ["Topic volume measures reporting, not the actual frequency of incidents."])


def market_briefing(conn: sqlite3.Connection, registry: dict, hours: int = 24) -> dict:
    refs = Refs()
    rows = []
    names = {f"mkt:{m['symbol']}": m["name"] for m in registry.get("markets", [])}
    names.update({f"fx:USD{c}": f"USD/{c}" for c in registry.get("fx_quotes", [])})
    names.update({f"crypto:{c}": c.title() for c in registry.get("crypto", [])})
    for series, name in names.items():
        pts = conn.execute("SELECT ts, value, source FROM signal_observations WHERE series=? ORDER BY ts DESC LIMIT 6", (series,)).fetchall()
        if len(pts) < 2:
            continue
        chg = (pts[0]["value"] / pts[1]["value"] - 1) * 100
        rows.append((abs(chg), {"text": f"{name}: {pts[0]['value']:,.2f} on {pts[0]['ts']} ({chg:+.2f}% vs {pts[1]['ts']}).",
                                "refs": [refs.add(f"{name} daily series", None, pts[0]["source"], pts[0]["ts"], "dataset")]}))
    rows.sort(key=lambda r: -r[0])
    sigs = [dict(r) for r in conn.execute("SELECT * FROM signals WHERE signal_type IN ('market','fx') AND detected_at >= ? ORDER BY score DESC", (_since(hours),))]
    sections = [
        _section("Unusual moves", [{"text": f"{s['title']} — {_signal_measure(s)}.", "refs": [_sig_ref(refs, s)]} for s in sigs], "No market series moved beyond its thresholds."),
        _section("Largest latest daily moves", [r[1] for r in rows[:10]], "No market data available."),
        _section("Market-related reporting", [{"text": _event_line(e), "refs": refs.for_event(e)} for e in _events(conn, hours, "AND category IN ('financial_markets','energy','trade')")[:6]],
                 "None recorded in this period."),
    ]
    return _finish("market", None, hours, sections, refs, [
        "Prices are delayed end-of-day values from free endpoints, not exchange-grade real-time feeds.",
        "News listed next to market moves is not evidence that the news caused the move."])


def signals_briefing(conn: sqlite3.Connection, hours: int = 24) -> dict:
    refs = Refs()
    sigs = [dict(r) for r in conn.execute("SELECT * FROM signals WHERE detected_at >= ? ORDER BY score DESC", (_since(hours),))]
    groups = {"statistical_anomaly": "Statistical anomalies", "observed_measurement": "Instrument observations",
              "official_alert": "Official alerts", "measurement_alert": "Measurement alerts (unconfirmed)"}
    sections = []
    for status, heading in groups.items():
        g = [s for s in sigs if s["status"] == status]
        sections.append(_section(heading, [{"text": f"{s['title']} — {_signal_measure(s)}. Limits: {s['limitations']}", "refs": [_sig_ref(refs, s)],
                                            "signal_id": s["id"]} for s in g[:10]], "None in this period."))
    return _finish("signals", None, hours, sections, refs, ["A signal is an unusual change in data; it is not a prediction."])


def _finish(kind, scope, hours, sections, refs: Refs, limitations) -> dict:
    return {"kind": kind, "scope": scope, "period_hours": hours, "generated_at": now_iso(), "mode": "deterministic",
            "sections": sections, "references": refs.items, "limitations": limitations, "verification_legend": VERIFICATION}


# ---------------------------------------------------------------- optional AI

def ollama_summary(briefing: dict, model: str | None = None, host: str | None = None) -> str | None:
    """Ask a local Ollama model for a 3–5 sentence summary of the evidence.
    Returns None (and the template text is used) if Ollama is unavailable or
    the output fails validation."""
    import requests

    host = host or os.environ.get("OLLAMA_HOST", "http://127.0.0.1:11434")
    model = model or os.environ.get("OLLAMA_MODEL", "llama3.1:8b")
    evidence = []
    for sec in briefing["sections"]:
        for it in sec["items"]:
            if it["refs"]:
                evidence.append(f"- {it['text']} " + " ".join(f"[{r}]" for r in it["refs"]))
    if not evidence:
        return None
    prompt = ("You summarise news evidence. Use ONLY the evidence lines below. Write 3 to 5 plain sentences. "
              "End every sentence with the bracketed reference numbers it relies on, e.g. [2]. Do not add facts, numbers, "
              "dates, names or quotes that are not in the evidence. Do not speculate about causes. If evidence is thin, say so.\n\n"
              "EVIDENCE:\n" + "\n".join(evidence[:40]))
    try:
        r = requests.post(f"{host}/api/generate", json={"model": model, "prompt": prompt, "stream": False,
                                                        "options": {"temperature": 0.1}}, timeout=120)
        r.raise_for_status()
        text = r.json().get("response", "").strip()
    except Exception:
        return None
    return text if validate_ai_text(text, "\n".join(evidence), len(briefing["references"])) else None


def validate_ai_text(text: str, evidence: str, n_refs: int) -> bool:
    if not text:
        return False
    if any(int(c) < 1 or int(c) > n_refs for c in re.findall(r"\[(\d+)\]", text)):
        return False  # cites a reference that does not exist
    sentences = [s for s in re.split(r"(?<=[.!?\]])\s+", text) if len(s.split()) >= 4]
    if any(not re.search(r"\[\d+\]", s) for s in sentences):
        return False  # a factual sentence without a citation
    ev_numbers = set(re.findall(r"\d+(?:\.\d+)?", evidence))
    body = re.sub(r"\[\d+\]", "", text)
    return all(n in ev_numbers for n in re.findall(r"\d+(?:\.\d+)?", body))


def generate_all(conn: sqlite3.Connection, registry: dict, use_ai: bool = False) -> list[dict]:
    out = [global_briefing(conn, 24), geopolitical_briefing(conn, 24), market_briefing(conn, registry, 24), signals_briefing(conn, 72),
           global_briefing(conn, 168)]
    if use_ai:
        for b in out:
            s = ollama_summary(b)
            if s:
                b["ai_summary"] = s
                b["mode"] = "deterministic+ollama"
    with tx(conn):
        for b in out:
            bid = f"{b['kind']}-{b['scope'] or 'world'}-{b['period_hours']}h-{b['generated_at'][:13]}"
            conn.execute("INSERT OR REPLACE INTO briefings(id,kind,scope,period_hours,generated_at,mode,body) VALUES (?,?,?,?,?,?,?)",
                         (bid, b["kind"], b["scope"], b["period_hours"], b["generated_at"], b["mode"], j(b)))
    return out
