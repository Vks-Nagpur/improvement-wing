"""Public data export: compact, bounded, read-only JSON for the static site.

Retention is bounded (news 72 h, events 7 days, quakes 30 days, signals 30
days, market series 6 months) so the published dataset cannot grow without
limit. Every file carries generated_at so the site can flag stale data.
"""
from __future__ import annotations

import json
import shutil
import sqlite3
from collections import Counter, defaultdict
from datetime import datetime, timedelta, timezone
from pathlib import Path

from . import PROCESSING_VERSION, __version__
from .briefings import country_briefing
from .db import uj
from .gazetteer import COUNTRIES
from .processing.classify import LABELS
from .processing.normalize import now_iso, to_iso


def _since(**kw) -> str:
    return to_iso(datetime.now(timezone.utc) - timedelta(**kw))


def _write(path: Path, data) -> int:
    path.parent.mkdir(parents=True, exist_ok=True)
    raw = json.dumps(data, ensure_ascii=False, separators=(",", ":"))
    path.write_text(raw)
    return len(raw)


def export_all(conn: sqlite3.Connection, registry: dict, out: Path, briefings: list[dict]) -> dict:
    gen = now_iso()
    if (out / "countries").exists():
        shutil.rmtree(out / "countries")
    sizes = {}

    # ---- sources & health
    srcs = []
    for s in conn.execute("SELECT * FROM sources ORDER BY kind, name"):
        last = conn.execute("SELECT * FROM fetch_logs WHERE source_id=? ORDER BY id DESC LIMIT 1", (s["id"],)).fetchone()
        srcs.append({"id": s["id"], "name": s["name"], "kind": s["kind"], "type": s["source_type"], "category": s["category"],
                     "country": s["country"], "language": s["language"], "url": s["url"], "enabled": bool(s["enabled"]),
                     "interval_minutes": s["interval_minutes"], "last_success": s["last_success"], "last_attempt": s["last_attempt"],
                     "last_error": s["last_error"], "failures": s["consecutive_failures"],
                     "last_items": last["items"] if last else None, "last_new": last["new_items"] if last else None,
                     "status": "disabled" if not s["enabled"] else "never_run" if not s["last_attempt"] else
                               "ok" if not s["consecutive_failures"] else "failing"})
    tl = conn.execute("SELECT * FROM fetch_logs WHERE source_id='gdelt-timelines' ORDER BY id DESC LIMIT 1").fetchone()
    if tl:
        srcs.append({"id": "gdelt-timelines", "name": "GDELT — daily coverage volume (watch list)", "kind": "gdelt", "type": "aggregator",
                     "category": None, "country": None, "language": "en", "url": "https://api.gdeltproject.org/api/v2/doc/doc", "enabled": True,
                     "interval_minutes": 180, "last_success": tl["finished_at"] if tl["status"] == "ok" else None, "last_attempt": tl["started_at"],
                     "last_error": None if tl["status"] == "ok" else "no series fetched", "failures": 0 if tl["status"] == "ok" else 1,
                     "last_items": tl["items"], "last_new": tl["new_items"], "status": "ok" if tl["status"] == "ok" else "failing"})
    sizes["sources.json"] = _write(out / "sources.json", {"generated_at": gen, "sources": srcs})

    # ---- news (72 h)
    rows = conn.execute("""SELECT a.*, s.source_type, (SELECT count(*) FROM articles b WHERE b.cluster_id=a.cluster_id) AS cn
                           FROM articles a JOIN sources s ON s.id=a.source_id
                           WHERE a.duplicate_of IS NULL AND coalesce(a.published_at, a.collected_at) >= ?
                           ORDER BY coalesce(a.published_at, a.collected_at) DESC LIMIT 1500""", (_since(hours=72),)).fetchall()
    news = [{"id": r["id"], "t": r["title"], "u": r["url"], "p": r["publisher"], "x": r["excerpt"] or None,
             "d": r["published_at"] or r["collected_at"], "g": r["collected_at"], "c": r["category"],
             "k": [k for k, v in uj(r["countries"], {}).items() if v >= 0.9], "cl": r["cluster_id"], "n": r["cn"],
             "o": bool(r["is_official"]), "s": r["source_id"], "l": r["language"]} for r in rows]
    dup_total = conn.execute("SELECT count(*) FROM articles WHERE duplicate_of IS NOT NULL AND collected_at >= ?", (_since(hours=72),)).fetchone()[0]
    dup_methods = dict(conn.execute("SELECT duplicate_method, count(*) FROM articles WHERE duplicate_of IS NOT NULL AND collected_at >= ? GROUP BY 1",
                                    (_since(hours=72),)).fetchall())
    sizes["news.json"] = _write(out / "news.json", {"generated_at": gen, "window_hours": 72, "articles": news,
                                                    "duplicates_removed": dup_total, "duplicate_methods": dup_methods})

    # ---- events (7 days) with relationships
    rels = defaultdict(list)
    for r in conn.execute("SELECT * FROM event_relationships"):
        rels[r["source_event"]].append({"to": r["target_event"], "type": r["rel_type"], "evidence": r["evidence"], "method": r["method"],
                                        "confidence": r["confidence"], "observed": bool(r["observed"])})
        rels[r["target_event"]].append({"to": r["source_event"], "type": r["rel_type"], "evidence": r["evidence"], "method": r["method"],
                                        "confidence": r["confidence"], "observed": bool(r["observed"])})
    evrows = conn.execute("SELECT * FROM events WHERE last_updated >= ? ORDER BY last_updated DESC LIMIT 1200", (_since(days=7),)).fetchall()
    events = []
    for e in evrows:
        refs = uj(e["source_refs"], [])
        events.append({"id": e["id"], "title": e["title"], "category": e["category"], "description": e["description"], "country": e["country"],
                       "countries": uj(e["related_countries"], []), "lat": e["lat"], "lon": e["lon"], "started_at": e["started_at"],
                       "first_detected": e["first_detected"], "last_updated": e["last_updated"], "origin": e["origin"],
                       "verification": e["verification"], "confidence": e["extraction_confidence"], "magnitude": e["magnitude"],
                       "revision": e["revision"], "outlets": len({r.get("publisher") for r in refs}),
                       "refs": [{"t": r["title"], "u": r.get("url"), "p": r.get("publisher"), "d": r.get("published_at")} for r in refs[:6]],
                       "related": sorted(rels.get(e["id"], []), key=lambda x: -x["confidence"])[:8]})
    sizes["events.json"] = _write(out / "events.json", {"generated_at": gen, "window_days": 7, "events": events})

    # ---- signals (30 days)
    sigs = [dict(r) | {"evidence": uj(r["evidence"], [])} for r in
            conn.execute("SELECT * FROM signals WHERE detected_at >= ? ORDER BY detected_at DESC, score DESC LIMIT 600", (_since(days=30),))]
    sizes["signals.json"] = _write(out / "signals.json", {"generated_at": gen, "signals": sigs})

    # ---- quakes (30 days)
    qs = [[q["id"], q["time"], q["mag"], q["place"], q["lat"], q["lon"], q["depth"], q["country"], q["tsunami"], q["alert"], q["url"]]
          for q in conn.execute("SELECT * FROM quakes WHERE time >= ? ORDER BY time DESC", (_since(days=30),))]
    sizes["quakes.json"] = _write(out / "quakes.json", {"generated_at": gen, "source": "USGS Earthquake Hazards Program GeoJSON feeds",
                                                         "fields": ["id", "time", "mag", "place", "lat", "lon", "depth", "country", "tsunami", "alert", "url"],
                                                         "note": "M2.5+ for the last 7 days; M4.5+ for days 8–30.", "quakes": qs})

    # ---- markets
    series_meta = [(f"mkt:{m['symbol']}", m["name"], m["kind"], m["unit"], m.get("country")) for m in registry.get("markets", [])]
    series_meta += [(f"fx:USD{c}", f"USD/{c}", "fx", f"{c} per USD", None) for c in registry.get("fx_quotes", [])]
    series_meta += [(f"crypto:{c}", c.title(), "crypto", "USD", None) for c in registry.get("crypto", [])]
    markets = []
    for sid, name, kind, unit, country in series_meta:
        pts = conn.execute("SELECT ts, value, source FROM signal_observations WHERE series=? AND ts >= ? ORDER BY ts", (sid, _since(days=190)[:10])).fetchall()
        if not pts:
            markets.append({"id": sid, "name": name, "kind": kind, "unit": unit, "country": country, "points": [], "source": None})
            continue
        markets.append({"id": sid, "name": name, "kind": kind, "unit": unit, "country": country, "source": pts[-1]["source"],
                        "points": [[p["ts"], round(p["value"], 6)] for p in pts]})
    sizes["markets.json"] = _write(out / "markets.json", {"generated_at": gen, "series": markets,
                                                          "note": "Daily closes. Delayed data from free endpoints; not exchange-grade real-time."})

    # ---- countries
    ind = defaultdict(dict)
    for r in conn.execute("SELECT * FROM country_indicators ORDER BY period"):
        ind[r["iso2"]][r["indicator"]] = {"label": r["label"], "value": r["value"], "unit": r["unit"], "period": r["period"],
                                          "source": r["source"], "url": r["source_url"], "retrieved": r["retrieved_at"]}
    ev_by_c = defaultdict(list)
    for e in events:
        for c in set(e["countries"]) | ({e["country"]} if e["country"] else set()):
            ev_by_c[c].append(e)
    news_by_c = defaultdict(list)
    for a in news:
        for c in a["k"]:
            news_by_c[c].append(a)
    sig_by_c = defaultdict(list)
    for s in sigs:
        if s["country"]:
            sig_by_c[s["country"]].append(s)
    vol = defaultdict(list)
    for r in conn.execute("SELECT series, ts, value FROM signal_observations WHERE series LIKE 'vol:country:%' AND ts >= ? ORDER BY ts", (_since(days=31)[:10],)):
        vol[r["series"].split(":")[2]].append([r["ts"], r["value"]])
    clist = []
    day = _since(hours=24)
    for c in conn.execute("SELECT * FROM countries ORDER BY name"):
        iso2 = c["iso2"]
        evs = ev_by_c.get(iso2, [])
        cats = Counter(e["category"] for e in evs if e["category"] != "general")
        pop = ind[iso2].get("SP.POP.TOTL")
        gdp = ind[iso2].get("NY.GDP.MKTP.CD")
        entry = {"iso2": iso2, "iso3": c["iso3"], "name": c["name"], "region": c["region"], "wb_region": c["wb_region"],
                 "income": c["income_level"], "capital": c["capital"], "lat": c["lat"], "lon": c["lon"],
                 "population": pop and {"value": pop["value"], "period": pop["period"]},
                 "gdp": gdp and {"value": gdp["value"], "period": gdp["period"]},
                 "events_24h": sum(1 for e in evs if e["last_updated"] >= day), "events_7d": len(evs),
                 "news_72h": len(news_by_c.get(iso2, [])), "signals_30d": len(sig_by_c.get(iso2, [])),
                 "top_category": cats.most_common(1)[0][0] if cats else None}
        clist.append(entry)
        profile = {"generated_at": gen, "country": entry, "meta_source": c["meta_source"], "meta_updated": c["meta_updated"],
                   "indicators": ind.get(iso2, {}), "events": evs[:80], "news": news_by_c.get(iso2, [])[:60],
                   "signals": sig_by_c.get(iso2, [])[:30], "volume": vol.get(iso2, []),
                   "briefings": {h: country_briefing(conn, iso2, h) for h in (24, 168, 720)} if (evs or sig_by_c.get(iso2)) else {}}
        _write(out / "countries" / f"{iso2}.json", profile)
    sizes["countries.json"] = _write(out / "countries.json", {"generated_at": gen, "countries": clist})

    # ---- briefings
    sizes["briefings.json"] = _write(out / "briefings.json", {"generated_at": gen, "briefings": briefings})

    # ---- history (for charts & comparisons)
    hist = {"generated_at": gen}
    hist["articles_per_day"] = [list(r) for r in conn.execute(
        "SELECT substr(coalesce(published_at,collected_at),1,10) d, count(*), count(DISTINCT cluster_id) FROM articles WHERE duplicate_of IS NULL AND collected_at >= ? GROUP BY d ORDER BY d",
        (_since(days=45),))]
    hist["category_per_day"] = [list(r) for r in conn.execute(
        "SELECT substr(coalesce(published_at,collected_at),1,10) d, category, count(DISTINCT cluster_id) FROM articles WHERE duplicate_of IS NULL AND collected_at >= ? GROUP BY d, category ORDER BY d",
        (_since(days=45),))]
    hist["quakes_per_day"] = [list(r) for r in conn.execute(
        """SELECT substr(time,1,10) d, sum(mag>=2.5 AND mag<4.5), sum(mag>=4.5 AND mag<6), sum(mag>=6) FROM quakes WHERE time >= ? GROUP BY d ORDER BY d""",
        (_since(days=120),))]
    hist["signals_per_day"] = [list(r) for r in conn.execute(
        "SELECT substr(detected_at,1,10) d, signal_type, count(*) FROM signals GROUP BY d, signal_type ORDER BY d")]
    hist["topic_volume"] = defaultdict(list)
    for r in conn.execute("SELECT series, ts, value FROM signal_observations WHERE series LIKE 'vol:topic:%' AND ts >= ? ORDER BY ts", (_since(days=31)[:10],)):
        hist["topic_volume"][r["series"].split(":")[2]].append([r["ts"], r["value"]])
    hist["country_volume"] = vol
    hist["collection_days"] = [r[0] for r in conn.execute("SELECT DISTINCT substr(started_at,1,10) FROM fetch_logs WHERE status='ok' ORDER BY 1")]
    sizes["history.json"] = _write(out / "history.json", hist)

    # ---- meta
    ok = sum(1 for s in srcs if s["status"] == "ok")
    meta = {"generated_at": gen, "app_version": __version__, "processing_version": PROCESSING_VERSION, "mode": "static",
            "stale_after_hours": 12, "counts": {"articles_72h": len(news), "events_7d": len(events), "signals_30d": len(sigs),
                                                 "quakes_30d": len(qs), "countries": len(clist), "duplicates_removed_72h": dup_total},
            "sources": {"total": len(srcs), "ok": ok, "failing": sum(1 for s in srcs if s["status"] == "failing"),
                        "disabled": sum(1 for s in srcs if s["status"] == "disabled")},
            "category_labels": LABELS, "files": sizes}
    _write(out / "meta.json", meta)
    export_dashboard(out, news, events, sigs, qs, markets, clist, srcs, hist, gen)
    return meta


def export_dashboard(out: Path, news, events, sigs, qs, markets, clist, srcs, hist, gen) -> None:
    """Flat row tables (one JSON array of objects per file) for dashboard tools."""
    d = out / "dashboard"
    rows = lambda name, data: _write(d / f"{name}.json", data)  # noqa: E731
    rows("events", [{"id": e["id"], "title": e["title"], "category": LABELS.get(e["category"], e["category"]), "country": e["country"] or "",
                     "country_name": COUNTRIES[e["country"]].name if e["country"] in COUNTRIES else "", "verification": e["verification"],
                     "outlets": e["outlets"], "first_reported": e["started_at"] or e["first_detected"], "last_updated": e["last_updated"],
                     "origin": e["origin"], "magnitude": e["magnitude"], "url": e["refs"][0]["u"] if e["refs"] else "",
                     "source": e["refs"][0]["p"] if e["refs"] else ""} for e in events])
    rows("signals", [{"id": s["id"], "title": s["title"], "type": s["signal_type"], "status": s["status"], "country": s["country"] or "",
                      "country_name": COUNTRIES[s["country"]].name if s["country"] in COUNTRIES else "", "detected_at": s["detected_at"],
                      "observed": s["observed"], "baseline": s["baseline"], "pct_change": s["pct_change"], "deviation": s["deviation"],
                      "unit": s["unit"] or "", "confidence": s["confidence"], "data_as_of": s["data_freshness"] or "", "limitations": s["limitations"],
                      "evidence_url": next((x.get("url") for x in s["evidence"] if x.get("url")), "")} for s in sigs])
    rows("quakes", [{"id": q[0], "time": q[1], "magnitude": q[2], "place": q[3], "lat": q[4], "lon": q[5], "depth_km": q[6],
                     "country": q[7] or "", "tsunami_flag": q[8], "pager_alert": q[9] or "", "url": q[10]} for q in qs])
    latest = []
    series_rows = []
    for m in markets:
        p = m["points"]
        if len(p) < 2:
            continue
        def chg(k):
            return round((p[-1][1] / p[-1 - k][1] - 1) * 100, 2) if len(p) > k and p[-1 - k][1] else None
        latest.append({"id": m["id"], "name": m["name"], "kind": m["kind"], "unit": m["unit"], "last": p[-1][1], "as_of": p[-1][0],
                       "chg_1d": chg(1), "chg_5d": chg(5), "chg_21d": chg(21), "source": m["source"]})
        series_rows += [{"id": m["id"], "name": m["name"], "kind": m["kind"], "day": day, "value": v} for day, v in p[-130:]]
    rows("markets_latest", latest)
    rows("markets_series", series_rows)
    rows("countries", [{"iso2": c["iso2"], "name": c["name"], "region": c["region"], "capital": c["capital"] or "",
                        "income": c["income"] or "", "population": c["population"]["value"] if c["population"] else None,
                        "population_year": c["population"]["period"] if c["population"] else "",
                        "gdp_usd": c["gdp"]["value"] if c["gdp"] else None, "gdp_year": c["gdp"]["period"] if c["gdp"] else "",
                        "events_24h": c["events_24h"], "events_7d": c["events_7d"], "news_72h": c["news_72h"],
                        "signals_30d": c["signals_30d"], "top_topic": LABELS.get(c["top_category"], "") if c["top_category"] else ""} for c in clist])
    rows("sources", [{"id": s["id"], "name": s["name"], "kind": s["kind"], "type": s["type"] or "", "status": s["status"],
                      "last_success": s["last_success"] or "", "last_attempt": s["last_attempt"] or "", "last_items": s["last_items"],
                      "error": s["last_error"] or ""} for s in srcs])
    rows("category_daily", [{"day": r[0], "category": LABELS.get(r[1], r[1]), "stories": r[2]} for r in hist["category_per_day"]])
    rows("quakes_daily", [{"day": r[0], "m25_45": r[1], "m45_6": r[2], "m6_plus": r[3]} for r in hist["quakes_per_day"]])
    vol_rows = [{"key": k, "kind": "topic", "day": day, "articles": v} for k, pts in hist["topic_volume"].items() for day, v in pts]
    vol_rows += [{"key": k, "kind": "country", "day": day, "articles": v} for k, pts in hist["country_volume"].items() for day, v in pts]
    rows("coverage_volume", vol_rows)
    rows("news", [{"id": a["id"], "title": a["t"], "publisher": a["p"], "published": a["d"], "category": LABELS.get(a["c"], a["c"]),
                   "countries": " ".join(a["k"]), "cluster_size": a["n"], "official": a["o"], "url": a["u"]} for a in news[:600]])
    rows("run", [{"generated_at": gen, "articles_72h": len(news), "events_7d": len(events), "signals_30d": len(sigs), "quakes_30d": len(qs),
                  "sources_ok": sum(1 for s in srcs if s["status"] == "ok"), "sources_failing": sum(1 for s in srcs if s["status"] == "failing"),
                  "sources_total": len(srcs)}])
