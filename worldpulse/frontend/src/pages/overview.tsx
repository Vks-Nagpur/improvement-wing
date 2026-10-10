import { useMemo, useState } from 'react'
import { BriefingView, Chips, ErrorBox, EventItem, Loading, PageHead, SignalItem, Stat } from '../components/common'
import { DotMap, StackedBars } from '../components/charts'
import type { Article, Briefing, Event, History, Meta, Signal } from '../data'
import { useData } from '../data'
import { ago, cat, CAT, compact, Ext, flag, go, hoursSince } from '../util'

export function Overview() {
  const meta = useData<Meta>('meta.json')
  const ev = useData<{ events: Event[] }>('events.json')
  const sig = useData<{ signals: Signal[] }>('signals.json')
  const hist = useData<History>('history.json')
  const br = useData<{ briefings: Briefing[] }>('briefings.json')
  const q = useData<{ quakes: unknown[][] }>('quakes.json')
  if (meta.error) return <ErrorBox error={meta.error} />
  if (!meta.data) return <Loading rows={6} />
  const m = meta.data
  const events = ev.data?.events || []
  const top = events.filter((e) => e.verification !== 'single_source_report').sort((a, b) => b.outlets - a.outlets || b.last_updated.localeCompare(a.last_updated)).slice(0, 8)
  const recentSig = (sig.data?.signals || []).filter((s) => hoursSince(s.detected_at) < 72).slice(0, 6)
  const global = br.data?.briefings.find((b) => b.kind === 'global' && b.period_hours === 24)
  const quakes = (q.data?.quakes || []).filter((r) => (r[2] as number) >= 4.5 && hoursSince(r[1] as string) < 168)

  return (
    <>
      <div className="hero">
        <svg className="pulse" width="320" height="320" viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="none" stroke="white" strokeWidth="1.5" /><circle cx="50" cy="50" r="30" fill="none" stroke="white" strokeWidth="1" /><path d="M6 52h20l7-18 12 34 8-22 6 6h35" stroke="white" strokeWidth="2.5" fill="none" /></svg>
        <h2>What is happening in the world</h2>
        <p>Live picture built only from public sources: news feeds, USGS seismometers, disaster alerts, World Bank and market data. Every item links to where it came from.</p>
        <div className="row" style={{ marginTop: 14 }}>
          <span className="badge">🕒 Updated {ago(m.generated_at)}</span>
          <span className="badge">📡 {m.sources.ok}/{m.sources.total} sources working</span>
          <span className="badge">🧹 {compact(m.counts.duplicates_removed_72h)} duplicates removed</span>
        </div>
      </div>

      <div className="grid g4" style={{ marginBottom: 16 }}>
        <Stat label="News stories (72 h)" value={compact(m.counts.articles_72h)} note="unique articles after de-duplication" glow="var(--c1)" onClick={() => go('/monitor')} />
        <Stat label="Developments (7 days)" value={compact(m.counts.events_7d)} note="grouped stories, quakes & alerts" glow="var(--c2)" onClick={() => go('/monitor')} />
        <Stat label="Unusual changes (30 d)" value={compact(m.counts.signals_30d)} note="OpenSignals detections" glow="var(--c4)" onClick={() => go('/signals')} />
        <Stat label="Earthquakes (30 d)" value={compact(m.counts.quakes_30d)} note="USGS catalogue" glow="var(--c3)" onClick={() => go('/quakes')} />
      </div>

      <div className="grid g2" style={{ marginBottom: 16 }}>
        <div className="card">
          <div className="card-head"><div><h3>🔥 Top developments</h3><div className="sub">Reported by several outlets or confirmed by instruments/officials</div></div><a href="#/monitor" className="small">See all →</a></div>
          {ev.loading ? <Loading /> : top.length ? <div className="list">{top.map((e) => <EventItem key={e.id} e={e} />)}</div> : <div className="empty">No multi-source developments in the latest data.</div>}
        </div>
        <div className="card">
          <div className="card-head"><div><h3>⚡ Unusual changes</h3><div className="sub">Detected in the last 72 hours</div></div><a href="#/signals" className="small">See all →</a></div>
          {sig.loading ? <Loading /> : recentSig.length ? <div className="list">{recentSig.map((s) => <SignalItem key={s.id} s={s} />)}</div> :
            <div className="empty">Nothing crossed its threshold. Signals need a history of normal values first, so this fills in as data accumulates.</div>}
        </div>
      </div>

      <div className="grid g2" style={{ marginBottom: 16 }}>
        <div className="card">
          <h3>🗂️ What the news is about</h3>
          <div className="sub">Distinct stories per day by topic (WorldPulse feeds)</div>
          {hist.data ? <TopicBars h={hist.data} /> : <Loading />}
        </div>
        <div className="card">
          <h3>🌍 Where things are happening</h3>
          <div className="sub">Earthquakes M4.5+ (last 7 days, sized by magnitude) and located developments</div>
          <DotMap points={[
            ...quakes.map((r) => ({ lat: r[4] as number, lon: r[5] as number, r: Math.max(2, ((r[2] as number) - 4) * 3), color: '#f97316', label: `M${r[2]} — ${r[3]}` })),
            ...events.filter((e) => e.lat != null && e.origin === 'gdacs').map((e) => ({ lat: e.lat!, lon: e.lon!, r: 6, color: 'var(--c7)', label: e.title, href: `/event/${e.id}` })),
          ]} />
        </div>
      </div>

      {global && (
        <div className="card">
          <div className="card-head"><div><h3>📋 Daily global briefing</h3><div className="sub">Every line links to its sources</div></div><a href="#/briefings" className="small">All briefings →</a></div>
          <BriefingView b={global} />
        </div>
      )}
    </>
  )
}

function TopicBars({ h }: { h: History }) {
  const days = [...new Set(h.category_per_day.map((r) => r[0]))].sort().slice(-14)
  const cats = Object.keys(CAT).filter((c) => c !== 'general')
  const stacks = cats.map((c) => ({ name: cat(c).label, color: cat(c).color, values: days.map((d) => h.category_per_day.find((r) => r[0] === d && r[1] === c)?.[2] || 0) }))
    .filter((s) => s.values.some(Boolean))
  return <StackedBars days={days} stacks={stacks} />
}

export function Monitor() {
  const ev = useData<{ events: Event[] }>('events.json')
  const news = useData<{ articles: Article[]; duplicates_removed: number; duplicate_methods: Record<string, number> }>('news.json')
  const [tab, setTab] = useState<'events' | 'news'>('events')
  const [c, setC] = useState('all')
  const [v, setV] = useState('all')
  const [q, setQ] = useState('')
  const events = useMemo(() => (ev.data?.events || []).filter((e) => (c === 'all' || e.category === c) && (v === 'all' || e.verification === v) &&
    (!q || e.title.toLowerCase().includes(q.toLowerCase()))), [ev.data, c, v, q])
  const arts = useMemo(() => (news.data?.articles || []).filter((a) => (c === 'all' || a.c === c) && (!q || a.t.toLowerCase().includes(q.toLowerCase()))), [news.data, c, q])
  const catOpts = [{ v: 'all', label: 'All topics' }, ...Object.entries(CAT).map(([k, x]) => ({ v: k, label: `${x.icon} ${x.label}` }))]
  return (
    <>
      <PageHead title="World Monitor" sub="Everything collected, grouped into developments. Similar headlines are grouped as related coverage — that does not mean the reports agree." />
      <div className="tabs">
        <button className={tab === 'events' ? 'on' : ''} onClick={() => setTab('events')}>Developments ({ev.data?.events.length ?? '…'})</button>
        <button className={tab === 'news' ? 'on' : ''} onClick={() => setTab('news')}>All headlines ({news.data?.articles.length ?? '…'})</button>
      </div>
      <div className="row" style={{ marginBottom: 12 }}>
        <input className="input" placeholder="Filter by words…" value={q} onChange={(e) => setQ(e.target.value)} style={{ minWidth: 240 }} />
        {tab === 'events' && (
          <select className="input" value={v} onChange={(e) => setV(e.target.value)}>
            <option value="all">Any verification</option>
            <option value="instrument_observation">Measured by instruments</option>
            <option value="official_alert">Official alerts</option>
            <option value="official_statement">Official sources</option>
            <option value="multi_source_reporting">Several outlets</option>
            <option value="single_source_report">One outlet (unverified)</option>
          </select>
        )}
      </div>
      <div style={{ marginBottom: 14 }}><Chips options={catOpts} value={c} onChange={setC} /></div>
      {tab === 'events' ? (
        ev.error ? <ErrorBox error={ev.error} /> : ev.loading ? <Loading /> :
          <div className="card"><div className="list">{events.slice(0, 300).map((e) => <EventItem key={e.id} e={e} />)}</div>
            {!events.length && <div className="empty">No developments match these filters.</div>}
            {events.length > 300 && <div className="small faint">Showing 300 of {events.length}. Narrow the filters to see more.</div>}</div>
      ) : news.error ? <ErrorBox error={news.error} /> : news.loading ? <Loading /> : (
        <div className="card">
          <div className="small muted" style={{ marginBottom: 8 }}>
            {news.data!.duplicates_removed} duplicate copies removed in 72 h ({Object.entries(news.data!.duplicate_methods).map(([k, n]) => `${n} by ${k.replace('_', ' ')}`).join(', ') || 'none'}). Original links are kept; WorldPulse stores headlines and short excerpts only.
          </div>
          <div className="list">
            {arts.slice(0, 400).map((a) => (
              <div className="item" key={a.id}>
                <div className="cat-bar" style={{ background: cat(a.c).color }} />
                <div style={{ minWidth: 0 }}>
                  <div className="t"><Ext href={a.u}>{a.t}</Ext></div>
                  {a.x && <div className="small muted" style={{ marginTop: 2 }}>{a.x}</div>}
                  <div className="m"><span>{a.p}</span><span>{ago(a.d)}</span><span style={{ color: cat(a.c).color }}>{cat(a.c).label}</span>
                    {a.k.map((k) => <a key={k} href={`#/country/${k}`} className="muted">{flag(k)} {k}</a>)}
                    {a.n > 1 && <span>· {a.n} related reports</span>}{a.o && <span className="badge b-info">official</span>}</div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </>
  )
}
