"""SQLite storage with versioned schema migrations.

Raw collected records (articles, observations, indicators, fetch logs) are kept
apart from derived results (clusters, events, relationships, signals,
briefings) so derived tables can be rebuilt without re-fetching anything.
"""
from __future__ import annotations

import json
import sqlite3
from contextlib import contextmanager
from pathlib import Path
from typing import Any, Iterator

MIGRATIONS: list[str] = [
    # 1 — base schema
    """
    CREATE TABLE sources (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        kind TEXT NOT NULL,              -- rss | gdelt | usgs | worldbank | market | fx | gdacs | ioda | firms
        url TEXT NOT NULL,
        category TEXT,
        country TEXT,
        language TEXT,
        source_type TEXT,                -- publisher | aggregator | government | institution | scientific | market
        interval_minutes INTEGER NOT NULL DEFAULT 30,
        enabled INTEGER NOT NULL DEFAULT 1,
        etag TEXT,
        last_modified TEXT,
        last_attempt TEXT,
        last_success TEXT,
        last_error TEXT,
        consecutive_failures INTEGER NOT NULL DEFAULT 0
    );

    -- RAW: articles as collected (metadata + short excerpt + link only)
    CREATE TABLE articles (
        id TEXT PRIMARY KEY,             -- sha1 of canonical url
        source_id TEXT NOT NULL REFERENCES sources(id),
        url TEXT NOT NULL,
        canonical_url TEXT NOT NULL,
        title TEXT NOT NULL,
        norm_title TEXT NOT NULL,
        publisher TEXT,
        excerpt TEXT,
        language TEXT,
        published_at TEXT,
        collected_at TEXT NOT NULL,
        category TEXT,
        category_scores TEXT,            -- json
        countries TEXT,                  -- json {iso2: confidence}
        cluster_id TEXT,
        duplicate_of TEXT,
        duplicate_method TEXT,           -- exact_url | canonical_url | headline | similarity
        is_official INTEGER NOT NULL DEFAULT 0,
        processing_version TEXT NOT NULL
    );
    CREATE UNIQUE INDEX ix_articles_canon ON articles(canonical_url);
    CREATE INDEX ix_articles_pub ON articles(published_at);
    CREATE INDEX ix_articles_norm ON articles(norm_title);
    CREATE INDEX ix_articles_cluster ON articles(cluster_id);

    CREATE VIRTUAL TABLE articles_fts USING fts5(title, excerpt, publisher, content='articles', content_rowid='rowid');
    CREATE TRIGGER articles_ai AFTER INSERT ON articles BEGIN
        INSERT INTO articles_fts(rowid, title, excerpt, publisher) VALUES (new.rowid, new.title, new.excerpt, new.publisher);
    END;
    CREATE TRIGGER articles_ad AFTER DELETE ON articles BEGIN
        INSERT INTO articles_fts(articles_fts, rowid, title, excerpt, publisher) VALUES('delete', old.rowid, old.title, old.excerpt, old.publisher);
    END;

    -- DERIVED: story clusters
    CREATE TABLE article_clusters (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        first_seen TEXT NOT NULL,
        last_seen TEXT NOT NULL,
        article_count INTEGER NOT NULL,
        publisher_count INTEGER NOT NULL,
        category TEXT,
        countries TEXT
    );

    -- DERIVED: events (from clusters or from scientific / official datasets)
    CREATE TABLE events (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        category TEXT NOT NULL,
        description TEXT,
        country TEXT,
        related_countries TEXT,          -- json list
        lat REAL, lon REAL,
        started_at TEXT,
        first_detected TEXT NOT NULL,
        last_updated TEXT NOT NULL,
        origin TEXT NOT NULL,            -- news_cluster | usgs | gdacs | ioda
        cluster_id TEXT,
        source_refs TEXT NOT NULL,       -- json list of {title,url,publisher,published_at}
        datasets TEXT,                   -- json list of dataset ids
        verification TEXT NOT NULL,      -- observed | official | corroborated | reported | single_source
        extraction_confidence REAL NOT NULL,
        magnitude REAL,
        revision INTEGER NOT NULL DEFAULT 1
    );
    CREATE INDEX ix_events_time ON events(last_updated);
    CREATE INDEX ix_events_country ON events(country);
    CREATE INDEX ix_events_cat ON events(category);

    CREATE TABLE event_revisions (
        event_id TEXT NOT NULL,
        revision INTEGER NOT NULL,
        changed_at TEXT NOT NULL,
        snapshot TEXT NOT NULL,
        PRIMARY KEY (event_id, revision)
    );

    CREATE TABLE countries (
        iso2 TEXT PRIMARY KEY,
        iso3 TEXT NOT NULL,
        name TEXT NOT NULL,
        region TEXT,
        wb_region TEXT,
        income_level TEXT,
        capital TEXT,
        lat REAL, lon REAL,
        meta_source TEXT,
        meta_updated TEXT
    );

    -- RAW: indicator values, one row per country/indicator/period
    CREATE TABLE country_indicators (
        iso2 TEXT NOT NULL,
        indicator TEXT NOT NULL,
        label TEXT NOT NULL,
        value REAL,
        unit TEXT,
        period TEXT NOT NULL,
        source TEXT NOT NULL,
        source_url TEXT NOT NULL,
        retrieved_at TEXT NOT NULL,
        PRIMARY KEY (iso2, indicator, period)
    );

    CREATE TABLE event_relationships (
        source_event TEXT NOT NULL,
        target_event TEXT NOT NULL,
        rel_type TEXT NOT NULL,          -- same_event | follow_up | shared_location | shared_organization | shared_country | reported_consequence | possible_context
        evidence TEXT NOT NULL,
        method TEXT NOT NULL,
        confidence REAL NOT NULL,
        observed INTEGER NOT NULL,       -- 1 = observed link, 0 = hypothesis
        PRIMARY KEY (source_event, target_event, rel_type)
    );

    CREATE TABLE signals (
        id TEXT PRIMARY KEY,
        signal_type TEXT NOT NULL,       -- news_volume | topic_volume | earthquake | quake_cluster | market | fx | disaster_alert | internet
        category TEXT NOT NULL,
        country TEXT,
        region TEXT,
        title TEXT NOT NULL,
        detected_at TEXT NOT NULL,
        observed REAL,
        baseline REAL,
        baseline_std REAL,
        deviation REAL,
        pct_change REAL,
        score REAL,
        unit TEXT,
        evidence TEXT NOT NULL,          -- json list
        data_freshness TEXT,
        confidence TEXT NOT NULL,        -- high | medium | low
        limitations TEXT NOT NULL,
        status TEXT NOT NULL             -- statistical_anomaly | observed_measurement | official_alert | measurement_alert
    );
    CREATE INDEX ix_signals_time ON signals(detected_at);

    -- RAW: time series observations (news counts, prices, quake counts...)
    CREATE TABLE signal_observations (
        series TEXT NOT NULL,
        ts TEXT NOT NULL,
        value REAL NOT NULL,
        source TEXT NOT NULL,
        retrieved_at TEXT NOT NULL,
        PRIMARY KEY (series, ts)
    );

    CREATE TABLE quakes (
        id TEXT PRIMARY KEY,
        time TEXT NOT NULL,
        mag REAL,
        place TEXT,
        lat REAL, lon REAL, depth REAL,
        country TEXT,
        tsunami INTEGER,
        alert TEXT,
        felt INTEGER,
        url TEXT,
        updated TEXT,
        retrieved_at TEXT NOT NULL
    );
    CREATE INDEX ix_quakes_time ON quakes(time);

    CREATE TABLE briefings (
        id TEXT PRIMARY KEY,
        kind TEXT NOT NULL,              -- global | country | market | geopolitical | signals
        scope TEXT,
        period_hours INTEGER NOT NULL,
        generated_at TEXT NOT NULL,
        mode TEXT NOT NULL,              -- deterministic | ollama
        body TEXT NOT NULL               -- json {sections:[{heading, items:[{text, refs:[...]}]}], references:[...], limitations:[...]}
    );

    CREATE TABLE alert_rules (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        rule_type TEXT NOT NULL,         -- threshold | pct_change | keyword | geographic | category | anomaly
        params TEXT NOT NULL,            -- json
        enabled INTEGER NOT NULL DEFAULT 1,
        created_at TEXT NOT NULL
    );

    CREATE TABLE alert_history (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        rule_id INTEGER NOT NULL,
        dedupe_key TEXT NOT NULL,
        triggered_at TEXT NOT NULL,
        message TEXT NOT NULL,
        evidence TEXT NOT NULL,
        seen INTEGER NOT NULL DEFAULT 0,
        UNIQUE (rule_id, dedupe_key)
    );

    CREATE TABLE fetch_logs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        source_id TEXT NOT NULL,
        started_at TEXT NOT NULL,
        finished_at TEXT,
        status TEXT NOT NULL,            -- ok | not_modified | error | skipped
        http_status INTEGER,
        items INTEGER,
        new_items INTEGER,
        error TEXT
    );
    CREATE INDEX ix_fetch_logs_source ON fetch_logs(source_id, started_at);

    CREATE TABLE processing_errors (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        stage TEXT NOT NULL,
        ref TEXT,
        at TEXT NOT NULL,
        error TEXT NOT NULL
    );
    """,
]


def connect(path: str | Path) -> sqlite3.Connection:
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(str(path), timeout=30)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")
    migrate(conn)
    return conn


def migrate(conn: sqlite3.Connection) -> int:
    version = conn.execute("PRAGMA user_version").fetchone()[0]
    for i, sql in enumerate(MIGRATIONS[version:], start=version + 1):
        conn.executescript(sql)
        conn.execute(f"PRAGMA user_version={i}")
        conn.commit()
    return conn.execute("PRAGMA user_version").fetchone()[0]


@contextmanager
def tx(conn: sqlite3.Connection) -> Iterator[sqlite3.Connection]:
    try:
        yield conn
        conn.commit()
    except Exception:
        conn.rollback()
        raise


def j(v: Any) -> str:
    return json.dumps(v, ensure_ascii=False, separators=(",", ":"))


def uj(v: str | None, default: Any = None) -> Any:
    if not v:
        return default
    try:
        return json.loads(v)
    except ValueError:
        return default


def log_error(conn: sqlite3.Connection, stage: str, ref: str | None, err: str, at: str) -> None:
    conn.execute("INSERT INTO processing_errors(stage, ref, at, error) VALUES (?,?,?,?)", (stage, ref, at, err[:2000]))
