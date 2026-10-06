#!/usr/bin/env python3
"""Refuse a status in docs/status.json that its evidence does not support.

Rules (LC-P2-001: no status upgraded without evidence):
- every value is one of the allowed words for its axis;
- every named test exists as `fn <name>` under a #[test] in the crates;
- tests 'partial' needs at least one named test, 'broad' at least three,
  'none' none;
- implementation 'complete' needs tests other than 'none', unless the
  feature note says why (help text, installer);
- legal 'primary-source verified' needs `verification_record` naming a
  file in the repository;
- real-world validation beyond 'synthetic' needs `validation_record`
  naming a file in the repository.

Run from anywhere: python3 ledgercraft/docs/report/check_status.py
Exit code 1 on any problem (used by CI).
"""
import json
import re
import sys
from pathlib import Path

LC = Path(__file__).resolve().parents[2]


def test_names():
    names = set()
    for p in (LC / "crates").rglob("*.rs"):
        text = p.read_text(encoding="utf-8")
        names.update(re.findall(r"#\[test\]\s*(?:#\[[^\]]*\]\s*)*fn\s+([a-z0-9_]+)", text))
    return names


def check(status=None):
    status = status or json.loads((LC / "docs" / "status.json").read_text(encoding="utf-8"))
    axes = status["axes"]
    tests = test_names()
    problems = []
    for f in status["features"]:
        fid = f["id"]
        for axis, allowed in axes.items():
            if f.get(axis) not in allowed:
                problems.append(f"{fid}: {axis} '{f.get(axis)}' is not one of {allowed}")
        ev = f.get("evidence", [])
        for t in ev:
            if t not in tests:
                problems.append(f"{fid}: evidence test '{t}' does not exist")
        need = {"none": (0, 0), "partial": (1, 10**9), "broad": (3, 10**9)}[f.get("tests", "none")] if f.get("tests") in ("none", "partial", "broad") else (0, 10**9)
        if not need[0] <= len(ev) <= need[1]:
            problems.append(f"{fid}: tests '{f.get('tests')}' needs {need[0]}{'+' if need[1] > 0 else ''} named tests, has {len(ev)}")
        if f.get("implementation") == "complete" and f.get("tests") == "none" and not f.get("note"):
            problems.append(f"{fid}: 'complete' without tests needs a note saying why")
        if f.get("legal") == "primary-source verified":
            rec = f.get("verification_record")
            if not rec or not (LC / rec).exists():
                problems.append(f"{fid}: 'primary-source verified' needs verification_record naming a file")
        if f.get("real_world") not in (None, "synthetic"):
            rec = f.get("validation_record")
            if not rec or not (LC / rec).exists():
                problems.append(f"{fid}: real-world '{f.get('real_world')}' needs validation_record naming a file")
    return problems


if __name__ == "__main__":
    probs = check()
    for p in probs:
        print("status:", p)
    print("status.json:", "OK" if not probs else f"{len(probs)} problem(s)")
    sys.exit(1 if probs else 0)
