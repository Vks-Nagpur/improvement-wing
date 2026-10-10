"""Self-hosted REST API (FastAPI). Binds to 127.0.0.1 by default.

All list endpoints take ``limit`` (max 200) and ``offset`` and return
{"items": [...], "total": n, "limit": l, "offset": o}. Errors are
{"error": {"code": ..., "message": ...}}.
"""
from __future__ import annotations

import sqlite3
import threading
import time
from collections import defaultdict, deque
from pathlib import Path

from fastapi import FastAPI, HTTPException, Query, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from pydantic import BaseModel, Field

from . import __version__, alerts, briefings, collect, pipeline
from .db import connect, j, uj
from .gazetteer import COUNTRIES, resolve_country
from .query import answer


class QueryIn(BaseModel):
    question: str = Field(min_length=3, max_length=500)


class RuleIn(BaseModel):
    name: str = Field(min_length=1, max_length=120)
    rule_type: str
    params: dict


def _page(conn, sql, args, limit, offset, conv=dict):
    total = conn.execute(f"SELECT count(*) FROM ({sql})", args).fetchone()[0]
    rows = conn.execute(f"{sql} LIMIT ? OFFSET ?", [*args, limit, offset]).fetchall()
    return {"items": [conv(r) for r in rows], "total": total, "limit": limit, "offset": offset}


def _event(r):
    d = dict(r)
    for k in ("related_countries", "source_refs", "datasets"):
        d[k] = uj(d[k], [])
    return d


def _signal(r):
    d = dict(r)
    d["evidence"] = uj(d["evidence"], [])
    return d


def create_app(db_path: Path = pipeline.DEFAULT_DB, scheduler: bool = True) -> FastAPI:
    app = FastAPI(title="WorldPulse API", version=__version__)
    app.add_middleware(CORSMiddleware, allow_origins=["http://localhost:5173", "http://127.0.0.1:5173"], allow_methods=["GET", "POST", "DELETE"],
                       allow_headers=["content-type"])
    local = threading.local()

    def db() -> sqlite3.Connection:
        if not hasattr(local, "conn"):
            local.conn = connect(db_path)
        return local.conn

    hits: dict[str, deque] = defaultdict(deque)

    @app.middleware("http")
    async def limiter(request: Request, call_next):
        # expensive endpoints: 30 requests / minute / client
        if request.url.path in ("/api/query", "/api/search", "/api/briefings/generate"):
            key = request.client.host if request.client else "?"
            q = hits[key]
            now = time.time()
            while q and now - q[0] > 60:
                q.popleft()
            if len(q) >= 30:
                return JSONResponse({"error": {"code": "rate_limited", "message": "Too many requests; try again in a minute."}}, status_code=429)
            q.append(now)
        return await call_next(request)

    @app.exception_handler(HTTPException)
    async def http_err(_, exc: HTTPException):
        return JSONResponse({"error": {"code": exc.status_code, "message": exc.detail}}, status_code=exc.status_code)

    L = Query(50, ge=1, le=200)
    O = Query(0, ge=0)

    @app.get("/api/health")
    def health():
        conn = db()
        srcs = conn.execute("SELECT id, name, last_success, last_error, consecutive_failures FROM sources WHERE enabled=1").fetchall()
        return {"status": "ok", "version": __version__,
                "last_article": conn.execute("SELECT max(collected_at) FROM articles").fetchone()[0],
                "sources_ok": sum(1 for s in srcs if not s["consecutive_failures"] and s["last_success"]),
                "sources_failing": [dict(s) for s in srcs if s["consecutive_failures"]]}

    @app.get("/api/sources")
    def sources():
        return {"items": [dict(r) for r in db().execute("SELECT * FROM sources ORDER BY kind, name")]}

    @app.get("/api/news")
    def news(country: str | None = None, category: str | None = None, source: str | None = None, language: str | None = None,
             since: str | None = None, q: str | None = None, include_duplicates: bool = False, limit: int = L, offset: int = O):
        where, args = ["1=1"], []
        if not include_duplicates:
            where.append("duplicate_of IS NULL")
        if country:
            where.append("countries LIKE ?")
            args.append(f'%"{(resolve_country(country) or country).upper()}"%')
        for col, val in (("category", category), ("source_id", source), ("language", language)):
            if val:
                where.append(f"{col}=?")
                args.append(val)
        if since:
            where.append("coalesce(published_at, collected_at) >= ?")
            args.append(since)
        if q:
            where.append("rowid IN (SELECT rowid FROM articles_fts WHERE articles_fts MATCH ?)")
            args.append(_fts(q))
        sql = f"SELECT * FROM articles WHERE {' AND '.join(where)} ORDER BY coalesce(published_at, collected_at) DESC"
        return _page(db(), sql, args, limit, offset, lambda r: dict(r) | {"countries": uj(r["countries"], {}), "category_scores": uj(r["category_scores"], {})})

    @app.get("/api/events")
    def events(country: str | None = None, category: str | None = None, verification: str | None = None, since: str | None = None,
               limit: int = L, offset: int = O):
        where, args = ["1=1"], []
        if country:
            iso = (resolve_country(country) or country).upper()
            where.append("(country=? OR related_countries LIKE ?)")
            args += [iso, f'%"{iso}"%']
        if category:
            where.append("category=?")
            args.append(category)
        if verification:
            where.append("verification=?")
            args.append(verification)
        if since:
            where.append("last_updated >= ?")
            args.append(since)
        return _page(db(), f"SELECT * FROM events WHERE {' AND '.join(where)} ORDER BY last_updated DESC", args, limit, offset, _event)

    @app.get("/api/events/{event_id}")
    def event(event_id: str):
        conn = db()
        r = conn.execute("SELECT * FROM events WHERE id=?", (event_id,)).fetchone()
        if not r:
            raise HTTPException(404, "event not found")
        rels = [dict(x) for x in conn.execute("SELECT * FROM event_relationships WHERE source_event=? OR target_event=?", (event_id, event_id))]
        revs = [dict(x) | {"snapshot": uj(x["snapshot"])} for x in conn.execute("SELECT * FROM event_revisions WHERE event_id=? ORDER BY revision", (event_id,))]
        return _event(r) | {"relationships": rels, "revisions": revs}

    @app.get("/api/countries")
    def countries(region: str | None = None, q: str | None = None):
        rows = db().execute("SELECT * FROM countries ORDER BY name").fetchall()
        items = [dict(r) for r in rows if (not region or r["region"] == region) and (not q or q.lower() in r["name"].lower())]
        return {"items": items, "total": len(items)}

    def _iso(code: str) -> str:
        iso = resolve_country(code)
        if not iso or iso not in COUNTRIES:
            raise HTTPException(404, f"unknown country: {code}")
        return iso

    @app.get("/api/countries/{code}")
    def country(code: str, hours: int = Query(168, ge=1, le=24 * 45)):
        iso = _iso(code)
        row = db().execute("SELECT * FROM countries WHERE iso2=?", (iso,)).fetchone()
        return {"country": dict(row), "briefing": briefings.country_briefing(db(), iso, hours)}

    @app.get("/api/countries/{code}/news")
    def country_news(code: str, limit: int = L, offset: int = O):
        return news(country=_iso(code), limit=limit, offset=offset)

    @app.get("/api/countries/{code}/indicators")
    def country_indicators(code: str):
        iso = _iso(code)
        return {"items": [dict(r) for r in db().execute("SELECT * FROM country_indicators WHERE iso2=? ORDER BY indicator, period DESC", (iso,))]}

    @app.get("/api/signals")
    def signals(signal_type: str | None = None, country: str | None = None, since: str | None = None, limit: int = L, offset: int = O):
        where, args = ["1=1"], []
        if signal_type:
            where.append("signal_type=?")
            args.append(signal_type)
        if country:
            where.append("country=?")
            args.append(_iso(country))
        if since:
            where.append("detected_at >= ?")
            args.append(since)
        return _page(db(), f"SELECT * FROM signals WHERE {' AND '.join(where)} ORDER BY detected_at DESC, score DESC", args, limit, offset, _signal)

    @app.get("/api/signals/{signal_id}")
    def signal(signal_id: str):
        r = db().execute("SELECT * FROM signals WHERE id=?", (signal_id,)).fetchone()
        if not r:
            raise HTTPException(404, "signal not found")
        return _signal(r)

    @app.get("/api/briefings")
    def list_briefings(kind: str | None = None, country: str | None = None, hours: int = Query(24, ge=1, le=24 * 45)):
        conn = db()
        if country:
            return {"items": [briefings.country_briefing(conn, _iso(country), hours)]}
        reg = collect.load_registry()
        fns = {"global": lambda: briefings.global_briefing(conn, hours), "geopolitical": lambda: briefings.geopolitical_briefing(conn, hours),
               "market": lambda: briefings.market_briefing(conn, reg, hours), "signals": lambda: briefings.signals_briefing(conn, hours)}
        if kind and kind not in fns:
            raise HTTPException(400, f"kind must be one of {sorted(fns)}")
        return {"items": [fns[k]() for k in ([kind] if kind else fns)]}

    @app.get("/api/search")
    def search(q: str = Query(..., min_length=2, max_length=200), country: str | None = None, category: str | None = None,
               since: str | None = None, until: str | None = None, source: str | None = None, language: str | None = None,
               verification: str | None = None, sort: str = Query("relevance", pattern="^(relevance|date)$"), limit: int = L, offset: int = O):
        conn = db()
        results = []
        where, args = ["articles_fts MATCH ?"], [_fts(q)]
        for cond, val in (("a.countries LIKE ?", f'%"{_iso(country)}"%' if country else None), ("a.category=?", category),
                          ("a.source_id=?", source), ("a.language=?", language),
                          ("coalesce(a.published_at,a.collected_at) >= ?", since), ("coalesce(a.published_at,a.collected_at) <= ?", until)):
            if val:
                where.append(cond)
                args.append(val)
        order = "bm25(articles_fts)" if sort == "relevance" else "coalesce(a.published_at,a.collected_at) DESC"
        for r in conn.execute(f"""SELECT a.*, bm25(articles_fts) AS rank FROM articles_fts JOIN articles a ON a.rowid=articles_fts.rowid
                                  WHERE {' AND '.join(where)} AND a.duplicate_of IS NULL ORDER BY {order} LIMIT 500""", args):
            results.append({"type": "article", "title": r["title"], "description": r["excerpt"], "date": r["published_at"] or r["collected_at"],
                            "source": r["publisher"], "country": list(uj(r["countries"], {})), "category": r["category"], "url": r["url"]})
        ev_where, ev_args = ["title LIKE ?"], [f"%{q}%"]
        if verification:
            ev_where.append("verification=?")
            ev_args.append(verification)
        for r in conn.execute(f"SELECT * FROM events WHERE {' AND '.join(ev_where)} ORDER BY last_updated DESC LIMIT 100", ev_args):
            refs = uj(r["source_refs"], [])
            results.append({"type": "event", "id": r["id"], "title": r["title"], "description": r["description"], "date": r["started_at"],
                            "source": refs[0].get("publisher") if refs else None, "country": uj(r["related_countries"], []),
                            "category": r["category"], "url": refs[0].get("url") if refs else None, "verification": r["verification"]})
        for c in COUNTRIES.values():
            if q.lower() in c.name.lower():
                results.insert(0, {"type": "country", "title": c.name, "description": c.region, "country": [c.iso2], "url": f"/countries/{c.iso2}"})
        for r in conn.execute("SELECT DISTINCT indicator, label FROM country_indicators WHERE label LIKE ?", (f"%{q}%",)):
            results.append({"type": "indicator", "title": r["label"], "description": r["indicator"], "source": "World Bank WDI"})
        if sort == "date":
            results.sort(key=lambda x: x.get("date") or "", reverse=True)
        return {"items": results[offset:offset + limit], "total": len(results), "limit": limit, "offset": offset}

    @app.post("/api/query")
    def query(body: QueryIn):
        return answer(db(), body.question)

    @app.get("/api/alerts/rules")
    def rules():
        return {"items": [dict(r) | {"params": uj(r["params"], {})} for r in db().execute("SELECT * FROM alert_rules ORDER BY id")]}

    @app.post("/api/alerts/rules")
    def add_rule(body: RuleIn):
        try:
            alerts.validate_rule(body.rule_type, body.params)
        except ValueError as e:
            raise HTTPException(400, str(e)) from e
        conn = db()
        cur = conn.execute("INSERT INTO alert_rules(name, rule_type, params, created_at) VALUES (?,?,?,datetime('now'))",
                           (body.name, body.rule_type, j(body.params)))
        conn.commit()
        return {"id": cur.lastrowid}

    @app.delete("/api/alerts/rules/{rule_id}")
    def del_rule(rule_id: int):
        conn = db()
        conn.execute("DELETE FROM alert_rules WHERE id=?", (rule_id,))
        conn.commit()
        return {"deleted": rule_id}

    @app.get("/api/alerts/history")
    def alert_history(limit: int = L, offset: int = O):
        return _page(db(), "SELECT * FROM alert_history ORDER BY id DESC", [], limit, offset, lambda r: dict(r) | {"evidence": uj(r["evidence"], [])})

    if scheduler:
        from apscheduler.schedulers.background import BackgroundScheduler
        sched = BackgroundScheduler(timezone="UTC")
        # every 15 minutes: collect only what is due (per-source intervals), analyse, export
        sched.add_job(lambda: pipeline.run(db_path, pipeline.DEFAULT_OUT, timelines=False), "interval", minutes=15, id="collect",
                      max_instances=1, coalesce=True, next_run_time=None)
        sched.add_job(lambda: pipeline.run(db_path, pipeline.DEFAULT_OUT, only={"none"}, timelines=True), "interval", hours=3, id="timelines",
                      max_instances=1, coalesce=True)
        app.add_event_handler("startup", sched.start)
        app.add_event_handler("shutdown", sched.shutdown)

    return app


def _fts(q: str) -> str:
    """Turn user text into a safe FTS5 query (quoted terms, implicit AND)."""
    terms = [t for t in "".join(ch if ch.isalnum() or ch in " -'" else " " for ch in q).split() if len(t) > 1][:10]
    if not terms:
        raise HTTPException(400, "query has no searchable terms")
    return " ".join('"' + t.replace('"', "") + '"' for t in terms)
