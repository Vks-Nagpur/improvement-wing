# WorldPulse

Free, open-source global intelligence built from public data. It answers three questions:

| System | Question | Built from |
|---|---|---|
| **World Monitor** | What is happening around the world? | Google News RSS, publisher RSS, GDELT, UN / WHO / ReliefWeb / CISA feeds |
| **Country Intelligence** | What is happening in a country? | Country-tagged developments, World Bank indicators, signals, sourced briefings |
| **OpenSignals** | What unusual changes are emerging? | GDELT coverage volume, USGS earthquakes, GDACS alerts, market & FX series, IODA internet measurements |

Nothing is simulated. If a source fails or data is missing, the site says so.

## Two ways to run it

### 1. Public dashboard (no server)
The `WorldPulse data` GitHub workflow (`.github/workflows/worldpulse.yml`) runs every 3 hours on GitHub's runners:
fetch → analyse → write briefings → export compact JSON → force-push one snapshot to the `worldpulse-data` branch
(latest snapshot only, so the repository does not grow). The SQLite database travels with the snapshot so baselines build up over time.

To publish the website on GitHub Pages:
1. Settings → Pages → Source: **GitHub Actions**.
2. Settings → Secrets and variables → Actions → Variables: add `WORLDPULSE_PAGES` = `true`.
3. Merge to the default branch. The next run builds `frontend/` with the latest data and deploys it.

Scheduled runs are best-effort: GitHub can delay or skip them. The site shows a warning when data is older than 12 hours.

### 2. Self-hosted engine (your computer)
```bash
cd worldpulse/backend
pip install -r requirements.txt
python -m worldpulse run --force          # first full collection (5–10 min; GDELT is rate-limited)
python -m worldpulse serve                # API on http://127.0.0.1:8000 + scheduler every 15 min
python -m worldpulse ask "What happened in India this week?"

cd ../frontend && npm install && npm run dev   # site on http://127.0.0.1:5173 reading frontend/public/data
# optional: VITE_API_URL=http://127.0.0.1:8000 npm run dev  (Ask also queries the local API)
```
Self-hosted adds: per-source schedules (15 min for news), 45 days of history, custom sources (edit `backend/worldpulse/sources.json`),
alert rules (`POST /api/alerts/rules`), and optional local AI summaries: install [Ollama](https://ollama.com), `ollama pull llama3.1:8b`,
then `python -m worldpulse run --ai`. AI text is kept only if every sentence cites an existing source and every number appears in the evidence.

The API listens on 127.0.0.1 only. Endpoints: `/api/news`, `/api/events`, `/api/events/{id}`, `/api/countries`, `/api/countries/{code}`,
`/api/countries/{code}/news`, `/api/countries/{code}/indicators`, `/api/signals`, `/api/signals/{id}`, `/api/briefings`, `/api/search`,
`POST /api/query`, `/api/sources`, `/api/health`, `/api/alerts/rules`, `/api/alerts/history`. Lists take `limit` (≤200) and `offset`.

NASA FIRMS fire detections need a free MAP_KEY: set `FIRMS_MAP_KEY` (or the repository secret) and enable `nasa-firms` in `sources.json`.

## Pipeline
`sources.json` → fetch (timeouts, retries with backoff, per-host rate limits, ETag/Last-Modified, size caps, public hosts only)
→ normalize (dates, canonical links, headlines, language) → validate → de-duplicate (exact URL, canonical URL, headline, similarity)
→ classify (keyword rules, scores kept) → tag countries (gazetteer, ambiguous names marked) → story clusters → events
→ signals → relationships → briefings → JSON export. Each stage is a function in `backend/worldpulse/` and is tested offline in `backend/tests/`.
A failing source is logged in `fetch_logs` and never stops the run; re-running never duplicates records.

## Trust rules
- Verification levels: **Measured** (instrument), **Official alert**, **Official source**, **Several outlets**, **One outlet (unverified)**.
- Signals are labelled *statistical anomaly*, *measured*, *official alert* or *unconfirmed measurement*, each with baseline, deviation, evidence, data date and limitations.
- Event links are labelled *observed* or *hypothesis*; time proximity is never presented as cause.
- Annual World Bank figures always show their year; market data is labelled delayed.
- Only headlines, short excerpts and links are stored. No paywall or access-control bypassing.
- Feed text is stripped of markup and rendered as text; only http(s) links are rendered.

## Known limitations
English-language, RSS-heavy coverage; topic classification is keyword-based; country tagging can miss or mis-tag ambiguous names;
Yahoo's chart endpoint is unofficial and may stop working; news-volume signals from WorldPulse's own feeds need 7+ days of collection.
