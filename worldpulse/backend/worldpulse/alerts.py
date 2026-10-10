"""Alert rules evaluated against newly stored data.

Rule types and params:
  threshold   {"series": "mkt:CL=F", "op": ">"|"<", "value": 100}
  pct_change  {"series": "mkt:CL=F", "pct": 5}                 # abs daily move
  keyword     {"keywords": ["sanctions"], "countries": ["IR"]}  # countries optional
  geographic  {"countries": ["JP"], "categories": [...]}        # categories optional
  category    {"categories": ["cybersecurity"], "min_outlets": 2}
  anomaly     {"countries": ["IN"]} or {"signal_types": ["earthquake"], "min_magnitude": 6}
Each match is stored once (rule id + dedupe key), with its evidence.
"""
from __future__ import annotations

import sqlite3
from datetime import datetime, timedelta, timezone

from .db import j, tx, uj
from .processing.normalize import now_iso, to_iso

RULE_TYPES = {"threshold", "pct_change", "keyword", "geographic", "category", "anomaly"}


def validate_rule(rule_type: str, params: dict) -> None:
    if rule_type not in RULE_TYPES:
        raise ValueError(f"rule_type must be one of {sorted(RULE_TYPES)}")
    need = {"threshold": ["series", "op", "value"], "pct_change": ["series", "pct"], "keyword": ["keywords"],
            "geographic": ["countries"], "category": ["categories"], "anomaly": []}[rule_type]
    missing = [k for k in need if k not in params]
    if missing:
        raise ValueError(f"missing params: {missing}")


def evaluate(conn: sqlite3.Connection, lookback_hours: int = 24) -> list[dict]:
    since = to_iso(datetime.now(timezone.utc) - timedelta(hours=lookback_hours))
    fired = []
    rules = conn.execute("SELECT * FROM alert_rules WHERE enabled=1").fetchall()
    with tx(conn):
        for rule in rules:
            p = uj(rule["params"], {})
            for key, msg, ev in _matches(conn, rule["rule_type"], p, since):
                cur = conn.execute("INSERT OR IGNORE INTO alert_history(rule_id, dedupe_key, triggered_at, message, evidence) VALUES (?,?,?,?,?)",
                                   (rule["id"], key, now_iso(), f"[{rule['name']}] {msg}", j(ev)))
                if cur.rowcount:
                    fired.append({"rule": rule["name"], "message": msg, "evidence": ev})
    return fired


def _events(conn, since, p):
    q, args = "SELECT * FROM events WHERE last_updated >= ?", [since]
    if p.get("countries"):
        q += " AND (" + " OR ".join(["country=?"] * len(p["countries"]) + ["related_countries LIKE ?"] * len(p["countries"])) + ")"
        args += p["countries"] + [f'%"{c}"%' for c in p["countries"]]
    if p.get("categories"):
        q += f" AND category IN ({','.join('?' * len(p['categories']))})"
        args += p["categories"]
    return conn.execute(q, args).fetchall()


def _matches(conn, typ, p, since):
    if typ in ("threshold", "pct_change"):
        pts = conn.execute("SELECT ts, value, source FROM signal_observations WHERE series=? ORDER BY ts DESC LIMIT 2", (p["series"],)).fetchall()
        if not pts:
            return
        last = pts[0]
        if typ == "threshold":
            ok = last["value"] > p["value"] if p["op"] == ">" else last["value"] < p["value"]
            if ok:
                yield (f"{p['series']}:{last['ts']}", f"{p['series']} is {last['value']:.2f} ({p['op']} {p['value']}) on {last['ts']}",
                       [{"series": p["series"], "date": last["ts"], "value": last["value"], "source": last["source"]}])
        elif len(pts) == 2 and pts[1]["value"]:
            chg = (last["value"] / pts[1]["value"] - 1) * 100
            if abs(chg) >= p["pct"]:
                yield (f"{p['series']}:{last['ts']}", f"{p['series']} moved {chg:+.2f}% to {last['value']:.2f} on {last['ts']}",
                       [{"series": p["series"], "date": last["ts"], "value": last["value"], "previous": pts[1]["value"], "source": last["source"]}])
    elif typ in ("keyword", "geographic", "category"):
        for e in _events(conn, since, p):
            refs = uj(e["source_refs"], [])
            if typ == "keyword" and not any(k.lower() in e["title"].lower() for k in p["keywords"]):
                continue
            if typ == "category" and len({r.get("publisher") for r in refs}) < p.get("min_outlets", 1):
                continue
            yield (e["id"], e["title"], refs[:3])
    elif typ == "anomaly":
        q, args = "SELECT * FROM signals WHERE detected_at >= ?", [since]
        if p.get("countries"):
            q += f" AND country IN ({','.join('?' * len(p['countries']))})"
            args += p["countries"]
        if p.get("signal_types"):
            q += f" AND signal_type IN ({','.join('?' * len(p['signal_types']))})"
            args += p["signal_types"]
        for s in conn.execute(q, args):
            if p.get("min_magnitude") and s["signal_type"] == "earthquake" and (s["observed"] or 0) < p["min_magnitude"]:
                continue
            yield (s["id"], s["title"], uj(s["evidence"], [])[:3])
