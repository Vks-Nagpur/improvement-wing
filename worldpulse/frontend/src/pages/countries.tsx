import { useMemo, useState } from 'react'
import { BriefingView, Chips, ErrorBox, EventItem, Loading, PageHead, SignalItem, Stat } from '../components/common'
import { LineChart } from '../components/charts'
import type { CountryProfile, CountrySummary } from '../data'
import { useData } from '../data'
import { ago, cat, compact, Ext, flag, fmtDate, go, num } from '../util'

export function Countries() {
  const { data, error, loading } = useData<{ countries: CountrySummary[] }>('countries.json')
  const [q, setQ] = useState('')
  const [region, setRegion] = useState('all')
  const [sort, setSort] = useState<'activity' | 'name' | 'population'>('activity')
  const list = useMemo(() => {
    let l = (data?.countries || []).filter((c) => (region === 'all' || c.region === region) &&
      (!q || c.name.toLowerCase().includes(q.toLowerCase()) || c.iso2 === q.toUpperCase() || c.iso3 === q.toUpperCase() || (c.capital || '').toLowerCase().includes(q.toLowerCase())))
    l = [...l].sort((a, b) => sort === 'name' ? a.name.localeCompare(b.name) : sort === 'population' ? (b.population?.value || 0) - (a.population?.value || 0) :
      (b.events_7d * 3 + b.news_72h + b.signals_30d * 5) - (a.events_7d * 3 + a.news_72h + a.signals_30d * 5))
    return l
  }, [data, q, region, sort])
  if (error) return <ErrorBox error={error} />
  if (loading) return <Loading />
  const regions = [...new Set(data!.countries.map((c) => c.region))].sort()
  const max = Math.max(1, ...list.map((c) => c.events_7d))
  return (
    <>
      <PageHead title="Country Intelligence" sub="An automatically updated profile for every country: recent developments, World Bank indicators, signals and a sourced briefing." />
      <div className="row" style={{ marginBottom: 14 }}>
        <input className="input" autoFocus placeholder="Search a country, code or capital…" value={q} onChange={(e) => setQ(e.target.value)} style={{ minWidth: 280, flex: 1 }} />
        <select className="input" value={region} onChange={(e) => setRegion(e.target.value)}><option value="all">All regions</option>{regions.map((r) => <option key={r}>{r}</option>)}</select>
        <Chips options={[{ v: 'activity', label: 'Most active' }, { v: 'name', label: 'A–Z' }, { v: 'population', label: 'Population' }]} value={sort} onChange={setSort} />
      </div>
      <div className="grid g3">
        {list.map((c) => (
          <div key={c.iso2} className="card" style={{ cursor: 'pointer' }} onClick={() => go(`/country/${c.iso2}`)}>
            <div className="row spread">
              <div className="row"><span className="flag">{flag(c.iso2)}</span><div><b>{c.name}</b><div className="small faint">{c.region}{c.capital ? ` · ${c.capital}` : ''}</div></div></div>
              {c.signals_30d > 0 && <span className="badge b-acc" title="Signals in the last 30 days">⚡ {c.signals_30d}</span>}
            </div>
            <div className="row small muted" style={{ marginTop: 10, gap: 14 }}>
              <span><b style={{ color: 'var(--text)' }}>{c.events_7d}</b> developments (7 d)</span>
              <span><b style={{ color: 'var(--text)' }}>{c.news_72h}</b> headlines (72 h)</span>
            </div>
            <div className="bar-bg" style={{ marginTop: 8 }}><div className="bar-fg" style={{ width: `${(c.events_7d / max) * 100}%` }} /></div>
            <div className="row small faint" style={{ marginTop: 8, justifyContent: 'space-between' }}>
              <span>{c.population ? `Pop. ${compact(c.population.value)} (${c.population.period})` : 'Population n/a'}</span>
              {c.top_category && <span style={{ color: cat(c.top_category).color }}>{cat(c.top_category).icon} {cat(c.top_category).label}</span>}
            </div>
          </div>
        ))}
      </div>
      {!list.length && <div className="empty">No country matches “{q}”.</div>}
    </>
  )
}

const KEY_INDICATORS = ['NY.GDP.MKTP.CD', 'NY.GDP.MKTP.KD.ZG', 'FP.CPI.TOTL.ZG', 'SL.UEM.TOTL.ZS', 'NE.RSB.GNFS.ZS', 'GC.DOD.TOTL.GD.ZS', 'PA.NUS.FCRF', 'SP.POP.TOTL']

function fmtInd(v: number, unit: string) {
  if (unit.startsWith('current US$')) return '$' + compact(v)
  if (unit === 'people') return compact(v)
  if (unit.startsWith('%')) return `${v.toFixed(1)}%`
  return num(v, 2)
}

export function Country({ code }: { code: string }) {
  const { data, error, loading } = useData<CountryProfile>(`countries/${code.toUpperCase()}.json`)
  const [period, setPeriod] = useState<'24' | '168' | '720'>('168')
  const [tab, setTab] = useState<'brief' | 'events' | 'news' | 'econ' | 'signals'>('brief')
  if (error) return <ErrorBox error={`No profile for ${code}: ${error}`} />
  if (loading || !data) return <Loading rows={8} />
  const c = data.country
  const ind = data.indicators
  const b = data.briefings[period]
  const hours = Number(period)
  const evs = data.events.filter((e) => (Date.now() - new Date(e.last_updated).getTime()) / 3.6e6 <= hours)
  return (
    <>
      <div className="hero" style={{ background: 'linear-gradient(120deg, #0f766e 0%, #4f46e5 60%, #7c3aed 100%)' }}>
        <div className="row"><span style={{ fontSize: 46 }}>{flag(c.iso2)}</span>
          <div><h2>{c.name}</h2><p style={{ margin: 0 }}>{[c.region, c.capital && `Capital: ${c.capital}`, c.income].filter(Boolean).join(' · ')}</p></div></div>
        <div className="row" style={{ marginTop: 14 }}>
          <span className="badge">ISO {c.iso2} / {c.iso3}</span>
          <span className="badge">Profile updated {ago(data.generated_at)}</span>
          {data.meta_source && <span className="badge">Facts: {data.meta_source}</span>}
        </div>
      </div>
      <div className="grid g4" style={{ marginBottom: 16 }}>
        <Stat label="Developments (7 days)" value={c.events_7d} glow="var(--c1)" />
        <Stat label="Headlines (72 h)" value={c.news_72h} glow="var(--c2)" />
        <Stat label="Signals (30 days)" value={c.signals_30d} glow="var(--c4)" />
        <Stat label="Population" value={c.population ? compact(c.population.value) : '—'} note={c.population ? `World Bank, ${c.population.period}` : 'not available'} glow="var(--c3)" />
      </div>
      <div className="tabs">
        {([['brief', '📋 Briefing'], ['events', '🗞️ Developments'], ['news', '📰 Headlines'], ['econ', '📊 Economy'], ['signals', '⚡ Signals']] as const).map(([k, l]) =>
          <button key={k} className={tab === k ? 'on' : ''} onClick={() => setTab(k)}>{l}</button>)}
      </div>
      {(tab === 'brief' || tab === 'events') && (
        <div style={{ marginBottom: 12 }}>
          <Chips options={[{ v: '24', label: 'Last 24 hours' }, { v: '168', label: 'Last 7 days' }, { v: '720', label: 'Last 30 days' }]} value={period} onChange={setPeriod} />
        </div>
      )}
      {tab === 'brief' && <div className="card">{b ? <BriefingView b={b} /> : <div className="empty">No developments or signals about {c.name} in the published window, so no briefing was generated. This reflects source coverage, not necessarily an absence of events.</div>}</div>}
      {tab === 'events' && <div className="card"><div className="list">{evs.map((e) => <EventItem key={e.id} e={e} />)}</div>{!evs.length && <div className="empty">None recorded in this period.</div>}
        {hours === 720 && <div className="small faint">The public dataset keeps 7 days of developments; the self-hosted engine keeps 45.</div>}</div>}
      {tab === 'news' && (
        <div className="card"><div className="list">{data.news.map((a) => (
          <div className="item" key={a.id}><div className="cat-bar" style={{ background: cat(a.c).color }} />
            <div><div className="t"><Ext href={a.u}>{a.t}</Ext></div><div className="m"><span>{a.p}</span><span>{ago(a.d)}</span><span style={{ color: cat(a.c).color }}>{cat(a.c).label}</span></div></div></div>))}
        </div>{!data.news.length && <div className="empty">No headlines mention {c.name} in the last 72 hours.</div>}</div>
      )}
      {tab === 'econ' && (
        <div className="grid g2">
          <div className="card">
            <h3>Economic indicators</h3>
            <div className="sub">Latest available annual values. These are not live measurements.</div>
            {Object.keys(ind).length ? (
              <table><thead><tr><th>Indicator</th><th className="num">Value</th><th>Year</th><th>Source</th></tr></thead>
                <tbody>{[...KEY_INDICATORS, ...Object.keys(ind).filter((k) => !KEY_INDICATORS.includes(k))].filter((k) => ind[k]).map((k) => {
                  const i = ind[k]
                  const old = new Date().getFullYear() - Number(i.period) >= 3
                  return <tr key={k}><td>{i.label}<div className="small faint">{i.unit}</div></td><td className="num"><b>{fmtInd(i.value, i.unit)}</b></td>
                    <td>{i.period}{old && <span className="badge b-warn" style={{ marginLeft: 6 }} title="3+ years old">old</span>}</td><td><Ext href={i.url}>{i.source}</Ext></td></tr>
                })}</tbody></table>
            ) : <div className="empty">World Bank returned no indicators for this country.</div>}
          </div>
          <div className="card">
            <h3>News coverage over time</h3>
            <div className="sub">Daily articles mentioning “{c.name}” in GDELT-monitored media (measures reporting, not events)</div>
            {data.volume.length ? <LineChart series={[{ name: 'Articles', color: 'var(--c1)', points: data.volume }]} /> :
              <div className="empty">Coverage history is collected for the watch-list countries only. Add this country to <code>volume_watchlist</code> in the source registry.</div>}
          </div>
        </div>
      )}
      {tab === 'signals' && <div className="card"><div className="list">{data.signals.map((s) => <SignalItem key={s.id} s={s} />)}</div>{!data.signals.length && <div className="empty">No signals for {c.name} in the last 30 days.</div>}</div>}
      <div className="small faint" style={{ marginTop: 12 }}>Country facts updated {fmtDate(data.meta_updated)}.</div>
    </>
  )
}
