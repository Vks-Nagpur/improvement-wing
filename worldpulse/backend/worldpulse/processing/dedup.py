"""Duplicate detection and story clustering.

Levels, in order:
1. exact URL — the same link seen twice
2. canonical URL — the same page with tracking parameters, AMP or 'www.'
3. normalized headline — identical headline text after normalization
   (only within 3 days, so a recurring headline like "Markets today" next
   week is not merged)
4. similarity — near-identical headlines (token-set Jaccard >= 0.8 and the
   same leading words) are flagged as duplicates

Story clusters are looser: Jaccard >= 0.45 on content words, within 48 hours,
and sharing at least one named entity (capitalised word / country). Similar
headlines do not always describe the same event, so a cluster is presented as
"related coverage", never as proof that the reports agree.
"""
from __future__ import annotations

import hashlib
import re
from datetime import datetime, timedelta

from .normalize import parse_iso

STOP = set("""a an the of and or to in on for with at by from as is are was were be been has have had will would could
should may might can its it this that these those after before over under into about amid against new says said
report reports reported than more most up down out off not no but how why what who when where which while""".split())


def tokens(norm_title: str) -> set[str]:
    return {t for t in norm_title.split() if t not in STOP and len(t) > 1}


def jaccard(a: set[str], b: set[str]) -> float:
    if not a or not b:
        return 0.0
    return len(a & b) / len(a | b)


def entities(title: str) -> set[str]:
    return {w.lower() for w in re.findall(r"\b[A-Z][a-zA-Z]{2,}\b", title)} - {"the", "new", "how", "why", "what", "live"}


class Index:
    """In-memory index of recent articles used for steps 3–4 and clustering."""

    def __init__(self) -> None:
        self.items: list[dict] = []
        self.by_norm: dict[str, dict] = {}

    def add(self, item: dict) -> None:
        item["_tok"] = tokens(item["norm_title"])
        item["_ent"] = entities(item["title"]) | set(item.get("countries", {}).keys())
        self.items.append(item)
        self.by_norm.setdefault(item["norm_title"], item)

    def find_duplicate(self, item: dict) -> tuple[dict, str] | None:
        t = parse_iso(item.get("published_at")) or parse_iso(item["collected_at"])
        same = self.by_norm.get(item["norm_title"])
        if same and _within(t, same, timedelta(days=3)) and len(item["norm_title"]) > 15:
            return same, "headline"
        tok = tokens(item["norm_title"])
        lead = item["norm_title"].split()[:3]
        for other in reversed(self.items[-3000:]):
            if not _within(t, other, timedelta(days=2)):
                continue
            if other["norm_title"].split()[:3] == lead and jaccard(tok, other["_tok"]) >= 0.8:
                return other, "similarity"
        return None

    def find_cluster(self, item: dict) -> str | None:
        t = parse_iso(item.get("published_at")) or parse_iso(item["collected_at"])
        tok = tokens(item["norm_title"])
        ent = entities(item["title"]) | set(item.get("countries", {}).keys())
        best, best_s = None, 0.0
        for other in reversed(self.items[-3000:]):
            if not other.get("cluster_id") or not _within(t, other, timedelta(hours=48)):
                continue
            s = jaccard(tok, other["_tok"])
            if s >= 0.45 and s > best_s and (ent & other["_ent"]):
                best, best_s = other["cluster_id"], s
        return best


def _within(t: datetime | None, other: dict, delta: timedelta) -> bool:
    o = parse_iso(other.get("published_at")) or parse_iso(other["collected_at"])
    if t is None or o is None:
        return True
    return abs(t - o) <= delta


def cluster_id_for(article_id: str) -> str:
    return "c_" + hashlib.sha1(article_id.encode()).hexdigest()[:12]
