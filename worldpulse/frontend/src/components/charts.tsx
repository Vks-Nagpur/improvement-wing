// Small dependency-free SVG charts that measure their container.
import { useEffect, useRef, useState } from 'react'

function useWidth<T extends HTMLElement>() {
  const ref = useRef<T>(null)
  const [w, setW] = useState(600)
  useEffect(() => {
    if (!ref.current) return
    const ro = new ResizeObserver(([e]) => setW(Math.max(200, e.contentRect.width)))
    ro.observe(ref.current)
    return () => ro.disconnect()
  }, [])
  return [ref, w] as const
}

export function Sparkline({ values, color = 'var(--accent)', height = 36, width = 120 }: { values: number[]; color?: string; height?: number; width?: number }) {
  if (values.length < 2) return <svg width={width} height={height} />
  const min = Math.min(...values), max = Math.max(...values)
  const x = (i: number) => (i / (values.length - 1)) * (width - 2) + 1
  const y = (v: number) => height - 2 - ((v - min) / (max - min || 1)) * (height - 4)
  const d = values.map((v, i) => `${i ? 'L' : 'M'}${x(i).toFixed(1)},${y(v).toFixed(1)}`).join('')
  return (
    <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`} aria-hidden>
      <path d={d} fill="none" stroke={color} strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  )
}

type Series = { name: string; color: string; points: [string, number][] }

export function LineChart({ series, height = 240, fmt = (v: number) => v.toLocaleString(), markers = [] }:
  { series: Series[]; height?: number; fmt?: (v: number) => string; markers?: { day: string; label: string }[] }) {
  const [ref, w] = useWidth<HTMLDivElement>()
  const [hover, setHover] = useState<number | null>(null)
  const all = series.flatMap((s) => s.points)
  if (!all.length) return <div className="empty">No data points yet.</div>
  const days = [...new Set(all.map((p) => p[0]))].sort()
  const vals = all.map((p) => p[1])
  const min = Math.min(...vals), max = Math.max(...vals)
  const pad = { l: 8 + Math.max(...[min, max].map((v) => fmt(v).length)) * 7, r: 12, t: 10, b: 24 }
  const iw = w - pad.l - pad.r, ih = height - pad.t - pad.b
  const x = (d: string) => pad.l + (days.indexOf(d) / Math.max(1, days.length - 1)) * iw
  const y = (v: number) => pad.t + ih - ((v - min) / (max - min || 1)) * ih
  const ticks = [min, (min + max) / 2, max]
  const xt = days.filter((_, i) => i % Math.ceil(days.length / Math.max(2, Math.floor(iw / 90))) === 0)
  const hd = hover != null ? days[hover] : null
  return (
    <div className="chart" ref={ref} onMouseLeave={() => setHover(null)}
      onMouseMove={(e) => {
        const r = (e.currentTarget as HTMLDivElement).getBoundingClientRect()
        const i = Math.round(((e.clientX - r.left - pad.l) / iw) * (days.length - 1))
        setHover(Math.max(0, Math.min(days.length - 1, i)))
      }}>
      <svg width="100%" height={height} viewBox={`0 0 ${w} ${height}`}>
        {ticks.map((t, i) => (
          <g key={i}>
            <line x1={pad.l} x2={w - pad.r} y1={y(t)} y2={y(t)} stroke="var(--border)" />
            <text x={pad.l - 6} y={y(t) + 4} textAnchor="end" fontSize={11} fill="var(--faint)">{fmt(t)}</text>
          </g>
        ))}
        {xt.map((d) => <text key={d} x={x(d)} y={height - 6} textAnchor="middle" fontSize={11} fill="var(--faint)">{d.slice(5)}</text>)}
        {markers.filter((m) => days.includes(m.day)).map((m) => (
          <g key={m.day + m.label}><line x1={x(m.day)} x2={x(m.day)} y1={pad.t} y2={pad.t + ih} stroke="var(--warn)" strokeDasharray="4 4" /></g>
        ))}
        {series.map((s) => {
          const pts = s.points.filter((p) => days.includes(p[0]))
          const d = pts.map((p, i) => `${i ? 'L' : 'M'}${x(p[0]).toFixed(1)},${y(p[1]).toFixed(1)}`).join('')
          return <path key={s.name} d={d} fill="none" stroke={s.color} strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" />
        })}
        {hd && <line x1={x(hd)} x2={x(hd)} y1={pad.t} y2={pad.t + ih} stroke="var(--faint)" />}
        {hd && series.map((s) => {
          const p = s.points.find((q) => q[0] === hd)
          return p ? <circle key={s.name} cx={x(hd)} cy={y(p[1])} r={4} fill={s.color} /> : null
        })}
      </svg>
      {hd && (
        <div className="tooltip" style={{ left: Math.min(x(hd) + 10, w - 180), top: 6 }}>
          <div className="muted">{hd}</div>
          {series.map((s) => {
            const p = s.points.find((q) => q[0] === hd)
            return p ? <div key={s.name}><i style={{ display: 'inline-block', width: 8, height: 8, borderRadius: 2, background: s.color, marginRight: 6 }} />{s.name}: <b>{fmt(p[1])}</b></div> : null
          })}
          {markers.filter((m) => m.day === hd).map((m) => <div key={m.label} style={{ color: 'var(--warn)' }}>⚑ {m.label}</div>)}
        </div>
      )}
      {series.length > 1 && <div className="legend">{series.map((s) => <span key={s.name}><i style={{ background: s.color }} />{s.name}</span>)}</div>}
    </div>
  )
}

export function StackedBars({ days, stacks, height = 220 }: { days: string[]; stacks: { name: string; color: string; values: number[] }[]; height?: number }) {
  const [ref, w] = useWidth<HTMLDivElement>()
  const [hover, setHover] = useState<number | null>(null)
  if (!days.length) return <div className="empty">No data yet.</div>
  const totals = days.map((_, i) => stacks.reduce((a, s) => a + (s.values[i] || 0), 0))
  const max = Math.max(1, ...totals)
  const pad = { l: 36, r: 8, t: 8, b: 24 }
  const iw = w - pad.l - pad.r, ih = height - pad.t - pad.b
  const bw = Math.min(32, (iw / days.length) * 0.7)
  const x = (i: number) => pad.l + (i + 0.5) * (iw / days.length)
  const step = Math.ceil(days.length / Math.max(2, Math.floor(iw / 60)))
  return (
    <div className="chart" ref={ref} onMouseLeave={() => setHover(null)}>
      <svg width="100%" height={height} viewBox={`0 0 ${w} ${height}`}>
        {[0, 0.5, 1].map((f) => (
          <g key={f}>
            <line x1={pad.l} x2={w - pad.r} y1={pad.t + ih * (1 - f)} y2={pad.t + ih * (1 - f)} stroke="var(--border)" />
            <text x={pad.l - 6} y={pad.t + ih * (1 - f) + 4} textAnchor="end" fontSize={11} fill="var(--faint)">{Math.round(max * f)}</text>
          </g>
        ))}
        {days.map((d, i) => {
          let acc = 0
          return (
            <g key={d} onMouseEnter={() => setHover(i)} opacity={hover == null || hover === i ? 1 : 0.55}>
              <rect x={x(i) - (iw / days.length) / 2} y={pad.t} width={iw / days.length} height={ih} fill="transparent" />
              {stacks.map((s) => {
                const v = s.values[i] || 0
                const h = (v / max) * ih
                acc += h
                return v ? <rect key={s.name} x={x(i) - bw / 2} y={pad.t + ih - acc} width={bw} height={Math.max(h - 1, 0.5)} rx={3} fill={s.color} /> : null
              })}
              {i % step === 0 && <text x={x(i)} y={height - 6} textAnchor="middle" fontSize={11} fill="var(--faint)">{d.slice(5)}</text>}
            </g>
          )
        })}
      </svg>
      {hover != null && (
        <div className="tooltip" style={{ left: Math.min(x(hover) + 12, w - 170), top: 4 }}>
          <div className="muted">{days[hover]}</div>
          {stacks.map((s) => s.values[hover] ? <div key={s.name}><i style={{ display: 'inline-block', width: 8, height: 8, borderRadius: 2, background: s.color, marginRight: 6 }} />{s.name}: <b>{s.values[hover]}</b></div> : null)}
        </div>
      )}
      <div className="legend">{stacks.map((s) => <span key={s.name}><i style={{ background: s.color }} />{s.name}</span>)}</div>
    </div>
  )
}

/** Equirectangular dot map (no external tiles). */
export function DotMap({ points, height = 300 }: { points: { lat: number; lon: number; r: number; color: string; label: string; href?: string }[]; height?: number }) {
  const [ref, w] = useWidth<HTMLDivElement>()
  const [hover, setHover] = useState<number | null>(null)
  const h = Math.min(height, w / 2)
  const x = (lon: number) => ((lon + 180) / 360) * w
  const y = (lat: number) => ((90 - lat) / 180) * h
  return (
    <div className="chart" ref={ref}>
      <svg width="100%" height={h} viewBox={`0 0 ${w} ${h}`} style={{ background: 'var(--panel-2)', borderRadius: 12 }}>
        {[-60, -30, 0, 30, 60].map((lat) => <line key={lat} x1={0} x2={w} y1={y(lat)} y2={y(lat)} stroke="var(--border)" strokeDasharray={lat ? '2 4' : ''} />)}
        {[-120, -60, 0, 60, 120].map((lon) => <line key={lon} y1={0} y2={h} x1={x(lon)} x2={x(lon)} stroke="var(--border)" strokeDasharray="2 4" />)}
        {points.map((p, i) => (
          <circle key={i} cx={x(p.lon)} cy={y(p.lat)} r={p.r} fill={p.color} fillOpacity={0.55} stroke={p.color} strokeWidth={1}
            onMouseEnter={() => setHover(i)} onMouseLeave={() => setHover(null)} style={{ cursor: p.href ? 'pointer' : 'default' }}
            onClick={() => p.href && (location.hash = p.href)} />
        ))}
      </svg>
      {hover != null && points[hover] && (
        <div className="tooltip" style={{ left: Math.min(x(points[hover].lon) + 10, w - 220), top: Math.max(0, y(points[hover].lat) - 30) }}>{points[hover].label}</div>
      )}
      <div className="small faint" style={{ marginTop: 6 }}>Positions on a simple latitude/longitude grid (no basemap).</div>
    </div>
  )
}
