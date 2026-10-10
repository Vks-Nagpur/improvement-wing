"""Normalization: URLs, headlines, dates, text and language."""
from __future__ import annotations

import hashlib
import html
import re
import unicodedata
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
from urllib.parse import parse_qsl, urlencode, urlparse, urlunparse

TRACKING_PARAMS = re.compile(r"^(utm_|fbclid|gclid|mc_|ocid|cmpid|ref$|ref_|src$|smid|at_|guccounter|ito$|CMP$)", re.I)
TAG_RE = re.compile(r"<[^>]+>")
WS_RE = re.compile(r"\s+")


def now_iso() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def to_iso(dt: datetime) -> str:
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt.astimezone(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def parse_iso(s: str | None) -> datetime | None:
    if not s:
        return None
    try:
        return datetime.fromisoformat(s.replace("Z", "+00:00"))
    except ValueError:
        return None


def parse_date(value: str | None) -> str | None:
    """Parse RFC 822, ISO 8601 or GDELT (20260101T120000Z) dates to UTC ISO.

    Dates more than one day in the future are rejected as invalid.
    """
    if not value:
        return None
    v = value.strip()
    dt: datetime | None = None
    try:
        dt = parsedate_to_datetime(v)
    except (TypeError, ValueError, IndexError):
        pass
    if dt is None:
        m = re.fullmatch(r"(\d{4})(\d{2})(\d{2})T?(\d{2})(\d{2})(\d{2})Z?", v)
        if m:
            dt = datetime(*map(int, m.groups()), tzinfo=timezone.utc)
    if dt is None:
        try:
            dt = datetime.fromisoformat(v.replace("Z", "+00:00"))
        except ValueError:
            return None
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    if (dt - datetime.now(timezone.utc)).total_seconds() > 86400:
        return None
    return to_iso(dt)


def canonical_url(url: str) -> str:
    """Lower-case scheme/host, drop fragments, tracking parameters, 'www.',
    default ports and trailing slashes; sort remaining query parameters."""
    p = urlparse(url.strip())
    host = (p.hostname or "").lower()
    if host.startswith("www."):
        host = host[4:]
    if p.port and p.port not in (80, 443):
        host = f"{host}:{p.port}"
    q = sorted((k, v) for k, v in parse_qsl(p.query, keep_blank_values=False) if not TRACKING_PARAMS.match(k))
    path = re.sub(r"/+$", "", p.path) or "/"
    if host.startswith("m.") or host.startswith("amp."):
        host = host.split(".", 1)[1]
    path = re.sub(r"/amp$", "", path)
    return urlunparse(("https", host, path, "", urlencode(q), ""))


def url_id(canonical: str) -> str:
    return hashlib.sha1(canonical.encode()).hexdigest()[:16]


def clean_text(value: str | None, limit: int = 400) -> str:
    """Strip markup and control characters from untrusted feed text.

    The result is plain text; renderers must still escape it.
    """
    if not value:
        return ""
    t = html.unescape(TAG_RE.sub(" ", value))
    t = "".join(ch for ch in t if unicodedata.category(ch)[0] != "C" or ch in "\n\t")
    t = WS_RE.sub(" ", t).strip()
    if len(t) > limit:
        t = t[: limit - 1].rsplit(" ", 1)[0] + "…"
    return t


def split_google_title(title: str) -> tuple[str, str | None]:
    """Google News titles end with ' - Publisher'."""
    if " - " in title:
        head, tail = title.rsplit(" - ", 1)
        if 0 < len(tail) <= 60:
            return head.strip(), tail.strip()
    return title.strip(), None


def normalize_title(title: str) -> str:
    t = unicodedata.normalize("NFKD", title).encode("ascii", "ignore").decode().lower()
    t = re.sub(r"\b(live|update[sd]?|breaking|watch|video|opinion|analysis)\b[:\s-]*", " ", t)
    t = re.sub(r"[^a-z0-9 ]+", " ", t)
    return WS_RE.sub(" ", t).strip()


_EN = set("the of and to in a is for on with that by from at as are was has have will be its new says after over".split())
_FR = set("le la les des du de et en un une pour dans sur est au aux par".split())
_ES = set("el la los las de del y en un una por para con que es se al".split())
_DE = set("der die das und ist nicht mit den von zu im auf für ein eine des".split())
_PT = set("o a os as de do da dos das e em um uma para com que por no na".split())


def detect_language(text: str, declared: str | None = None) -> str | None:
    """Declared feed language wins; otherwise a small stop-word vote for
    Latin-script languages and a script check for others. Returns None when
    unsure."""
    if declared:
        return declared.split("-")[0].lower()[:5]
    if not text:
        return None
    if re.search(r"[؀-ۿ]", text):
        return "ar"
    if re.search(r"[Ѐ-ӿ]", text):
        return "ru"
    if re.search(r"[一-鿿]", text):
        return "zh"
    if re.search(r"[ऀ-ॿ]", text):
        return "hi"
    words = set(re.findall(r"[a-zà-ÿ]+", text.lower()))
    scores = {k: len(words & s) for k, s in (("en", _EN), ("fr", _FR), ("es", _ES), ("de", _DE), ("pt", _PT))}
    best = max(scores, key=scores.get)
    return best if scores[best] >= 2 else None
