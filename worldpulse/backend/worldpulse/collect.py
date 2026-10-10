"""Collection stage: fetch every enabled source, isolate failures, store raw
records idempotently and log each attempt."""
from __future__ import annotations

import json
import os
import sqlite3
import traceback
from datetime import datetime, timedelta, timezone
from pathlib import Path
from urllib.parse import quote

from . import PROCESSING_VERSION
from .collectors import datasets as ds
from .collectors.rss import parse_feed
from .db import j, log_error, tx
from .gazetteer import COUNTRIES, find_countries
from .http import FetchError, fetch
from .processing.classify import classify
from .processing.dedup import Index, cluster_id_for
from .processing.normalize import (canonical_url, clean_text, detect_language, normalize_title, now_iso, parse_date,
                                   split_google_title, url_id)

REGISTRY = Path(__file__).with_name("sources.json")


def load_registry() -> dict:
    return json.loads(REGISTRY.read_text())


def sync_sources(conn: sqlite3.Connection, registry: dict) -> None:
    with tx(conn):
        for s in registry["sources"]:
            conn.execute(
                """INSERT INTO sources(id,name,kind,url,category,country,language,source_type,interval_minutes,enabled)
                   VALUES (?,?,?,?,?,?,?,?,?,?)
                   ON CONFLICT(id) DO UPDATE SET name=excluded.name, kind=excluded.kind, url=excluded.url,
                     category=excluded.category, country=excluded.country, language=excluded.language,
                     source_type=excluded.source_type, interval_minutes=excluded.interval_minutes, enabled=excluded.enabled""",
                (s["id"], s["name"], s["kind"], s["url"], s.get("category"), s.get("country"), s.get("language"),
                 s.get("source_type"), s.get("interval_minutes", 30), int(s.get("enabled", True))))


def _due(row: sqlite3.Row, force: bool) -> bool:
    if force or not row["last_attempt"]:
        return True
    last = datetime.fromisoformat(row["last_attempt"].replace("Z", "+00:00"))
    backoff = min(2 ** row["consecutive_failures"], 16)  # back off failing sources
    return datetime.now(timezone.utc) - last >= timedelta(minutes=row["interval_minutes"] * backoff)


class Collector:
    def __init__(self, conn: sqlite3.Connection, registry: dict, log=print):
        self.conn = conn
        self.registry = registry
        self.log = log
        self.index = Index()
        self._load_index()

    def _load_index(self) -> None:
        cutoff = (datetime.now(timezone.utc) - timedelta(days=3)).isoformat()
        for r in self.conn.execute(
                "SELECT id,title,norm_title,published_at,collected_at,countries,cluster_id FROM articles "
                "WHERE duplicate_of IS NULL AND collected_at >= ? ORDER BY collected_at", (cutoff[:19],)):
            self.index.add({**dict(r), "countries": json.loads(r["countries"] or "{}")})

    # ---- bookkeeping ----
    def _start(self, sid: str) -> tuple[int, str]:
        started = now_iso()
        cur = self.conn.execute("INSERT INTO fetch_logs(source_id, started_at, status) VALUES (?,?,'running')", (sid, started))
        self.conn.execute("UPDATE sources SET last_attempt=? WHERE id=?", (started, sid))
        self.conn.commit()
        return cur.lastrowid, started

    def _finish(self, log_id: int, sid: str, status: str, items: int = 0, new: int = 0, err: str | None = None,
                http_status: int | None = None) -> None:
        fin = now_iso()
        self.conn.execute("UPDATE fetch_logs SET finished_at=?, status=?, items=?, new_items=?, error=?, http_status=? WHERE id=?",
                          (fin, status, items, new, err, http_status, log_id))
        if status in ("ok", "not_modified"):
            self.conn.execute("UPDATE sources SET last_success=?, last_error=NULL, consecutive_failures=0 WHERE id=?", (fin, sid))
        else:
            self.conn.execute("UPDATE sources SET last_error=?, consecutive_failures=consecutive_failures+1 WHERE id=?", (err, sid))
        self.conn.commit()

    # ---- run ----
    def run(self, force: bool = False, only: set[str] | None = None, budget_s: float = 720) -> dict:
        import time as _time
        deadline = _time.monotonic() + budget_s
        summary = {}
        rows = self.conn.execute("SELECT * FROM sources WHERE enabled=1").fetchall()
        for row in rows:
            if only and row["id"] not in only and row["kind"] not in only:
                continue
            if not _due(row, force):
                summary[row["id"]] = "not due"
                continue
            handler = getattr(self, f"_do_{row['kind']}", None)
            if handler is None:
                continue
            if _time.monotonic() > deadline:
                summary[row["id"]] = "skipped (run time budget reached)"
                self.log(f"  {row['id']}: skipped (time budget)")
                continue
            t_src = _time.monotonic()
            log_id, _ = self._start(row["id"])
            try:
                status, items, new = handler(row)
                self._finish(log_id, row["id"], status, items, new)
                summary[row["id"]] = f"{status} items={items} new={new}"
            except Exception as e:  # one source must never stop the run
                msg = f"{type(e).__name__}: {e}"[:500]
                code = getattr(e, "status", None)
                self._finish(log_id, row["id"], "error", err=msg, http_status=code)
                log_error(self.conn, "fetch", row["id"], traceback.format_exc(limit=3), now_iso())
                self.conn.commit()
                summary[row["id"]] = f"error {msg}"
            self.log(f"  {row['id']}: {summary[row['id']]} ({_time.monotonic() - t_src:.1f}s)")
        return summary

    # ---- news ----
    def _do_rss(self, row) -> tuple[str, int, int]:
        r = fetch(row["url"], etag=row["etag"], last_modified=row["last_modified"])
        if r.not_modified:
            return "not_modified", 0, 0
        self.conn.execute("UPDATE sources SET etag=?, last_modified=? WHERE id=?",
                          (r.headers.get("etag"), r.headers.get("last-modified"), row["id"]))
        info, items = parse_feed(r.content)
        lang = row["language"] or info.get("language")
        new = 0
        with tx(self.conn):
            for it in items:
                new += self.ingest_article(row, it["title"], it["link"], it["published"], it["summary"], it["publisher"], lang)
        return "ok", len(items), new

    def _do_gdelt(self, row) -> tuple[str, int, int]:
        r = fetch(row["url"].replace(" ", "%20"))
        items = ds.parse_gdelt_artlist(r.content)
        new = 0
        with tx(self.conn):
            for it in items:
                lang = "en" if it["language"].lower() == "english" else None
                new += self.ingest_article(row, it["title"], it["link"], parse_date(it["published"]), "", it["publisher"], lang)
        return "ok", len(items), new

    def ingest_article(self, src, title, link, published, summary, publisher, lang) -> int:
        """Store one article. Returns 1 if it is new, 0 if it was already stored
        (idempotent: re-fetching a feed never duplicates rows)."""
        collected = now_iso()
        if src["source_type"] == "aggregator" and "news.google.com" in src["url"]:
            title, pub2 = split_google_title(title)
            publisher = publisher or pub2
            if summary and title and summary.startswith(title[:30]):
                summary = ""  # Google's description repeats the headline
        publisher = publisher or src["name"]
        canon = canonical_url(link)
        aid = url_id(canon)
        if self.conn.execute("SELECT 1 FROM articles WHERE canonical_url=?", (canon,)).fetchone():
            return 0
        norm = normalize_title(title)
        if not norm:
            return 0
        text = f"{title}. {summary or ''}"
        countries = find_countries(text)
        category, scores = classify(text, src["category"])
        item = {"id": aid, "title": title, "norm_title": norm, "published_at": published, "collected_at": collected,
                "countries": countries}
        dup = self.index.find_duplicate(item)
        dup_of, method, cluster = (dup[0]["id"], dup[1], dup[0].get("cluster_id")) if dup else (None, None, None)
        if not dup:
            cluster = self.index.find_cluster(item) or cluster_id_for(aid)
        item["cluster_id"] = cluster
        self.conn.execute(
            """INSERT OR IGNORE INTO articles(id,source_id,url,canonical_url,title,norm_title,publisher,excerpt,language,
               published_at,collected_at,category,category_scores,countries,cluster_id,duplicate_of,duplicate_method,
               is_official,processing_version) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)""",
            (aid, src["id"], link[:1000], canon[:1000], title, norm, clean_text(publisher, 100), clean_text(summary, 300),
             detect_language(text, lang), published, collected, category, j(scores), j(countries), cluster, dup_of, method,
             int(src["source_type"] in ("government", "institution")), PROCESSING_VERSION))
        if not dup:
            self.index.add(item)
        return 1

    # ---- disasters ----
    def _do_usgs(self, row) -> tuple[str, int, int]:
        r = fetch(row["url"])
        quakes = ds.parse_usgs(r.content)
        got = now_iso()
        new = 0
        with tx(self.conn):
            for q in quakes:
                if not q["time"]:
                    continue
                cur = self.conn.execute(
                    """INSERT INTO quakes(id,time,mag,place,lat,lon,depth,country,tsunami,alert,felt,url,updated,retrieved_at)
                       VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)
                       ON CONFLICT(id) DO UPDATE SET mag=excluded.mag, place=excluded.place, alert=excluded.alert,
                         felt=excluded.felt, tsunami=excluded.tsunami, updated=excluded.updated, retrieved_at=excluded.retrieved_at
                       WHERE excluded.updated IS NOT quakes.updated""",
                    (q["id"], q["time"], q["mag"], q["place"], q["lat"], q["lon"], q["depth"], q["country"], q["tsunami"],
                     q["alert"], q["felt"], q["url"], q["updated"], got))
                new += cur.rowcount
        return "ok", len(quakes), new

    def _do_gdacs(self, row) -> tuple[str, int, int]:
        r = fetch(row["url"])
        _, items = parse_feed(r.content)
        got = now_iso()
        new = 0
        with tx(self.conn):
            for it in items:
                rec = ds.gdacs_record(it)
                ex = self.conn.execute("SELECT 1 FROM signal_observations WHERE series=? AND ts=?",
                                       (f"gdacs:{rec['event_type']}:{rec['event_id']}", rec["published"] or got)).fetchone()
                self.conn.execute("INSERT OR REPLACE INTO signal_observations(series,ts,value,source,retrieved_at) VALUES (?,?,?,?,?)",
                                  (f"gdacs:{rec['event_type']}:{rec['event_id']}", rec["published"] or got,
                                   {"Green": 1, "Orange": 2, "Red": 3}.get(rec["alert_level"] or "", 0), j(rec), got))
                new += 0 if ex else 1
        return "ok", len(items), new

    def _do_ioda(self, row) -> tuple[str, int, int]:
        now = datetime.now(timezone.utc)
        url = f"{row['url']}?from={int((now - timedelta(hours=48)).timestamp())}&until={int(now.timestamp())}&entityType=country&limit=2000"
        r = fetch(url)
        alerts = ds.parse_ioda_alerts(r.content)
        got = now_iso()
        with tx(self.conn):
            for a in alerts:
                self.conn.execute("INSERT OR REPLACE INTO signal_observations(series,ts,value,source,retrieved_at) VALUES (?,?,?,?,?)",
                                  (f"ioda:{a['country']}:{a['datasource']}", a["time"], float(a["value"] or 0), j(a), got))
        return "ok", len(alerts), len(alerts)

    def _do_firms(self, row) -> tuple[str, int, int]:
        key = os.environ.get("FIRMS_MAP_KEY")
        if not key:
            raise FetchError("FIRMS_MAP_KEY not set (free key from firms.modaps.eosdis.nasa.gov)")
        got = now_iso()
        total = 0
        with tx(self.conn):
            for iso2 in self.registry.get("volume_watchlist", [])[:15]:
                iso3 = COUNTRIES[iso2].iso3
                r = fetch(f"{row['url']}{key}/VIIRS_SNPP_NRT/{iso3}/1")
                n = len(ds.parse_firms_csv(r.content))
                total += n
                self.conn.execute("INSERT OR REPLACE INTO signal_observations(series,ts,value,source,retrieved_at) VALUES (?,?,?,?,?)",
                                  (f"firms:{iso2}", got[:10], n, "NASA FIRMS VIIRS_SNPP_NRT", got))
        return "ok", total, total

    # ---- country data ----
    def _do_worldbank(self, row) -> tuple[str, int, int]:
        base = row["url"]
        got = now_iso()
        r = fetch(f"{base}country?format=json&per_page=400")
        meta = ds.parse_wb_countries(r.content)
        with tx(self.conn):
            for m in meta:
                self.conn.execute("""UPDATE countries SET wb_region=?, income_level=?, capital=?, lat=?, lon=?, meta_source=?, meta_updated=?
                                     WHERE iso2=?""", (m["wb_region"], m["income_level"], m["capital"], m["lat"], m["lon"],
                                                        "World Bank country API", got, m["iso2"]))
        total = len(meta)
        failures = []
        for ind, (label, unit) in ds.WB_INDICATORS.items():
            url = f"{base}country/all/indicator/{ind}?format=json&mrnev=1&per_page=400"
            try:
                rows = ds.parse_wb_indicator(fetch(url).content, ind)
            except Exception as e:
                failures.append(f"{ind}: {e}")
                continue
            with tx(self.conn):
                for v in rows:
                    self.conn.execute("""INSERT OR REPLACE INTO country_indicators(iso2,indicator,label,value,unit,period,source,source_url,retrieved_at)
                                         VALUES (?,?,?,?,?,?,?,?,?)""",
                                      (v["iso2"], ind, label, v["value"], unit, v["period"], "World Bank WDI",
                                       f"https://data.worldbank.org/indicator/{ind}?locations={v['iso2']}", got))
            total += len(rows)
        if failures and len(failures) == len(ds.WB_INDICATORS):
            raise FetchError("; ".join(failures)[:500])
        return "ok", total, total

    # ---- markets ----
    def _store_series(self, series: str, points, source: str) -> int:
        got = now_iso()
        n = 0
        for day, v in points:
            n += self.conn.execute("INSERT OR REPLACE INTO signal_observations(series,ts,value,source,retrieved_at) VALUES (?,?,?,?,?)",
                                   (series, day, v, source, got)).rowcount
        return n

    def _do_market(self, row) -> tuple[str, int, int]:
        n, errors = 0, []
        for m in self.registry.get("markets", []):
            try:
                r = fetch(f"{row['url']}{quote(m['symbol'])}?range=6mo&interval=1d")
                chart = ds.parse_yahoo_chart(r.content)
                with tx(self.conn):
                    n += self._store_series(f"mkt:{m['symbol']}", chart["points"],
                                            f"Yahoo Finance chart (delayed; exchange {chart['exchange']}; last trade {chart['market_time']})")
            except Exception as e:
                errors.append(f"{m['symbol']}: {e}")
        if errors and n == 0:
            raise FetchError("; ".join(errors)[:500])
        return "ok", n, n

    def _do_fx(self, row) -> tuple[str, int, int]:
        start = (datetime.now(timezone.utc) - timedelta(days=200)).date().isoformat()
        quotes = ",".join(self.registry.get("fx_quotes", []))
        r = fetch(f"{row['url']}{start}..?from=USD&to={quotes}")
        series = ds.parse_frankfurter_series(r.content)
        n = 0
        with tx(self.conn):
            for cur, pts in series.items():
                n += self._store_series(f"fx:USD{cur}", pts, "European Central Bank reference rate via Frankfurter (published ~16:00 CET on TARGET days)")
        return "ok", n, n

    def _do_crypto(self, row) -> tuple[str, int, int]:
        n = 0
        for coin in self.registry.get("crypto", []):
            r = fetch(f"{row['url']}coins/{coin}/market_chart?vs_currency=usd&days=180&interval=daily")
            with tx(self.conn):
                n += self._store_series(f"crypto:{coin}", ds.parse_coingecko_chart(r.content), "CoinGecko public API (aggregated spot price)")
        return "ok", n, n


def collect_gdelt_timelines(conn: sqlite3.Connection, registry: dict, log=print, budget_s: float = 360, max_series: int = 10) -> dict:
    """News-volume baselines from GDELT: daily article counts (30 days) for
    each watch-list country name and each geopolitical topic query.
    Stops after ``budget_s`` seconds so a slow GDELT cannot stall the run;
    series not reached keep their previous values."""
    import time as _time
    deadline = _time.monotonic() + budget_s
    summary = {}
    got = now_iso()
    jobs = [(f"vol:country:{iso2}", f'"{COUNTRIES[iso2].name}"') for iso2 in registry.get("volume_watchlist", [])]
    jobs += [(f"vol:topic:{k}", q) for k, q in registry.get("topic_watchlist", {}).items()]
    # GDELT rate-limits shared IPs hard. Fetch the series refreshed longest ago
    # first and only a few per run; the database keeps the rest from earlier runs.
    last = dict(conn.execute("SELECT series, max(retrieved_at) FROM signal_observations WHERE series LIKE 'vol:%' GROUP BY series").fetchall())
    jobs.sort(key=lambda jq: last.get(jq[0]) or "")
    jobs = jobs[:max_series]
    log_id = conn.execute("INSERT INTO fetch_logs(source_id, started_at, status) VALUES ('gdelt-timelines',?, 'running')", (got,)).lastrowid
    ok = 0
    for series, q in jobs:
        if _time.monotonic() > deadline:
            summary[series] = "skipped (time budget)"
            continue
        url = ("https://api.gdeltproject.org/api/v2/doc/doc?query=" + quote(q + " sourcelang:english") +
               "&mode=timelinevolraw&format=json&timespan=30d")
        try:
            pts = ds.parse_gdelt_timeline(fetch(url, retries=1, timeout=(10, 20)).content)
            with tx(conn):
                for day, v in pts:
                    conn.execute("INSERT OR REPLACE INTO signal_observations(series,ts,value,source,retrieved_at) VALUES (?,?,?,?,?)",
                                 (series, day, v, f"GDELT DOC 2.0 timelinevolraw query={q}", got))
            ok += 1
            summary[series] = len(pts)
            log(f"  {series}: {len(pts)} days")
        except Exception as e:
            summary[series] = f"error {e}"[:200]
            log(f"  {series}: {summary[series]}")
            if "429" in str(e):
                _time.sleep(15)  # back off harder after a rate-limit answer
            log_error(conn, "fetch", series, str(e), got)
    conn.execute("UPDATE fetch_logs SET finished_at=?, status=?, items=?, new_items=? WHERE id=?",
                 (now_iso(), "ok" if ok else "error", ok, ok, log_id))
    conn.commit()
    log(f"  gdelt-timelines: {ok}/{len(jobs)} series")
    return summary


def seed_countries(conn: sqlite3.Connection) -> None:
    with tx(conn):
        for c in COUNTRIES.values():
            conn.execute("INSERT OR IGNORE INTO countries(iso2, iso3, name, region) VALUES (?,?,?,?)", (c.iso2, c.iso3, c.name, c.region))
