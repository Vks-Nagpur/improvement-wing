import type { ReactNode } from 'react'
import type { Briefing, Event, Signal } from '../data'
import { ago, Badge, cat, Ext, flag, fmtDate, go, SIG_STATUS, VERIF } from '../util'

export function PageHead({ title, sub, right }: { title: string; sub?: ReactNode; right?: ReactNode }) {
  return (
    <div className="page-head row spread">
      <div><h2>{title}</h2>{sub && <p>{sub}</p>}</div>
      {right}
    </div>
  )
}

export function Loading({ rows = 4 }: { rows?: number }) {
  return <div className="card">{Array.from({ length: rows }).map((_, i) => <div key={i} className="skeleton" style={{ height: 18, margin: '12px 0', width: `${90 - i * 12}%` }} />)}</div>
}

export function ErrorBox({ error }: { error: string }) {
  return (
    <div className="banner bad">
      <span>⚠️</span>
      <div><b>This data could not be loaded.</b><div className="small">{error}. The public data is published by the "WorldPulse data" GitHub workflow; if it has not run yet, no data exists.</div></div>
    </div>
  )
}

export function Stat({ label, value, note, glow, onClick }: { label: string; value: ReactNode; note?: ReactNode; glow?: string; onClick?: () => void }) {
  return (
    <div className="card stat" style={{ ['--glow' as string]: glow, cursor: onClick ? 'pointer' : undefined }} onClick={onClick}>
      <div className="label">{label}</div>
      <div className="value">{value}</div>
      {note && <div className="note">{note}</div>}
    </div>
  )
}

export function VerifBadge({ v }: { v: string }) {
  const x = VERIF[v] || { label: v, tone: 'b-mute', tip: '' }
  return <Badge tone={x.tone} title={x.tip}><span className="dot" />{x.label}</Badge>
}

export function CatBadge({ c }: { c: string }) {
  const x = cat(c)
  return <span className="badge b-mute" style={{ color: x.color }}>{x.icon} {x.label}</span>
}

export function EventItem({ e, compact = false }: { e: Event; compact?: boolean }) {
  const ref = e.refs[0]
  return (
    <div className="item">
      <div className="cat-bar" style={{ background: cat(e.category).color }} />
      <div style={{ minWidth: 0, flex: 1 }}>
        <div className="t"><a href={`#/event/${encodeURIComponent(e.id)}`}>{e.title}</a></div>
        <div className="m">
          <VerifBadge v={e.verification} />
          {!compact && <CatBadge c={e.category} />}
          {e.country && <a href={`#/country/${e.country}`} className="muted">{flag(e.country)} {e.country}</a>}
          <span title={e.started_at || ''}>{ago(e.started_at || e.first_detected)}</span>
          {ref && <span>· <Ext href={ref.u}>{ref.p || 'source'}</Ext>{e.outlets > 1 ? ` +${e.outlets - 1} more` : ''}</span>}
          {e.related.length > 0 && <span>· 🔗 {e.related.length} related</span>}
        </div>
      </div>
    </div>
  )
}

export function SignalItem({ s }: { s: Signal }) {
  const st = SIG_STATUS[s.status] || { label: s.status, tone: 'b-mute', tip: '' }
  return (
    <div className="item">
      <div style={{ fontSize: 22, lineHeight: 1 }}>{sigIcon(s.signal_type)}</div>
      <div style={{ minWidth: 0, flex: 1 }}>
        <div className="t"><a href={`#/signal/${encodeURIComponent(s.id)}`}>{s.title}</a></div>
        <div className="m">
          <Badge tone={st.tone} title={st.tip}>{st.label}</Badge>
          <Badge tone={s.confidence === 'high' ? 'b-ok' : s.confidence === 'medium' ? 'b-info' : 'b-warn'} title="Confidence that the detection is real (not that anything will happen)">{s.confidence} confidence</Badge>
          {s.country && <a className="muted" href={`#/country/${s.country}`}>{flag(s.country)} {s.country}</a>}
          <span>{measure(s)}</span>
          <span className="faint">detected {ago(s.detected_at)}</span>
        </div>
      </div>
    </div>
  )
}

export function sigIcon(t: string) {
  return ({ news_volume: '📰', topic_volume: '📣', earthquake: '🌍', quake_cluster: '〰️', market: '📉', fx: '💱', disaster_alert: '🚨', internet: '📡' } as Record<string, string>)[t] || '◆'
}

export function measure(s: Signal) {
  if (s.signal_type === 'news_volume' || s.signal_type === 'topic_volume')
    return `${Math.round(s.observed || 0)} articles vs ~${Math.round(s.baseline || 0)} normal (${(s.pct_change || 0) > 0 ? '+' : ''}${Math.round(s.pct_change || 0)}%)`
  if (s.signal_type === 'market' || s.signal_type === 'fx') return `${(s.pct_change || 0) > 0 ? '+' : ''}${(s.pct_change || 0).toFixed(2)}% · ${s.deviation?.toFixed(1)}σ · data ${s.data_freshness}`
  if (s.signal_type === 'earthquake') return `magnitude ${s.observed?.toFixed(1)}`
  if (s.signal_type === 'quake_cluster') return `${s.observed} quakes in 24 h vs ~${s.baseline}/day`
  return `${s.observed ?? ''} ${s.unit || ''}`
}

export function BriefingView({ b }: { b: Briefing }) {
  return (
    <div>
      {b.ai_summary && (
        <div className="banner info"><span>🤖</span><div><b>AI summary (local model, citations checked)</b><div>{b.ai_summary}</div></div></div>
      )}
      {b.sections.map((s) => (
        <div key={s.heading} style={{ marginBottom: 16 }}>
          <h3 style={{ fontSize: 15, marginBottom: 6 }}>{s.heading}</h3>
          <ul style={{ margin: 0, paddingLeft: 20 }}>
            {s.items.map((it, i) => (
              <li key={i} style={{ margin: '5px 0' }}>
                {it.event_id ? <a href={`#/event/${encodeURIComponent(it.event_id)}`} style={{ color: 'var(--text)' }}>{it.text}</a> : it.text}
                {it.refs.map((r) => <sup key={r} className="ref"><a href={`#ref-${b.kind}-${r}`} onClick={(e) => { e.preventDefault(); document.getElementById(`ref-${b.kind}-${b.scope}-${r}`)?.scrollIntoView({ behavior: 'smooth', block: 'center' }) }}>{r}</a></sup>)}
              </li>
            ))}
          </ul>
        </div>
      ))}
      <details open={b.references.length < 12}>
        <summary>Sources ({b.references.length})</summary>
        <ol className="ref-list">
          {b.references.map((r) => (
            <li key={r.id} id={`ref-${b.kind}-${b.scope}-${r.id}`}><Ext href={r.url}>{r.title}</Ext> <span className="faint">— {r.publisher || r.kind}{r.date ? `, ${fmtDate(r.date)}` : ''}</span></li>
          ))}
        </ol>
      </details>
      <div className="banner warn" style={{ marginTop: 14 }}>
        <span>ℹ️</span>
        <div className="small"><b>Data limitations</b><ul style={{ margin: '4px 0 0', paddingLeft: 18 }}>{b.limitations.map((l) => <li key={l}>{l}</li>)}</ul>
          <div className="faint" style={{ marginTop: 4 }}>Generated {fmtDate(b.generated_at)} · {b.mode === 'deterministic' ? 'template mode (no AI)' : b.mode}</div></div>
      </div>
    </div>
  )
}

export function Chips<T extends string>({ options, value, onChange }: { options: { v: T; label: ReactNode }[]; value: T; onChange: (v: T) => void }) {
  return <div className="chips">{options.map((o) => <button key={o.v} className={`chip ${o.v === value ? 'on' : ''}`} onClick={() => onChange(o.v)}>{o.label}</button>)}</div>
}

export function CountryLink({ iso2, name }: { iso2: string; name?: string }) {
  return <a onClick={() => go(`/country/${iso2}`)} href={`#/country/${iso2}`}>{flag(iso2)} {name || iso2}</a>
}
