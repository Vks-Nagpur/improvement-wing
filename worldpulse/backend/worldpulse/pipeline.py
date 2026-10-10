"""Pipeline orchestration. Each stage is a separate function so it can be
run and tested on its own; a failing stage is logged and the rest continue."""
from __future__ import annotations

import os
import time
import traceback
from pathlib import Path

from . import alerts, analyze, briefings, collect, export
from .db import connect, log_error
from .processing.normalize import now_iso

DEFAULT_DB = Path(os.environ.get("WORLDPULSE_DB", Path(__file__).resolve().parents[2] / "data" / "worldpulse.sqlite"))
DEFAULT_OUT = Path(os.environ.get("WORLDPULSE_EXPORT", Path(__file__).resolve().parents[2] / "frontend" / "public" / "data"))


def _stage(conn, name, fn, log):
    t = time.time()
    try:
        res = fn()
        log(f"[{name}] ok in {time.time() - t:.1f}s → {res if not isinstance(res, (dict, list)) else len(res)}")
        return res
    except Exception:
        log(f"[{name}] FAILED\n{traceback.format_exc(limit=4)}")
        log_error(conn, name, None, traceback.format_exc(limit=4), now_iso())
        conn.commit()
        return None


def run(db_path: Path = DEFAULT_DB, out: Path | None = DEFAULT_OUT, force: bool = False, only: set[str] | None = None,
        timelines: bool = True, use_ai: bool = False, log=print) -> dict:
    conn = connect(db_path)
    registry = collect.load_registry()
    collect.seed_countries(conn)
    collect.sync_sources(conn, registry)
    log("[collect] fetching sources")
    _stage(conn, "collect", lambda: collect.Collector(conn, registry, log).run(force=force, only=only), log)
    if timelines:
        _stage(conn, "gdelt-timelines", lambda: collect.collect_gdelt_timelines(conn, registry, log), log)
    _stage(conn, "events", lambda: analyze.build_events(conn), log)
    _stage(conn, "relationships", lambda: analyze.build_relationships(conn), log)
    _stage(conn, "signals", lambda: analyze.detect_signals(conn, registry), log)
    _stage(conn, "alerts", lambda: alerts.evaluate(conn), log)
    bs = _stage(conn, "briefings", lambda: briefings.generate_all(conn, registry, use_ai=use_ai), log) or []
    _stage(conn, "prune", lambda: analyze.prune(conn), log)
    meta = None
    if out:
        meta = _stage(conn, "export", lambda: export.export_all(conn, registry, Path(out), bs), log)
    conn.execute("PRAGMA wal_checkpoint(TRUNCATE)")
    conn.close()
    return meta or {}
