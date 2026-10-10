"""Polite, bounded HTTP fetching.

- network timeouts on connect and read
- response size cap (stops reading past ``max_bytes``)
- only http/https, at most 3 redirects, no redirect to private addresses
- conditional requests (ETag / Last-Modified) so unchanged feeds cost little
- retries with exponential backoff on 429 / 5xx / connection errors,
  honouring Retry-After
- a minimum interval between requests to the same host
"""
from __future__ import annotations

import ipaddress
import socket
import threading
import time
from dataclasses import dataclass
from urllib.parse import urljoin, urlparse

import requests

from . import __version__

USER_AGENT = f"WorldPulse/{__version__} (+https://github.com/vks-nagpur/improvement-wing; open-source public-data monitor)"

_host_lock = threading.Lock()
_host_last: dict[str, float] = {}
HOST_MIN_INTERVAL = {"api.gdeltproject.org": 8.0, "news.google.com": 1.5}
DEFAULT_MIN_INTERVAL = 0.5


class FetchError(Exception):
    def __init__(self, msg: str, status: int | None = None):
        super().__init__(msg)
        self.status = status


@dataclass
class Response:
    url: str
    status: int
    content: bytes
    headers: dict[str, str]
    not_modified: bool = False

    @property
    def text(self) -> str:
        return self.content.decode("utf-8", errors="replace")


def _is_public_host(host: str) -> bool:
    try:
        infos = socket.getaddrinfo(host, None)
    except socket.gaierror:
        # Behind an egress proxy DNS may not resolve locally; the proxy decides.
        return True
    for info in infos:
        ip = ipaddress.ip_address(info[4][0])
        if ip.is_private or ip.is_loopback or ip.is_link_local or ip.is_reserved or ip.is_multicast:
            return False
    return True


def check_url(url: str) -> str:
    p = urlparse(url)
    if p.scheme not in ("http", "https") or not p.hostname:
        raise FetchError(f"refused URL scheme or host: {url[:200]}")
    if not _is_public_host(p.hostname):
        raise FetchError(f"refused non-public host: {p.hostname}")
    return url


def _wait_for_host(host: str) -> None:
    gap = HOST_MIN_INTERVAL.get(host, DEFAULT_MIN_INTERVAL)
    with _host_lock:
        now = time.monotonic()
        wait = _host_last.get(host, 0) + gap - now
        _host_last[host] = max(now, _host_last.get(host, 0) + gap)
    if wait > 0:
        time.sleep(wait)


def fetch(
    url: str,
    *,
    etag: str | None = None,
    last_modified: str | None = None,
    timeout: tuple[float, float] = (10, 30),
    max_bytes: int = 15_000_000,
    total_timeout: float = 60,
    retries: int = 3,
    session: requests.Session | None = None,
) -> Response:
    sess = session or requests.Session()
    headers = {"User-Agent": USER_AGENT, "Accept-Encoding": "gzip, deflate"}
    if etag:
        headers["If-None-Match"] = etag
    if last_modified:
        headers["If-Modified-Since"] = last_modified
    attempt = 0
    while True:
        attempt += 1
        current = check_url(url)
        try:
            for _hop in range(4):
                _wait_for_host(urlparse(current).hostname or "")
                r = sess.get(current, headers=headers, timeout=timeout, stream=True, allow_redirects=False)
                if r.status_code in (301, 302, 303, 307, 308) and "location" in r.headers:
                    current = check_url(urljoin(current, r.headers["location"]))
                    r.close()
                    continue
                break
            else:
                raise FetchError("too many redirects")
            if r.status_code == 304:
                return Response(current, 304, b"", dict(r.headers), not_modified=True)
            if r.status_code == 429 or r.status_code >= 500:
                ra = r.headers.get("retry-after", "")
                r.close()
                if attempt > retries:
                    raise FetchError(f"HTTP {r.status_code} after {retries} retries", r.status_code)
                delay = float(ra) if ra.isdigit() else 2 ** attempt
                time.sleep(min(delay, 20))
                continue
            if r.status_code >= 400:
                r.close()
                raise FetchError(f"HTTP {r.status_code}", r.status_code)
            buf = bytearray()
            t0 = time.monotonic()
            for chunk in r.iter_content(65536):
                buf += chunk
                if time.monotonic() - t0 > total_timeout:
                    r.close()
                    raise FetchError(f"download took longer than {total_timeout:.0f}s")
                if len(buf) > max_bytes:
                    r.close()
                    raise FetchError(f"response larger than {max_bytes} bytes")
            return Response(current, r.status_code, bytes(buf), {k.lower(): v for k, v in r.headers.items()})
        except (requests.ConnectionError, requests.Timeout) as e:
            if attempt > retries:
                raise FetchError(f"network error: {type(e).__name__}: {str(e)[:200]}") from e
            time.sleep(2 ** attempt)
