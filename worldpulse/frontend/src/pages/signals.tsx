import { useMemo, useState } from 'react'
import { Chips, ErrorBox, EventItem, Loading, measure, PageHead, SignalItem, sigIcon, Stat, VerifBadge, CatBadge } from '../components/common'
import { DotMap, LineChart, Sparkline, StackedBars } from '../components/charts'
import type { Event, History, Market, Signal } from '../data'
import { useData } from '../data'
import { ago, Badge, Ext, flag, fmtDate, hoursSince, num, pct, SIG_STATUS, VERIF } from '../util'

const TYPES: Record<string, string> = {
  all: 'All', news_volume: 'News coverage', topic_volume: 'Reporting topics', earthquake: 'Earthquakes', quake_cluster: 'Quake clusters',
  market: 'Markets', fx: 'Currencies', disaster_alert: 'Disaster alerts', internet: 'Internet',
}

export function Signals() {
  const { data, error, loading } = useData<{ signals: Signal[] }>('signals.json')
  const hist = useData<History>('history.json')
  const [t, setT] = useState('all')
  const list = useMemo(() => (data?.signals || []).filter((s) => t === 'all' || s.signal_type === t), [data, t])
  if (error) return <ErrorBox error={error} />
  if (loading) return <Loading />
  const counts = (data!.signals).reduce<Record<string, number>>((a, s) => ({ ...a, [s.signal_type]: (a[s.signal_type] || 0) + 1 }), {})
  return (
    <>
      <PageHead title="OpenSignals" sub="Unusual changes in observable data. A signal says “this number is far from its recent normal” — it is not a prediction, and more reporting does not always mean more events." />
      <div className="banner info"><span>🧭</span><div className="small"><b>How to read signals.</b> Each one shows the measured value, the normal range it is compared with, the supporting sources, how fresh the data is and its known limits.
        <b> Measured</b> and <b>Official alert</b> are observations. <b>Statistical anomaly</b> is a computed deviation. <b>Unconfirmed measurement</b> needs confirmation from an operator or authority.</div></div>
      <div style={{ marginBottom: 14 }}><Chips options={Object.entries(TYPES).filter(([k]) => k === 'all' || counts[k]).map(([v, l]) => ({ v, label: `${v !== 'all' ? sigIcon(v) + ' ' : ''}${l}${v !== 'all' ? ` (${counts[v]})` : ''}` }))} value={t} onChange={setT} /></div>
      <div className="grid g2">
        <div className="card"><h3>Detections</h3><div className="sub">Newest first, last 30 days</div>
          <div className="list">{list.map((s) => <SignalItem key={s.id} s={s} />)}</div>
          {!list.length && <div className="empty">No signals of this type. Baselines need history, so detections become possible as data accumulates (news coverage uses 30 days from GDELT immediately).</div>}</div>
        <div className="card"><h3>Signal history</h3><div className="sub">Detections per day by type</div>
          {hist.data ? <SignalHistory h={hist.data} /> : <Loading />}
          <h3 style={{ marginTop: 18 }}>Reporting on watched topics</h3><div className="sub">Daily article counts in GDELT-monitored media</div>
          {hist.data && Object.keys(hist.data.topic_volume).length ? (
            <div className="list">{Object.entries(hist.data.topic_volume).map(([k, pts]) => (
              <div key={k} className="item" style={{ alignItems: 'center' }}><div style={{ flex: 1 }}><b>{k.replace(/_/g, ' ')}</b><div className="small faint">latest {pts.at(-1)?.[0]}: {num(pts.at(-1)?.[1], 0)} articles</div></div>
                <Sparkline values={pts.map((p) => p[1])} width={160} /></div>))}</div>
          ) : <div className="empty">No topic volume collected yet.</div>}
        </div>
      </div>
    </>
  )
}

function SignalHistory({ h }: { h: History }) {
  const days = [...new Set(h.signals_per_day.map((r) => r[0]))].sort().slice(-30)
  const types = [...new Set(h.signals_per_day.map((r) => r[1]))]
  const colors = ['var(--c1)', 'var(--c2)', 'var(--c3)', 'var(--c4)', 'var(--c5)', 'var(--c6)', 'var(--c7)', 'var(--c8)']
  return <StackedBars days={days} stacks={types.map((t, i) => ({ name: TYPES[t] || t, color: colors[i % 8], values: days.map((d) => h.signals_per_day.find((r) => r[0] === d && r[1] === t)?.[2] || 0) }))} />
}

export function SignalDetail({ id }: { id: string }) {
  const { data, error } = useData<{ signals: Signal[] }>('signals.json')
  const hist = useData<History>('history.json')
  if (error) return <ErrorBox error={error} />
  if (!data) return <Loading />
  const s = data.signals.find((x) => x.id === id)
  if (!s) return <div className="empty">This signal is no longer in the published 30-day window.</div>
  const st = SIG_STATUS[s.status]
  const vol = s.signal_type === 'news_volume' && s.country ? hist.data?.country_volume[s.country] : undefined
  return (
    <>
      <PageHead title={`${sigIcon(s.signal_type)} ${s.title}`} sub={<span className="row"><Badge tone={st?.tone}>{st?.label}</Badge><span>{st?.tip}</span></span>} />
      <div className="grid g4" style={{ marginBottom: 16 }}>
        <Stat label="Observed" value={s.observed != null ? num(s.observed, 2) : '—'} note={s.unit || ''} glow="var(--c1)" />
        <Stat label="Normal (baseline)" value={s.baseline != null ? num(s.baseline, 2) : '—'} note={s.baseline_std != null ? `± ${num(s.baseline_std, 2)} typical variation` : 'no baseline for this type'} glow="var(--c2)" />
        <Stat label="Deviation" value={s.deviation != null ? `${s.deviation.toFixed(1)}σ` : '—'} note={s.pct_change != null ? pct(s.pct_change) : ''} glow="var(--c4)" />
        <Stat label="Confidence in detection" value={s.confidence} note={`data as of ${s.data_freshness || '—'}`} glow="var(--c3)" />
      </div>
      <div className="grid g2">
        <div className="card"><h3>Supporting evidence</h3><div className="sub">{measure(s)}</div>
          <div className="list">{s.evidence.map((e, i) => (
            <div key={i} className="item"><div><div className="t"><Ext href={e.url}>{e.title}</Ext></div>
              <div className="m">{e.type === 'dataset' ? <Badge tone="b-info">dataset</Badge> : <Badge>article</Badge>}{e.publisher && <span>{e.publisher}</span>}{e.published_at && <span>{fmtDate(e.published_at)}</span>}</div>
              {e.detail && <div className="small faint" style={{ marginTop: 4, wordBreak: 'break-word' }}>{e.detail}</div>}</div></div>))}</div></div>
        <div className="card"><h3>Known limitations</h3><p className="muted">{s.limitations}</p>
          <dl className="kv"><dt>Signal ID</dt><dd><code>{s.id}</code></dd><dt>Type</dt><dd>{TYPES[s.signal_type] || s.signal_type}</dd>
            <dt>Country / region</dt><dd>{s.country ? <a href={`#/country/${s.country}`}>{flag(s.country)} {s.country}</a> : s.region || '—'}</dd>
            <dt>First detected</dt><dd>{fmtDate(s.detected_at)}</dd></dl>
          {vol && <><h3 style={{ marginTop: 16 }}>Coverage over 30 days</h3><LineChart series={[{ name: 'Articles', color: 'var(--c1)', points: vol }]} height={200} /></>}
        </div>
      </div>
    </>
  )
}

export function Markets() {
  const { data, error, loading } = useData<{ series: Market[]; note: string }>('markets.json')
  const ev = useData<{ events: Event[] }>('events.json')
  const [sel, setSel] = useState<string | null>(null)
  if (error) return <ErrorBox error={error} />
  if (loading) return <Loading />
  const rows = data!.series.map((m) => {
    const p = m.points
    const ch = (k: number) => (p.length > k && p[p.length - 1 - k][1] ? (p[p.length - 1][1] / p[p.length - 1 - k][1] - 1) * 100 : null)
    return { m, last: p.at(-1), d1: ch(1), d5: ch(5), d21: ch(21) }
  })
  const current = rows.find((r) => r.m.id === sel) || rows.find((r) => r.m.points.length)
  const markers = (ev.data?.events || []).filter((e) => ['energy', 'financial_markets', 'trade'].includes(e.category) && e.outlets >= 3).slice(0, 12)
    .map((e) => ({ day: (e.started_at || e.first_detected).slice(0, 10), label: e.title.slice(0, 70) }))
  const groups: [string, string][] = [['index', '📊 Stock indices'], ['commodity', '🛢️ Commodities'], ['fx', '💱 Currencies (per US dollar)'], ['crypto', '🪙 Crypto']]
  return (
    <>
      <PageHead title="Markets" sub="Delayed end-of-day prices from free public endpoints. Not an exchange-grade real-time feed." />
      <div className="banner warn"><span>⏱️</span><div className="small">{data!.note} Each row shows the date of its latest value — check it before relying on a number. Currencies are European Central Bank reference rates (once per business day).</div></div>
      {current && (
        <div className="card" style={{ marginBottom: 16 }}>
          <div className="card-head"><div><h3>{current.m.name}</h3><div className="sub">{current.m.unit} · latest {current.last?.[0]} · {current.m.source}</div></div>
            <div className="row"><b style={{ fontSize: 22 }}>{num(current.last?.[1], 2)}</b><span className={(current.d1 || 0) >= 0 ? 'up' : 'down'}>{pct(current.d1)}</span></div></div>
          <LineChart series={[{ name: current.m.name, color: 'var(--c1)', points: current.m.points }]} markers={markers} fmt={(v) => num(v, 2)} />
          {markers.length > 0 && <div className="small faint">⚑ dashed lines mark widely reported market/energy/trade developments. Shown for context only; they do not show cause.</div>}
        </div>
      )}
      <div className="grid g2">
        {groups.map(([k, label]) => (
          <div key={k} className="card"><h3>{label}</h3>
            <table><thead><tr><th>Name</th><th className="num">Last</th><th className="num">1 day</th><th className="num">1 week</th><th className="num">1 month</th><th>Trend</th></tr></thead>
              <tbody>{rows.filter((r) => r.m.kind === k).map((r) => (
                <tr key={r.m.id} onClick={() => { setSel(r.m.id); window.scrollTo({ top: 0, behavior: 'smooth' }) }} style={{ cursor: 'pointer' }}>
                  <td><b>{r.m.name}</b><div className="small faint">{r.last ? `as of ${r.last[0]}` : 'no data — source failed'}{r.last && hoursSince(r.last[0]) > 96 ? ' · stale' : ''}</div></td>
                  <td className="num">{num(r.last?.[1], 2)}</td>
                  {[r.d1, r.d5, r.d21].map((v, i) => <td key={i} className={`num ${v == null ? '' : v >= 0 ? 'up' : 'down'}`}>{pct(v)}</td>)}
                  <td><Sparkline values={r.m.points.slice(-60).map((p) => p[1])} color={(r.d21 || 0) >= 0 ? 'var(--ok)' : 'var(--bad)'} width={90} height={28} /></td>
                </tr>))}</tbody></table></div>
        ))}
      </div>
    </>
  )
}

export function Quakes() {
  const { data, error, loading } = useData<{ quakes: (string | number | null)[][]; note: string; source: string }>('quakes.json')
  const hist = useData<History>('history.json')
  const [min, setMin] = useState<'2.5' | '4.5' | '6'>('4.5')
  if (error) return <ErrorBox error={error} />
  if (loading) return <Loading />
  const qs = data!.quakes.filter((q) => (q[2] as number) >= Number(min))
  const day = data!.quakes.filter((q) => hoursSince(q[1] as string) < 24)
  const strongest = [...data!.quakes].filter((q) => hoursSince(q[1] as string) < 168).sort((a, b) => (b[2] as number) - (a[2] as number))[0]
  const hd = hist.data?.quakes_per_day.slice(-30) || []
  return (
    <>
      <PageHead title="Earthquakes" sub={`${data!.source}. ${data!.note} Magnitudes and locations can be revised in the hours after an event.`} />
      <div className="grid g4" style={{ marginBottom: 16 }}>
        <Stat label="Last 24 hours" value={day.length} note="M2.5+ worldwide" glow="#f97316" />
        <Stat label="M4.5+ last 24 h" value={day.filter((q) => (q[2] as number) >= 4.5).length} glow="var(--c3)" />
        <Stat label="M6+ in 30 days" value={data!.quakes.filter((q) => (q[2] as number) >= 6).length} glow="var(--c7)" />
        <Stat label="Strongest (7 days)" value={strongest ? `M${strongest[2]}` : '—'} note={strongest ? String(strongest[3]) : ''} glow="var(--c4)" />
      </div>
      <div className="grid g2" style={{ marginBottom: 16 }}>
        <div className="card"><h3>Map</h3><div className="sub">Magnitude {min}+ in the published window</div>
          <DotMap points={qs.map((q) => ({ lat: q[4] as number, lon: q[5] as number, r: Math.max(1.5, ((q[2] as number) - 2) * 2), color: (q[2] as number) >= 6 ? 'var(--c7)' : (q[2] as number) >= 4.5 ? '#f97316' : 'var(--c3)', label: `M${q[2]} — ${q[3]} (${fmtDate(q[1] as string)})` }))} /></div>
        <div className="card"><h3>Activity per day</h3><div className="sub">Compare this month with earlier days. M2.5+ counts exist only for the last 7 days of each snapshot.</div>
          <StackedBars days={hd.map((r) => r[0])} stacks={[{ name: 'M2.5–4.5', color: 'var(--c3)', values: hd.map((r) => r[1]) }, { name: 'M4.5–6', color: '#f97316', values: hd.map((r) => r[2]) }, { name: 'M6+', color: 'var(--c7)', values: hd.map((r) => r[3]) }]} /></div>
      </div>
      <div className="card">
        <div className="row spread"><h3>Event list</h3><Chips options={[{ v: '2.5', label: 'M2.5+' }, { v: '4.5', label: 'M4.5+' }, { v: '6', label: 'M6+' }]} value={min} onChange={setMin} /></div>
        <div className="table-wrap" style={{ marginTop: 10 }}><table><thead><tr><th>Time (your zone)</th><th className="num">Mag.</th><th>Place</th><th className="num">Depth</th><th>Flags</th></tr></thead>
          <tbody>{qs.slice(0, 400).map((q) => (
            <tr key={q[0] as string}><td>{fmtDate(q[1] as string)}</td><td className="num"><b style={{ color: (q[2] as number) >= 6 ? 'var(--bad)' : undefined }}>{q[2]}</b></td>
              <td><Ext href={q[10] as string}>{q[3]}</Ext>{q[7] && <> · <a href={`#/country/${q[7]}`}>{flag(q[7] as string)}</a></>}</td><td className="num">{q[6] != null ? `${Math.round(q[6] as number)} km` : '—'}</td>
              <td>{q[9] && <Badge tone="b-warn" title="USGS PAGER estimated impact level">PAGER {q[9]}</Badge>} {q[8] ? <Badge tone="b-info" title="Region where tsunami information is issued; not a tsunami report">tsunami info</Badge> : null}</td></tr>))}</tbody></table></div>
      </div>
    </>
  )
}

export function EventDetail({ id }: { id: string }) {
  const { data, error } = useData<{ events: Event[] }>('events.json')
  if (error) return <ErrorBox error={error} />
  if (!data) return <Loading />
  const e = data.events.find((x) => x.id === id)
  if (!e) return <div className="empty">This development is no longer in the published 7-day window.</div>
  const byId = new Map(data.events.map((x) => [x.id, x]))
  const REL: Record<string, string> = { same_event: 'Same event', follow_up: 'Possible follow-up', shared_location: 'Same location', shared_organization: 'Same organization',
    shared_country: 'Same country & topic', reported_consequence: 'Reported consequence', possible_context: 'Possible context' }
  return (
    <>
      <PageHead title={e.title} sub={<span className="row"><VerifBadge v={e.verification} /><CatBadge c={e.category} /><span>{VERIF[e.verification]?.tip}</span></span>} />
      <div className="grid g2">
        <div className="card"><h3>Details</h3>
          {e.description && <p className="muted">{e.description}</p>}
          <dl className="kv">
            <dt>Countries</dt><dd>{e.countries.length ? e.countries.map((c) => <a key={c} href={`#/country/${c}`} style={{ marginRight: 8 }}>{flag(c)} {c}</a>) : 'not identified'}</dd>
            <dt>First reported</dt><dd>{fmtDate(e.started_at)}</dd><dt>First seen by WorldPulse</dt><dd>{fmtDate(e.first_detected)}</dd>
            <dt>Last update</dt><dd>{fmtDate(e.last_updated)}</dd><dt>Origin</dt><dd>{e.origin === 'news_cluster' ? 'Grouped news reports' : e.origin.toUpperCase()}</dd>
            {e.magnitude != null && <><dt>Magnitude</dt><dd>{e.magnitude}</dd></>}
            {e.lat != null && <><dt>Coordinates</dt><dd>{e.lat?.toFixed(2)}, {e.lon?.toFixed(2)}</dd></>}
            <dt>Extraction confidence</dt><dd title="How confident the grouping and country/topic tagging is, not whether the report is true">{Math.round(e.confidence * 100)}%</dd>
            <dt>Revision</dt><dd>{e.revision}</dd>
          </dl></div>
        <div className="card"><h3>Sources ({e.outlets} outlet{e.outlets === 1 ? '' : 's'})</h3>
          <ol className="ref-list">{e.refs.map((r, i) => <li key={i}><Ext href={r.u}>{r.t}</Ext> <span className="faint">— {r.p}{r.d ? `, ${fmtDate(r.d)}` : ''}</span></li>)}</ol>
          {e.outlets > 1 && <div className="small faint" style={{ marginTop: 8 }}>Several outlets can repeat one original report; this is not independent confirmation.</div>}</div>
      </div>
      <div className="card" style={{ marginTop: 16 }}><h3>🔗 Related developments</h3>
        <div className="sub">Links are labelled as an observed shared fact or a hypothesis. Happening close in time never proves one caused the other.</div>
        {e.related.length ? <div className="list">{e.related.map((r) => {
          const o = byId.get(r.to)
          return o ? <div key={r.to + r.type}><div className="row small" style={{ marginTop: 10 }}><Badge tone={r.observed ? 'b-info' : 'b-warn'}>{r.observed ? 'Observed link' : 'Hypothesis'}</Badge><b>{REL[r.type] || r.type}</b><span className="faint">{r.evidence} · {r.method} · confidence {Math.round(r.confidence * 100)}%</span></div><EventItem e={o} /></div> : null
        })}</div> : <div className="empty">No related developments detected.</div>}
      </div>
      <div className="small faint" style={{ marginTop: 10 }}>Updated {ago(e.last_updated)}.</div>
    </>
  )
}
