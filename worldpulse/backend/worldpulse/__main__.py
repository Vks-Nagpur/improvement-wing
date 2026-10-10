"""Command line: python -m worldpulse <command>

  run        fetch due sources, analyse, write briefings and export JSON
  serve      start the local API (127.0.0.1:8000) with the scheduler
  export     re-export JSON from the database without fetching
  ask        answer a question from the local database
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

from . import pipeline


def main() -> None:
    ap = argparse.ArgumentParser(prog="worldpulse")
    ap.add_argument("--db", type=Path, default=pipeline.DEFAULT_DB)
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("run")
    r.add_argument("--out", type=Path, default=pipeline.DEFAULT_OUT)
    r.add_argument("--force", action="store_true", help="ignore per-source intervals")
    r.add_argument("--only", nargs="*", help="source ids or kinds")
    r.add_argument("--no-timelines", action="store_true")
    r.add_argument("--ai", action="store_true", help="add Ollama summaries (validated) to briefings")
    s = sub.add_parser("serve")
    s.add_argument("--host", default="127.0.0.1")
    s.add_argument("--port", type=int, default=8000)
    s.add_argument("--no-scheduler", action="store_true")
    e = sub.add_parser("export")
    e.add_argument("--out", type=Path, default=pipeline.DEFAULT_OUT)
    a = sub.add_parser("ask")
    a.add_argument("question")
    args = ap.parse_args()

    if args.cmd == "run":
        meta = pipeline.run(args.db, args.out, force=args.force, only=set(args.only) if args.only else None,
                            timelines=not args.no_timelines, use_ai=args.ai)
        print(json.dumps(meta.get("counts", {}), indent=2))
    elif args.cmd == "export":
        from . import briefings, collect, export
        from .db import connect
        conn = connect(args.db)
        reg = collect.load_registry()
        print(json.dumps(export.export_all(conn, reg, args.out, briefings.generate_all(conn, reg))["counts"], indent=2))
    elif args.cmd == "ask":
        from .db import connect
        from .query import answer
        print(json.dumps(answer(connect(args.db), args.question), indent=2, ensure_ascii=False))
    elif args.cmd == "serve":
        import uvicorn
        from .api import create_app
        uvicorn.run(create_app(args.db, scheduler=not args.no_scheduler), host=args.host, port=args.port)


if __name__ == "__main__":
    main()
