import { useEffect, useMemo, useState } from 'react'
import { BriefingView, Chips, ErrorBox, EventItem, Loading, PageHead, VerifBadge, CatBadge } from '../components/common'
import type { Article, Briefing, CountrySummary, Event, Market, Signal, Source } from '../data'
import { API, apiQuery, load, useData } from '../data'
import { ago, Badge, cat, CAT, Ext, flag, fmtDate } from '../util'

// ---------------------------------------------------------------- Briefings
export function Briefings() {
  const { data, error, loading } = useData<{ briefings: Briefing[] }>('briefings.json')
  const [k, setK] = useState('global-24')
  if (error) return <ErrorBox error={error} />
  if (loading) return <Loading />
  const opts = data!.briefings.map((b) => ({ v: `${b.kind}-${b.period_hours}`, label: ({ global: '🌍 Global', geopolitical: '🏛️ Geopolitical', market: '💹 Markets', signals: '⚡ Emerging signals' } as Record<string, string>)[b.kind] + (b.period_hours === 168 ? ' (week)' : b.period_hours === 72 ? ' (3 days)' : ' (24 h)') }))
  const b = data!.briefings.find((x) => `${x.kind}-${x.period_hours}` === k) || data!.briefings[0]
  return (
    <>
      <PageHead title="Intelligence briefings" sub="Written in two steps: first the facts and their sources are pulled from stored data, then sentences are written only from those facts. Every line is numbered to its source. Country briefings are on each country page." />
      <div style={{ marginBottom: 14 }}><Chips options={opts} value={k} onChange={setK} /></div>
      <div className="card">{b ? <BriefingView b={b} /> : <div className="empty">No briefing.</div>}</div>
    </>
  )
}

// ---------------------------------------------------------------- Ask (static-mode engine)
const REGIONS: Record<string, string[]> = {
  'middle east': ['IL', 'PS', 'LB', 'SY', 'JO', 'IQ', 'IR', 'SA', 'YE', 'OM', 'AE', 'QA', 'BH', 'KW', 'EG', 'TR'],
  europe: ['GB', 'FR', 'DE', 'IT', 'ES', 'PL', 'NL', 'BE', 'SE', 'NO', 'FI', 'DK', 'AT', 'CH', 'IE', 'PT', 'GR', 'CZ', 'HU', 'RO', 'BG', 'UA', 'BY', 'MD'],
  africa: [], 'latin america': [], 'south asia': ['IN', 'PK', 'BD', 'LK', 'NP', 'AF', 'BT', 'MV'], 'east asia': ['CN', 'JP', 'KR', 'KP', 'TW', 'MN'],
  'southeast asia': ['ID', 'MY', 'SG', 'TH', 'VN', 'PH', 'MM', 'KH', 'LA'], 'north america': ['US', 'CA', 'MX'], gulf: ['SA', 'AE', 'QA', 'BH', 'KW', 'OM'],
}
const CAT_WORDS: Record<string, string[]> = {
  economics: ['economy', 'economic', 'inflation', 'gdp', 'recession'], financial_markets: ['market', 'markets', 'stocks', 'currency', 'crypto'],
  energy: ['oil', 'gas', 'energy', 'opec', 'crude'], trade: ['trade', 'tariff', 'tariffs', 'exports', 'shipping'],
  natural_disasters: ['earthquake', 'earthquakes', 'quake', 'flood', 'floods', 'cyclone', 'disaster', 'disasters', 'wildfire'],
  military_security: ['war', 'military', 'conflict', 'attack', 'security', 'missile'], diplomacy: ['diplomacy', 'diplomatic', 'talks', 'summit', 'relations'],
  elections: ['election', 'elections', 'vote'], cybersecurity: ['cyber', 'hack', 'ransomware'], public_health: ['health', 'outbreak', 'disease'],
  environment: ['climate', 'environment', 'pollution'], technology: ['technology', 'tech', 'ai', 'chips'], geopolitics: ['sanctions', 'geopolitical', 'protest', 'protests', 'coup'],
}
const STOP = new Set('what which who when where how why is are was were did do does happened happening show me tell list give the a an in on of for about during last past this that today yesterday week month day days hours recent latest major significant important any all and or with involving between affecting summarize summarise developments news events countries country experienced'.split(' '))

type Answer = { summary: string; items: { e: Event; refs: Event['refs'] }[]; gaps: string[]; interp: { hours: number; countries: string[]; categories: string[]; keywords: string[] } }

function answerStatic(q: string, events: Event[], countries: CountrySummary[]): Answer {
  const ql = q.toLowerCase()
  let hours = 168
  const m = ql.match(/(?:last|past)\s+(\d+)\s*(hour|day|week|month)/)
  if (m) hours = Number(m[1]) * ({ h: 1, d: 24, w: 168, m: 720 } as Record<string, number>)[m[2][0]]
  else if (/\btoday\b|24 hours/.test(ql)) hours = 24
  else if (/yesterday/.test(ql)) hours = 48
  else if (/month/.test(ql)) hours = 720
  const cs = new Set<string>()
  for (const c of countries) if (new RegExp(`\\b${c.name.toLowerCase().replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\b`).test(ql)) cs.add(c.iso2)
  for (const [r, list] of Object.entries(REGIONS)) if (ql.includes(r)) {
    const l = list.length ? list : countries.filter((c) => (r === 'africa' ? c.region === 'Sub-Saharan Africa' : c.region.startsWith('Latin'))).map((c) => c.iso2)
    l.forEach((x) => cs.add(x))
  }
  const cats = Object.entries(CAT_WORDS).filter(([, ws]) => ws.some((w) => new RegExp(`\\b${w}\\b`).test(ql))).map(([k]) => k)
  const names = new Set([...cs].flatMap((c) => (countries.find((x) => x.iso2 === c)?.name.toLowerCase().split(' ') || [])))
  const kws = (q.match(/[a-zA-Z][a-zA-Z-]{2,}/g) || []).filter((w) => !STOP.has(w.toLowerCase()) && !names.has(w.toLowerCase()) && !Object.values(CAT_WORDS).flat().includes(w.toLowerCase()) && !Object.keys(REGIONS).some((r) => r.includes(w.toLowerCase())))
  const since = Date.now() - hours * 3.6e6
  let hits = events.filter((e) => new Date(e.last_updated).getTime() >= since)
  if (cs.size) hits = hits.filter((e) => (e.country && cs.has(e.country)) || e.countries.some((c) => cs.has(c)))
  if (cs.size >= 2 && cs.size <= 4) { const both = hits.filter((e) => [...cs].every((c) => e.countries.includes(c))); if (both.length) hits = both }
  if (cats.length) hits = hits.filter((e) => cats.includes(e.category))
  if (/significant/.test(ql) && cats.includes('natural_disasters')) hits = hits.filter((e) => e.origin !== 'usgs' || (e.magnitude || 0) >= 5.5)
  if (kws.length && !cs.size && !cats.length) hits = hits.filter((e) => kws.some((k) => e.title.toLowerCase().includes(k.toLowerCase())))
  // a named hazard narrows to that hazard ("earthquakes" should not return fires)
  const HAZ: [RegExp, RegExp][] = [[/earthquake|quake|tremor/, /earthquake|quake|tremor|seismic/i], [/flood/, /flood/i], [/cyclone|hurricane|typhoon|storm/, /cyclone|hurricane|typhoon|storm/i],
    [/wildfire|forest fire|bushfire/, /fire/i], [/volcan|eruption/, /volcan|eruption/i], [/tsunami/, /tsunami/i], [/drought/, /drought/i]]
  const haz = HAZ.filter(([q]) => q.test(ql)).map(([, t]) => t)
  if (haz.length) hits = hits.filter((e) => haz.some((t) => t.test(e.title)))
  const bonus: Record<string, number> = { instrument_observation: 3, official_alert: 2.5, official_statement: 2, multi_source_reporting: 1.5, single_source_report: 1 }
  const score = (e: Event) => (1 + kws.filter((k) => e.title.toLowerCase().includes(k.toLowerCase())).length * 2) * (bonus[e.verification] || 1) * (1 + e.outlets / 6) * Math.exp(-(Date.now() - new Date(e.last_updated).getTime()) / 3.6e6 / Math.max(hours, 24))
  hits.sort((a, b) => score(b) - score(a))
  const scope = cs.size ? [...cs].slice(0, 5).map((c) => countries.find((x) => x.iso2 === c)?.name).join(', ') + (cs.size > 5 ? '…' : '') : 'worldwide'
  const period = hours >= 48 ? `the last ${Math.round(hours / 24)} days` : `the last ${hours} hours`
  const topic = cats.map((c) => cat(c).label.toLowerCase()).join(', ') || 'all topics'
  const gaps: string[] = []
  if (!hits.length) gaps.push('Nothing in the published data matched. Try a longer period or fewer words. The public dataset holds 7 days of developments.')
  if (hours > 168) gaps.push('The public dataset only keeps 7 days of developments; the self-hosted engine keeps 45 days.')
  if (hits.length && hits.slice(0, 12).every((e) => e.verification === 'single_source_report')) gaps.push('All matches are single-outlet reports and remain unverified.')
  if (cats.includes('energy') || cats.includes('financial_markets')) gaps.push('For prices see Markets. News next to a price move does not show that one caused the other.')
  return { summary: hits.length ? `Found ${hits.length} recorded developments (${scope}; ${topic}; ${period}). The most relevant are listed with their sources.` : `No recorded developments matched (${scope}; ${topic}; ${period}).`,
    items: hits.slice(0, 12).map((e) => ({ e, refs: e.refs.slice(0, 2) })), gaps, interp: { hours, countries: [...cs], categories: cats, keywords: kws } }
}

const EXAMPLES = ['What happened in the Middle East during the last 24 hours?', 'Show major economic developments in India this week.',
  'Which countries experienced significant earthquakes today?', 'What major events are affecting global oil markets?', 'Summarize recent developments involving China and Taiwan.']

export function Ask() {
  const ev = useData<{ events: Event[] }>('events.json')
  const cs = useData<{ countries: CountrySummary[] }>('countries.json')
  const [q, setQ] = useState('')
  const [a, setA] = useState<Answer | null>(null)
  const [apiAns, setApiAns] = useState<{ summary: string; gaps: string[] } | null>(null)
  const run = async (text: string) => {
    setQ(text)
    if (!ev.data || !cs.data) return
    setA(answerStatic(text, ev.data.events, cs.data.countries))
    if (API) { try { setApiAns(await apiQuery(text)) } catch { setApiAns(null) } }
  }
  return (
    <>
      <PageHead title="Ask WorldPulse" sub="Ask in plain English. Answers are built only from stored records and always show their sources. No AI model is needed; nothing is made up." />
      <div className="card" style={{ marginBottom: 16 }}>
        <div className="row"><input className="input" style={{ flex: 1, fontSize: 16, padding: '13px 16px' }} placeholder="e.g. What happened in Ukraine this week?" value={q} onChange={(e) => setQ(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && q.trim().length > 2 && run(q)} />
          <button className="btn" disabled={!ev.data} onClick={() => q.trim().length > 2 && run(q)}>Ask</button></div>
        <div className="chips" style={{ marginTop: 12 }}>{EXAMPLES.map((x) => <button key={x} className="chip" onClick={() => run(x)}>{x}</button>)}</div>
      </div>
      {ev.error && <ErrorBox error={ev.error} />}
      {a && (
        <div className="card">
          <h3>Answer</h3>
          <p>{a.summary}</p>
          <div className="row small muted" style={{ marginBottom: 8 }}>
            <span>Understood as →</span><Badge tone="b-acc">⏱ last {a.interp.hours >= 48 ? `${Math.round(a.interp.hours / 24)} days` : `${a.interp.hours} h`}</Badge>
            {a.interp.countries.slice(0, 8).map((c) => <Badge key={c}>{flag(c)} {c}</Badge>)}{a.interp.countries.length > 8 && <Badge>+{a.interp.countries.length - 8}</Badge>}
            {a.interp.categories.map((c) => <Badge key={c}>{cat(c).icon} {cat(c).label}</Badge>)}{a.interp.keywords.map((k) => <Badge key={k}>“{k}”</Badge>)}
          </div>
          <div className="list">{a.items.map(({ e }) => <EventItem key={e.id} e={e} />)}</div>
          {a.gaps.length > 0 && <div className="banner warn" style={{ marginTop: 12 }}><span>ℹ️</span><div className="small"><b>What is missing or uncertain</b><ul style={{ margin: '4px 0 0', paddingLeft: 18 }}>{a.gaps.map((g) => <li key={g}>{g}</li>)}</ul></div></div>}
          {apiAns && <div className="small faint">Self-hosted engine: {apiAns.summary}</div>}
        </div>
      )}
    </>
  )
}

// ---------------------------------------------------------------- Search
type Hit = { type: string; title: string; desc?: string | null; date?: string | null; source?: string | null; countries: string[]; category?: string; url?: string | null; internal?: string; verification?: string }

export function Search({ initial }: { initial: string }) {
  const [q, setQ] = useState(initial)
  const [f, setF] = useState({ type: 'all', category: 'all', country: '', since: '', sort: 'relevance', verification: 'all' })
  const [all, setAll] = useState<Hit[] | null>(null)
  useEffect(() => setQ(initial), [initial])
  useEffect(() => {
    Promise.all([load<{ events: Event[] }>('events.json'), load<{ articles: Article[] }>('news.json'), load<{ countries: CountrySummary[] }>('countries.json'),
      load<{ series: Market[] }>('markets.json'), load<{ signals: Signal[] }>('signals.json')]).then(([ev, n, c, m, s]) => {
      setAll([
        ...c.countries.map((x) => ({ type: 'country', title: x.name, desc: `${x.region}${x.capital ? ' · capital ' + x.capital : ''}`, countries: [x.iso2], internal: `/country/${x.iso2}`, source: 'World Bank / ISO 3166' })),
        ...ev.events.map((e) => ({ type: 'event', title: e.title, desc: e.description, date: e.started_at || e.first_detected, source: e.refs[0]?.p, countries: e.countries, category: e.category, internal: `/event/${e.id}`, url: e.refs[0]?.u, verification: e.verification })),
        ...n.articles.map((a) => ({ type: 'headline', title: a.t, desc: a.x, date: a.d, source: a.p, countries: a.k, category: a.c, url: a.u })),
        ...m.series.map((x) => ({ type: 'indicator', title: x.name, desc: `${x.unit} · latest ${x.points.at(-1)?.[0] || 'n/a'}`, source: x.source, countries: x.country ? [x.country] : [], internal: '/markets', date: x.points.at(-1)?.[0] })),
        ...s.signals.map((x) => ({ type: 'signal', title: x.title, desc: x.limitations, date: x.detected_at, source: 'OpenSignals', countries: x.country ? [x.country] : [], internal: `/signal/${x.id}` })),
      ])
    }, () => setAll([]))
  }, [])
  const results = useMemo(() => {
    if (!all || q.trim().length < 2) return []
    const terms = q.toLowerCase().split(/\s+/).filter((t) => t.length > 1)
    const since = f.since ? Date.now() - Number(f.since) * 3.6e6 : 0
    const r = all.map((h) => {
      const text = `${h.title} ${h.desc || ''} ${h.source || ''} ${h.countries.join(' ')}`.toLowerCase()
      const s = terms.reduce((a, t) => a + (h.title.toLowerCase().includes(t) ? 3 : text.includes(t) ? 1 : -100), 0) + (h.type === 'country' ? 5 : 0)
      return { h, s }
    }).filter(({ h, s }) => s > 0 && (f.type === 'all' || h.type === f.type) && (f.category === 'all' || h.category === f.category) &&
      (!f.country || h.countries.includes(f.country.toUpperCase())) && (!since || !h.date || new Date(h.date).getTime() >= since) && (f.verification === 'all' || h.verification === f.verification))
    r.sort((a, b) => f.sort === 'date' ? (b.h.date || '').localeCompare(a.h.date || '') : b.s - a.s)
    return r.slice(0, 200).map((x) => x.h)
  }, [all, q, f])
  const set = (k: keyof typeof f) => (e: { target: { value: string } }) => setF({ ...f, [k]: e.target.value })
  return (
    <>
      <PageHead title="Search" sub="Countries, developments, headlines, market indicators and signals in the published data." />
      <div className="card" style={{ marginBottom: 16 }}>
        <input className="input" style={{ width: '100%', fontSize: 16, padding: '12px 14px' }} autoFocus placeholder="Search anything…" value={q} onChange={(e) => setQ(e.target.value)} />
        <div className="row" style={{ marginTop: 10 }}>
          <select className="input" value={f.type} onChange={set('type')}><option value="all">All types</option><option value="country">Countries</option><option value="event">Developments</option><option value="headline">Headlines</option><option value="indicator">Indicators</option><option value="signal">Signals</option></select>
          <select className="input" value={f.category} onChange={set('category')}><option value="all">Any topic</option>{Object.entries(CAT).map(([k, x]) => <option key={k} value={k}>{x.label}</option>)}</select>
          <input className="input" placeholder="Country code (e.g. IN)" value={f.country} onChange={set('country')} style={{ width: 170 }} maxLength={2} />
          <select className="input" value={f.since} onChange={set('since')}><option value="">Any date</option><option value="24">Last 24 h</option><option value="72">Last 3 days</option><option value="168">Last 7 days</option></select>
          <select className="input" value={f.verification} onChange={set('verification')}><option value="all">Any verification</option><option value="instrument_observation">Measured</option><option value="official_alert">Official alert</option><option value="official_statement">Official source</option><option value="multi_source_reporting">Several outlets</option><option value="single_source_report">One outlet</option></select>
          <Chips options={[{ v: 'relevance', label: 'Relevance' }, { v: 'date', label: 'Newest' }]} value={f.sort} onChange={(v) => setF({ ...f, sort: v })} />
        </div>
      </div>
      {!all ? <Loading /> : q.trim().length < 2 ? <div className="empty">Type at least two letters.</div> : (
        <div className="card"><div className="small muted">{results.length}{results.length === 200 ? '+' : ''} results</div>
          <div className="list">{results.map((h, i) => (
            <div key={i} className="item"><div style={{ minWidth: 0 }}>
              <div className="t">{h.internal ? <a href={`#${h.internal}`}>{h.title}</a> : <Ext href={h.url}>{h.title}</Ext>}</div>
              {h.desc && <div className="small muted">{h.desc}</div>}
              <div className="m"><Badge tone="b-acc">{h.type}</Badge>{h.verification && <VerifBadge v={h.verification} />}{h.category && <CatBadge c={h.category} />}
                {h.countries.slice(0, 4).map((c) => <span key={c}>{flag(c)} {c}</span>)}{h.source && <span>{h.source}</span>}{h.date && <span>{fmtDate(h.date)}</span>}
                {h.url && h.internal && <Ext href={h.url}>original ↗</Ext>}</div></div></div>))}</div>
          {!results.length && <div className="empty">No results.</div>}</div>
      )}
    </>
  )
}

// ---------------------------------------------------------------- Alerts (browser)
type Rule = { id: string; name: string; type: 'quake' | 'keyword' | 'country' | 'category' | 'market'; value: string; threshold?: number }
type Fired = { key: string; rule: string; text: string; link: string; at: string }
const LS = 'worldpulse.alerts.v1'

function readStore(): { rules: Rule[]; fired: Fired[] } {
  try { return JSON.parse(localStorage.getItem(LS) || '') } catch { return { rules: [], fired: [] } }
}
function writeStore(s: { rules: Rule[]; fired: Fired[] }) { try { localStorage.setItem(LS, JSON.stringify(s)) } catch { /* storage unavailable */ } }

export async function evaluateAlerts(notify = true): Promise<Fired[]> {
  const st = readStore()
  if (!st.rules?.length) return []
  const [ev, sig, mk] = await Promise.all([load<{ events: Event[] }>('events.json'), load<{ signals: Signal[] }>('signals.json'), load<{ series: Market[] }>('markets.json')])
  const known = new Set(st.fired.map((f) => f.key))
  const fresh: Fired[] = []
  const add = (r: Rule, key: string, text: string, link: string) => { const k = `${r.id}:${key}`; if (!known.has(k)) { known.add(k); fresh.push({ key: k, rule: r.name, text, link, at: new Date().toISOString() }) } }
  for (const r of st.rules) {
    if (r.type === 'quake') ev.events.filter((e) => e.origin === 'usgs' && (e.magnitude || 0) >= (r.threshold || 6)).forEach((e) => add(r, e.id, e.title, `/event/${e.id}`))
    if (r.type === 'keyword') ev.events.filter((e) => e.title.toLowerCase().includes(r.value.toLowerCase())).forEach((e) => add(r, e.id, e.title, `/event/${e.id}`))
    if (r.type === 'category') ev.events.filter((e) => e.category === r.value && e.outlets >= 2).forEach((e) => add(r, e.id, e.title, `/event/${e.id}`))
    if (r.type === 'country') sig.signals.filter((s) => s.country === r.value.toUpperCase()).forEach((s) => add(r, s.id, s.title, `/signal/${s.id}`))
    if (r.type === 'market') {
      const m = mk.series.find((x) => x.id === r.value)
      const p = m?.points
      if (m && p && p.length > 1) { const c = (p[p.length - 1][1] / p[p.length - 2][1] - 1) * 100; if (Math.abs(c) >= (r.threshold || 3)) add(r, p[p.length - 1][0], `${m.name} moved ${c.toFixed(2)}% on ${p[p.length - 1][0]}`, '/markets') }
    }
  }
  if (fresh.length) {
    writeStore({ rules: st.rules, fired: [...fresh, ...st.fired].slice(0, 300) })
    if (notify && 'Notification' in window && Notification.permission === 'granted') fresh.slice(0, 3).forEach((f) => new Notification(`WorldPulse: ${f.rule}`, { body: f.text }))
  }
  return fresh
}

export function Alerts() {
  const [st, setSt] = useState(readStore())
  const mk = useData<{ series: Market[] }>('markets.json')
  const [form, setForm] = useState<Rule>({ id: '', name: '', type: 'quake', value: '', threshold: 6 })
  const save = (s: typeof st) => { writeStore(s); setSt(s) }
  useEffect(() => { evaluateAlerts(false).then(() => setSt(readStore())) }, [])
  const addRule = () => {
    const r = { ...form, id: Math.random().toString(36).slice(2, 9), name: form.name || describe(form) }
    save({ ...st, rules: [...(st.rules || []), r] })
    evaluateAlerts().then(() => setSt(readStore()))
  }
  const describe = (r: Rule) => ({ quake: `Earthquakes ≥ M${r.threshold}`, keyword: `Mentions “${r.value}”`, country: `Unusual activity: ${r.value.toUpperCase()}`, category: `${cat(r.value).label} (2+ outlets)`, market: `${r.value} moves ≥ ${r.threshold}%` }[r.type])
  return (
    <>
      <PageHead title="Alerts" sub="Your own monitoring rules. They are checked each time you open WorldPulse and when new data is published." />
      <div className="banner warn"><span>🔔</span><div className="small">This public site has no server, so it cannot alert you while it is closed. Rules are saved in this browser only. For continuous alerts, run the self-hosted engine (see Sources &amp; method).</div></div>
      <div className="grid g2">
        <div className="card"><h3>New rule</h3>
          <div className="row" style={{ marginTop: 10 }}>
            <select className="input" value={form.type} onChange={(e) => setForm({ ...form, type: e.target.value as Rule['type'], value: '' })}>
              <option value="quake">Earthquake above magnitude</option><option value="keyword">Keyword in a development</option><option value="country">Unusual activity in a country</option>
              <option value="category">Topic (multi-outlet)</option><option value="market">Market move above %</option></select>
            {form.type === 'quake' && <input className="input" type="number" step="0.1" min="2.5" max="9.5" value={form.threshold} onChange={(e) => setForm({ ...form, threshold: Number(e.target.value) })} style={{ width: 100 }} />}
            {form.type === 'keyword' && <input className="input" placeholder="e.g. sanctions" value={form.value} onChange={(e) => setForm({ ...form, value: e.target.value })} />}
            {form.type === 'country' && <input className="input" placeholder="Country code, e.g. IN" maxLength={2} value={form.value} onChange={(e) => setForm({ ...form, value: e.target.value })} style={{ width: 160 }} />}
            {form.type === 'category' && <select className="input" value={form.value} onChange={(e) => setForm({ ...form, value: e.target.value })}><option value="">Choose…</option>{Object.entries(CAT).map(([k, x]) => <option key={k} value={k}>{x.label}</option>)}</select>}
            {form.type === 'market' && <><select className="input" value={form.value} onChange={(e) => setForm({ ...form, value: e.target.value })}><option value="">Choose…</option>{mk.data?.series.map((m) => <option key={m.id} value={m.id}>{m.name}</option>)}</select>
              <input className="input" type="number" step="0.5" min="0.5" value={form.threshold} onChange={(e) => setForm({ ...form, threshold: Number(e.target.value) })} style={{ width: 90 }} /></>}
            <button className="btn" disabled={form.type !== 'quake' && !form.value} onClick={addRule}>Add</button>
          </div>
          {'Notification' in window && Notification.permission !== 'granted' && <button className="btn ghost" style={{ marginTop: 12 }} onClick={() => Notification.requestPermission().then(() => setSt(readStore()))}>Enable browser notifications</button>}
          <h3 style={{ marginTop: 18 }}>Your rules</h3>
          <div className="list">{(st.rules || []).map((r) => (
            <div key={r.id} className="item" style={{ alignItems: 'center' }}><div style={{ flex: 1 }}><b>{r.name}</b></div>
              <button className="btn ghost" onClick={() => save({ ...st, rules: st.rules.filter((x) => x.id !== r.id) })}>Remove</button></div>))}</div>
          {!st.rules?.length && <div className="empty">No rules yet.</div>}
        </div>
        <div className="card"><div className="row spread"><h3>Alert history</h3>{st.fired?.length > 0 && <button className="btn ghost" onClick={() => save({ ...st, fired: [] })}>Clear</button>}</div>
          <div className="list">{(st.fired || []).map((f) => (
            <div key={f.key} className="item"><div><div className="t"><a href={`#${f.link}`}>{f.text}</a></div><div className="m"><Badge tone="b-acc">{f.rule}</Badge><span>{ago(f.at)}</span></div></div></div>))}</div>
          {!st.fired?.length && <div className="empty">Nothing has matched your rules yet.</div>}
        </div>
      </div>
    </>
  )
}

// ---------------------------------------------------------------- Sources & method
export function Sources() {
  const { data, error, loading } = useData<{ sources: Source[]; generated_at: string }>('sources.json')
  if (error) return <ErrorBox error={error} />
  if (loading) return <Loading />
  const tone = (s: string) => ({ ok: 'b-ok', failing: 'b-bad', disabled: 'b-mute', never_run: 'b-warn' } as Record<string, string>)[s] || 'b-mute'
  return (
    <>
      <PageHead title="Sources & method" sub="Where every number comes from, whether each source worked on the latest run, and how WorldPulse processes data." />
      <div className="card" style={{ marginBottom: 16 }}>
        <h3>Source health</h3><div className="sub">Latest run {fmtDate(data!.generated_at)}. A failing source never stops the others.</div>
        <div className="table-wrap"><table><thead><tr><th>Source</th><th>Type</th><th>Status</th><th>Last success</th><th className="num">Items</th><th>Last error</th></tr></thead>
          <tbody>{data!.sources.map((s) => (
            <tr key={s.id}><td><b>{s.name}</b><div className="small faint"><Ext href={s.url.startsWith('http') && !s.url.includes(' ') ? s.url : undefined}>{s.kind}</Ext> · every {s.interval_minutes >= 1440 ? `${Math.round(s.interval_minutes / 1440)} d` : `${s.interval_minutes} min`} (self-hosted)</div></td>
              <td>{s.type}</td><td><Badge tone={tone(s.status)}><span className="dot" />{s.status.replace('_', ' ')}</Badge></td><td>{ago(s.last_success)}</td>
              <td className="num">{s.last_items ?? '—'}</td><td className="small" style={{ color: 'var(--bad)', maxWidth: 320 }}>{s.last_error}</td></tr>))}</tbody></table></div>
      </div>
      <div className="grid g2">
        <div className="card"><h3>How it works</h3>
          <ol className="muted" style={{ paddingLeft: 18 }}>
            <li><b>Fetch</b> public feeds politely (timeouts, retries with backoff, per-site rate limits, caching headers).</li>
            <li><b>Normalize</b> dates, links (tracking removed), headlines, language.</li>
            <li><b>De-duplicate</b> by exact link, canonical link, identical headline, then near-identical headline.</li>
            <li><b>Classify</b> topics with transparent keyword rules and <b>tag countries</b> by name, capital and demonym.</li>
            <li><b>Group</b> related headlines into developments and add instrument data (USGS) and official alerts (GDACS).</li>
            <li><b>Detect signals</b>: compare today with the recent normal (mean and standard deviation).</li>
            <li><b>Link</b> developments, labelling each link as observed or hypothesis — never as cause.</li>
            <li><b>Write briefings</b> from extracted facts only, with numbered sources.</li>
            <li><b>Publish</b> compact JSON with bounded retention for this site.</li>
          </ol></div>
        <div className="card"><h3>What WorldPulse does not do</h3>
          <ul className="muted" style={{ paddingLeft: 18 }}>
            <li>It does not bypass paywalls or store full articles — only headlines, short excerpts and links.</li>
            <li>It does not score outlets for “truth”. It shows measurable facts: official or not, how many outlets, corrections.</li>
            <li>It does not predict events. Signals describe unusual data, with their limits.</li>
            <li>It does not invent anything: if evidence is missing, it says so.</li>
            <li>Market data is delayed and comes from free endpoints.</li>
          </ul>
          <h3 style={{ marginTop: 14 }}>Run your own engine</h3>
          <p className="muted small">The self-hosted engine fetches more often, keeps 45 days of history, supports custom sources and alerts, and can add local AI summaries with Ollama. See <code>worldpulse/README.md</code> in the repository.</p></div>
      </div>
    </>
  )
}
