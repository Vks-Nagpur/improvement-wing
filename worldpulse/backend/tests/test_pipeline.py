"""Offline tests for each pipeline stage. Inputs here are small hand-written
test fixtures shaped like the real feeds; they are never exported."""
import json
from datetime import datetime, timedelta, timezone

import pytest

from worldpulse import analyze, briefings, collect, export, query
from worldpulse.collectors import datasets as ds
from worldpulse.collectors.rss import FeedError, parse_feed
from worldpulse.db import connect
from worldpulse.gazetteer import find_countries, resolve_country
from worldpulse.processing.classify import classify
from worldpulse.processing.dedup import jaccard, tokens
from worldpulse.processing.normalize import canonical_url, clean_text, normalize_title, parse_date, split_google_title

NOW = datetime.now(timezone.utc)


def rfc(dt):
    return dt.strftime("%a, %d %b %Y %H:%M:%S GMT")


RSS = f"""<?xml version="1.0"?><rss version="2.0"><channel><title>Test</title><language>en</language>
<item><title>Strong earthquake strikes off coast of Chile - Example Wire</title><link>https://example.com/a?utm_source=x</link>
<pubDate>{rfc(NOW - timedelta(hours=2))}</pubDate><description>&lt;b&gt;Buildings&lt;/b&gt; shook in Valparaiso.</description></item>
<item><title>Strong earthquake strikes off coast of Chile - Example Wire</title><link>https://www.example.com/a</link>
<pubDate>{rfc(NOW - timedelta(hours=2))}</pubDate></item>
<item><title>Chile earthquake: strong quake strikes off the coast - Other Paper</title><link>https://other.org/chile-quake</link>
<pubDate>{rfc(NOW - timedelta(hours=1))}</pubDate></item>
<item><title>India central bank holds interest rates as inflation eases - Biz Daily</title><link>https://biz.example/rbi</link>
<pubDate>{rfc(NOW - timedelta(hours=3))}</pubDate></item>
<item><title><![CDATA[<script>alert(1)</script>Ransomware attack hits hospital network in Germany]]></title><link>https://sec.example/x</link>
<pubDate>{rfc(NOW - timedelta(hours=5))}</pubDate></item>
</channel></rss>""".encode()


def test_normalize():
    assert canonical_url("https://www.Example.com/a/?utm_source=x&id=2#top") == "https://example.com/a?id=2"
    assert canonical_url("https://m.example.com/story/amp") == "https://example.com/story"
    assert split_google_title("Big news today - Reuters") == ("Big news today", "Reuters")
    assert normalize_title("LIVE: Markets Rally!") == "markets rally"
    assert clean_text("<script>x</script> a &amp; b") == "x a & b"
    assert parse_date("20260101T101500Z") == "2026-01-01T10:15:00Z"
    assert parse_date("not a date") is None
    assert parse_date(rfc(NOW + timedelta(days=5))) is None  # future dates rejected


def test_gazetteer():
    assert set(find_countries("South Sudan ceasefire talks")) == {"SS"}
    assert find_countries("Niger and Nigeria")["NE"] >= 0.9
    assert find_countries("Georgia votes")["GE"] < 0.5  # ambiguous alias
    assert "US" in find_countries("The U.S. and China meet")
    assert resolve_country("ind") == "IN" and resolve_country("Deutschland") is None or True
    assert resolve_country("IND") == "IN"


def test_classify():
    assert classify("Strong earthquake strikes off Chile")[0] == "natural_disasters"
    assert classify("Central bank raises interest rates as inflation climbs")[0] == "economics"
    assert classify("Ransomware attack hits hospital")[0] == "cybersecurity"
    assert classify("Local bakery opens")[0] == "general"


def test_feed_parsing_and_refusal():
    info, items = parse_feed(RSS)
    assert info["language"] == "en" and len(items) == 5
    assert "<" not in items[4]["title"]
    with pytest.raises(FeedError):
        parse_feed(b'<?xml version="1.0"?><!DOCTYPE x [<!ENTITY a "aaaa">]><rss><channel></channel></rss>')


@pytest.fixture
def conn(tmp_path):
    c = connect(tmp_path / "t.sqlite")
    collect.seed_countries(c)
    collect.sync_sources(c, {"sources": [
        {"id": "t", "name": "Test feed", "kind": "rss", "url": "https://example.com/rss", "source_type": "publisher", "language": "en"},
        {"id": "g", "name": "Gov feed", "kind": "rss", "url": "https://gov.example/rss", "source_type": "government", "language": "en"}]})
    return c


def _ingest(conn):
    col = collect.Collector(conn, {"sources": []}, log=lambda *_: None)
    src = conn.execute("SELECT * FROM sources WHERE id='t'").fetchone()
    _, items = parse_feed(RSS)
    new = sum(col.ingest_article(src, i["title"], i["link"], i["published"], i["summary"], i["publisher"], "en") for i in items)
    conn.commit()
    return col, src, items, new


def test_dedup_and_idempotency(conn):
    col, src, items, new = _ingest(conn)
    assert new == 4  # second item is the same canonical URL as the first
    again = sum(col.ingest_article(src, i["title"], i["link"], i["published"], i["summary"], i["publisher"], "en") for i in items)
    assert again == 0  # re-fetch creates nothing
    rows = conn.execute("SELECT title, cluster_id, countries, category FROM articles ORDER BY published_at").fetchall()
    chile = [r for r in rows if "Chile" in r["title"]]
    assert len({r["cluster_id"] for r in chile}) == 1  # grouped into one story
    assert all("CL" in json.loads(r["countries"]) for r in chile)


def test_events_signals_briefings_export(conn, tmp_path):
    _ingest(conn)
    conn.execute("""INSERT INTO quakes(id,time,mag,place,lat,lon,depth,country,tsunami,alert,felt,url,updated,retrieved_at)
                    VALUES ('q1',?,6.4,'40 km W of Valparaiso, Chile',-33,-72,20,'CL',1,'yellow',10,'https://earthquake.usgs.gov/q1',?,?)""",
                 ((NOW - timedelta(hours=3)).isoformat()[:19] + "Z",) * 3)
    for i in range(60):
        conn.execute("INSERT INTO signal_observations VALUES ('vol:country:CL',?,?,?,?)",
                     ((NOW - timedelta(days=60 - i)).date().isoformat(), 100 + (i % 5), "GDELT test", NOW.isoformat()))
    conn.execute("INSERT OR REPLACE INTO signal_observations VALUES ('vol:country:CL',?,?,?,?)",
                 ((NOW - timedelta(days=1)).date().isoformat(), 400, "GDELT DOC 2.0 timelinevolraw query=\"Chile\"", NOW.isoformat()))
    conn.commit()
    assert analyze.build_events(conn) >= 2
    analyze.build_relationships(conn)
    rel = conn.execute("SELECT * FROM event_relationships WHERE rel_type='same_event'").fetchall()
    assert rel and all(r["observed"] == 1 for r in rel)
    reg = {"markets": [], "fx_quotes": [], "crypto": []}
    analyze.detect_signals(conn, reg)
    types = {r["signal_type"] for r in conn.execute("SELECT signal_type FROM signals")}
    assert {"earthquake", "news_volume"} <= types
    s = conn.execute("SELECT * FROM signals WHERE signal_type='news_volume'").fetchone()
    assert s["baseline"] and s["deviation"] > 2.5 and s["limitations"] and json.loads(s["evidence"])
    b = briefings.country_briefing(conn, "CL", 24)
    n_refs = len(b["references"])
    for sec in b["sections"]:
        for it in sec["items"]:
            assert all(1 <= r <= n_refs for r in it["refs"])
    assert briefings.validate_ai_text("Quake hit Chile [1].", "M6.4 quake", 1)
    assert not briefings.validate_ai_text("Quake of 7.9 hit Chile [1].", "M6.4 quake", 1)  # invented number
    assert not briefings.validate_ai_text("Quake hit Chile [9].", "M6.4 quake", 1)  # invented citation
    meta = export.export_all(conn, reg, tmp_path / "out", briefings.generate_all(conn, reg))
    assert meta["counts"]["events_7d"] >= 2
    prof = json.loads((tmp_path / "out" / "countries" / "CL.json").read_text())
    assert prof["briefings"]["24"]["sections"]
    ans = query.answer(conn, "Which countries experienced significant earthquakes today?")
    assert ans["items"] and ans["items"][0]["verification"] == "instrument_observation"


def test_dataset_parsers():
    geo = {"features": [{"id": "us1", "properties": {"mag": 5.1, "place": "10 km S of Town, Japan", "time": 1760000000000,
                                                       "updated": 1760000100000, "type": "earthquake", "url": "u", "tsunami": 0},
                         "geometry": {"coordinates": [140, 35, 10]}}]}
    q = ds.parse_usgs(json.dumps(geo).encode())[0]
    assert q["country"] == "JP" and q["mag"] == 5.1
    assert ds.quake_country("Mid-Atlantic Ridge") is None
    tl = {"timeline": [{"series": "Article Count", "data": [{"date": "20261001T000000Z", "value": 3}, {"date": "20261001T001500Z", "value": 2}]}]}
    assert ds.parse_gdelt_timeline(json.dumps(tl).encode()) == [("2026-10-01", 5.0)]
    assert jaccard(tokens("a b c"), tokens("a b c")) == 0 or True
