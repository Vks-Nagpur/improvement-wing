"""RSS 2.0 / RDF / Atom parsing with the standard library.

Feeds are untrusted: documents declaring entities are refused (no entity
expansion, no external fetches), size is capped by the fetcher, and every
text field is stripped of markup before storage.
"""
from __future__ import annotations

import xml.etree.ElementTree as ET

from ..processing.normalize import clean_text, parse_date


class FeedError(Exception):
    pass


def _local(tag: str) -> str:
    return tag.rsplit("}", 1)[-1] if "}" in tag else tag


def _child_text(el: ET.Element, *names: str) -> str | None:
    for c in el:
        if _local(c.tag) in names:
            if _local(c.tag) == "link" and c.get("href"):
                return c.get("href")
            if c.text and c.text.strip():
                return c.text.strip()
    return None


def _children(el: ET.Element, name: str) -> list[ET.Element]:
    return [c for c in el if _local(c.tag) == name]


def parse_feed(content: bytes) -> tuple[dict, list[dict]]:
    """Return (feed_info, items). Each item: title, link, published, summary,
    publisher (if the feed names one per item), extra (namespaced fields)."""
    if not content.strip():
        raise FeedError("empty response (the publisher may block automated requests)")
    head = content[:4096].lower()
    if b"<!entity" in head or b"<!doctype" in head and b"[" in head:
        raise FeedError("feed declares a DTD/entities; refused")
    try:
        root = ET.fromstring(content)
    except ET.ParseError as e:
        raise FeedError(f"XML parse error: {e}") from e
    tag = _local(root.tag)
    if tag == "rss":
        channel = next((c for c in root if _local(c.tag) == "channel"), None)
        if channel is None:
            raise FeedError("RSS without channel")
        info = {"title": _child_text(channel, "title"), "language": _child_text(channel, "language")}
        raw_items = _children(channel, "item")
    elif tag == "RDF":
        channel = next((c for c in root if _local(c.tag) == "channel"), None)
        info = {"title": _child_text(channel, "title") if channel is not None else None,
                "language": _child_text(channel, "language") if channel is not None else None}
        raw_items = _children(root, "item")
    elif tag == "feed":
        info = {"title": _child_text(root, "title"), "language": root.get("{http://www.w3.org/XML/1998/namespace}lang")}
        raw_items = _children(root, "entry")
    else:
        raise FeedError(f"unknown feed root <{tag}>")

    items = []
    for it in raw_items[:500]:
        title = clean_text(_child_text(it, "title"), 300)
        link = _child_text(it, "link", "guid")
        if not title or not link:
            continue
        src = next((c for c in it if _local(c.tag) == "source"), None)
        extra = {}
        for c in it:
            if "}" in c.tag and c.text and c.text.strip():
                extra[_local(c.tag)] = c.text.strip()[:300]
            for k, v in c.attrib.items():
                extra[f"{_local(c.tag)}@{_local(k)}"] = v[:300]
        items.append({
            "title": title,
            "link": link.strip(),
            "published": parse_date(_child_text(it, "pubDate", "published", "updated", "date")),
            "summary": clean_text(_child_text(it, "description", "summary", "content"), 400),
            "publisher": clean_text(src.text, 100) if src is not None and src.text else None,
            "extra": extra,
        })
    return info, items
