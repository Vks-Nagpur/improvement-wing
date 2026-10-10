"""Adapters for structured public datasets.

Each function takes already-fetched bytes (so it can be tested offline) and
returns plain records. Fetching, logging and failure isolation live in the
pipeline.
"""
from __future__ import annotations

import csv
import io
import json
from datetime import datetime, timezone

from ..gazetteer import COUNTRIES, ISO3_TO_ISO2, find_countries
from ..processing.normalize import clean_text, to_iso


# ---------- USGS earthquakes (GeoJSON summary feeds) ----------

def parse_usgs(content: bytes) -> list[dict]:
    data = json.loads(content)
    out = []
    for f in data.get("features", [])[:20000]:
        p = f.get("properties") or {}
        g = (f.get("geometry") or {}).get("coordinates") or [None, None, None]
        if p.get("type") not in (None, "earthquake"):
            continue
        place = clean_text(p.get("place"), 200)
        out.append({
            "id": str(f.get("id"))[:40],
            "time": to_iso(datetime.fromtimestamp(p["time"] / 1000, tz=timezone.utc)) if p.get("time") else None,
            "updated": to_iso(datetime.fromtimestamp(p["updated"] / 1000, tz=timezone.utc)) if p.get("updated") else None,
            "mag": p.get("mag"),
            "place": place,
            "lon": g[0], "lat": g[1], "depth": g[2] if len(g) > 2 else None,
            "country": quake_country(place),
            "tsunami": int(p.get("tsunami") or 0),
            "alert": p.get("alert"),
            "felt": p.get("felt"),
            "url": p.get("url"),
            "status": p.get("status"),
        })
    return out


_US_STATES = {"Alaska", "California", "Hawaii", "Nevada", "Oklahoma", "Texas", "Washington", "Oregon", "Utah", "Idaho",
              "Montana", "Wyoming", "Kansas", "Puerto Rico", "New Mexico", "Arizona", "Tennessee", "Missouri", "Arkansas", "CA", "AK", "NV", "HI"}


def quake_country(place: str) -> str | None:
    """USGS 'place' strings end with a region: '10 km SW of Town, Chile'.
    Only the final segment is used, and only if it names a country (or a US
    state). Offshore regions ('Mid-Atlantic Ridge') return None."""
    if not place:
        return None
    tail = place.rsplit(",", 1)[-1].strip()
    if tail in _US_STATES:
        return "US"
    hits = find_countries(tail)
    clear = {k: v for k, v in hits.items() if v >= 0.9}
    return max(clear, key=clear.get) if clear else None


# ---------- GDELT DOC 2.0 ----------

def parse_gdelt_artlist(content: bytes) -> list[dict]:
    text = content.decode("utf-8", errors="replace").strip()
    if not text.startswith("{"):
        raise ValueError(f"GDELT returned non-JSON: {text[:120]}")
    data = json.loads(text)
    out = []
    for a in data.get("articles", [])[:500]:
        sc = a.get("sourcecountry") or ""
        out.append({
            "title": clean_text(a.get("title"), 300),
            "link": a.get("url"),
            "published": a.get("seendate"),
            "publisher": clean_text(a.get("domain"), 100),
            "language": (a.get("language") or "")[:20],
            "source_country": sc,
        })
    return [o for o in out if o["title"] and o["link"]]


def parse_gdelt_timeline(content: bytes) -> list[tuple[str, float]]:
    """Return [(YYYY-MM-DD, article_count)] summed per UTC day."""
    text = content.decode("utf-8", errors="replace").strip()
    if not text.startswith("{"):
        raise ValueError(f"GDELT returned non-JSON: {text[:120]}")
    data = json.loads(text)
    per_day: dict[str, float] = {}
    for series in data.get("timeline", []):
        for pt in series.get("data", []):
            d = str(pt.get("date", ""))[:8]
            if len(d) == 8 and d.isdigit():
                day = f"{d[:4]}-{d[4:6]}-{d[6:]}"
                per_day[day] = per_day.get(day, 0) + float(pt.get("value") or 0)
        break  # first series is the article count
    return sorted(per_day.items())


# ---------- World Bank ----------

WB_INDICATORS = {
    "NY.GDP.MKTP.CD": ("GDP", "current US$"),
    "NY.GDP.MKTP.KD.ZG": ("GDP growth", "% per year"),
    "NY.GDP.PCAP.CD": ("GDP per capita", "current US$"),
    "FP.CPI.TOTL.ZG": ("Inflation (consumer prices)", "% per year"),
    "SL.UEM.TOTL.ZS": ("Unemployment", "% of labour force (ILO modelled)"),
    "NE.RSB.GNFS.ZS": ("Trade balance (goods & services)", "% of GDP"),
    "NE.TRD.GNFS.ZS": ("Trade openness", "% of GDP"),
    "GC.DOD.TOTL.GD.ZS": ("Central government debt", "% of GDP"),
    "PA.NUS.FCRF": ("Official exchange rate", "local currency per US$, period average"),
    "BX.KLT.DINV.WD.GD.ZS": ("Foreign direct investment, net inflows", "% of GDP"),
    "EG.IMP.CONS.ZS": ("Net energy imports", "% of energy use"),
    "EG.ELC.ACCS.ZS": ("Access to electricity", "% of population"),
    "SP.POP.TOTL": ("Population", "people"),
}


def parse_wb_countries(content: bytes) -> list[dict]:
    data = json.loads(content)
    rows = data[1] if isinstance(data, list) and len(data) > 1 and data[1] else []
    out = []
    for r in rows:
        if (r.get("region") or {}).get("value") == "Aggregates":
            continue
        iso2 = r.get("iso2Code")
        if iso2 not in COUNTRIES:
            continue
        out.append({
            "iso2": iso2,
            "wb_region": (r.get("region") or {}).get("value"),
            "income_level": (r.get("incomeLevel") or {}).get("value"),
            "capital": r.get("capitalCity") or None,
            "lat": float(r["latitude"]) if r.get("latitude") else None,
            "lon": float(r["longitude"]) if r.get("longitude") else None,
        })
    return out


def parse_wb_indicator(content: bytes, indicator: str) -> list[dict]:
    data = json.loads(content)
    if not isinstance(data, list) or len(data) < 2 or not data[1]:
        msg = data[0].get("message") if isinstance(data, list) and data and isinstance(data[0], dict) else None
        raise ValueError(f"World Bank returned no rows for {indicator}: {msg}")
    out = []
    for r in data[1]:
        iso3 = r.get("countryiso3code")
        iso2 = ISO3_TO_ISO2.get(iso3)
        if not iso2 or r.get("value") is None:
            continue
        out.append({"iso2": iso2, "indicator": indicator, "value": float(r["value"]), "period": str(r.get("date"))})
    return out


# ---------- Markets ----------

def parse_yahoo_chart(content: bytes) -> dict:
    data = json.loads(content)
    res = (data.get("chart") or {}).get("result") or []
    if not res:
        err = (data.get("chart") or {}).get("error")
        raise ValueError(f"no chart result: {err}")
    r = res[0]
    meta = r.get("meta", {})
    ts = r.get("timestamp") or []
    closes = ((r.get("indicators") or {}).get("quote") or [{}])[0].get("close") or []
    points = []
    for t, c in zip(ts, closes):
        if c is None:
            continue
        points.append((to_iso(datetime.fromtimestamp(t, tz=timezone.utc))[:10], float(c)))
    dedup: dict[str, float] = {}
    for d, c in points:
        dedup[d] = c  # last value of the day wins
    return {
        "currency": meta.get("currency"),
        "exchange": meta.get("exchangeName"),
        "market_time": to_iso(datetime.fromtimestamp(meta["regularMarketTime"], tz=timezone.utc)) if meta.get("regularMarketTime") else None,
        "points": sorted(dedup.items()),
    }


def parse_frankfurter_series(content: bytes) -> dict[str, list[tuple[str, float]]]:
    data = json.loads(content)
    out: dict[str, list[tuple[str, float]]] = {}
    for day, rates in sorted((data.get("rates") or {}).items()):
        for cur, v in rates.items():
            out.setdefault(cur, []).append((day, float(v)))
    return out


def parse_coingecko_chart(content: bytes) -> list[tuple[str, float]]:
    data = json.loads(content)
    per_day: dict[str, float] = {}
    for t, v in data.get("prices", []):
        per_day[to_iso(datetime.fromtimestamp(t / 1000, tz=timezone.utc))[:10]] = float(v)
    return sorted(per_day.items())


# ---------- GDACS ----------

def gdacs_record(item: dict) -> dict:
    ex = item.get("extra", {})
    countries = []
    for name in (ex.get("country") or "").split(","):
        name = name.strip()
        if not name:
            continue
        hits = {k: v for k, v in find_countries(name).items() if v >= 0.9}
        countries += list(hits)
    iso3 = ex.get("iso3") or ""
    for code in iso3.split(","):
        if code.strip() in ISO3_TO_ISO2:
            countries.append(ISO3_TO_ISO2[code.strip()])
    lat = lon = None
    point = ex.get("point") or ""
    if point:
        try:
            lat, lon = (float(x) for x in point.split()[:2])
        except ValueError:
            pass
    return {
        "title": item["title"], "link": item["link"], "published": item.get("published"),
        "summary": item.get("summary"), "alert_level": (ex.get("alertlevel") or "").title() or None,
        "event_type": ex.get("eventtype"), "event_id": ex.get("eventid"), "severity": ex.get("severity"),
        "countries": sorted(set(countries)), "lat": lat, "lon": lon,
        "is_current": (ex.get("iscurrent") or "").lower() == "true",
    }


# ---------- IODA ----------

def parse_ioda_alerts(content: bytes) -> list[dict]:
    data = json.loads(content)
    rows = data.get("data") or []
    out = []
    for a in rows[:5000]:
        ent = a.get("entity") or {}
        if ent.get("type") != "country":
            continue
        code = (ent.get("code") or "").upper()
        if code not in COUNTRIES:
            continue
        out.append({
            "country": code,
            "datasource": a.get("datasource"),
            "time": to_iso(datetime.fromtimestamp(int(a["time"]), tz=timezone.utc)) if a.get("time") else None,
            "level": a.get("level"),
            "condition": a.get("condition"),
            "value": a.get("value"),
            "history_value": a.get("historyValue"),
        })
    return out


# ---------- NASA FIRMS ----------

def parse_firms_csv(content: bytes) -> list[dict]:
    rows = list(csv.DictReader(io.StringIO(content.decode("utf-8", errors="replace"))))
    return [{"lat": float(r["latitude"]), "lon": float(r["longitude"]), "date": r.get("acq_date"),
             "confidence": r.get("confidence"), "frp": r.get("frp")} for r in rows if r.get("latitude")]
