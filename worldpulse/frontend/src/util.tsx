import type { ReactNode } from 'react'

export const CAT: Record<string, { label: string; color: string; icon: string }> = {
  geopolitics: { label: 'Geopolitics', color: 'var(--c6)', icon: '🌐' },
  diplomacy: { label: 'Diplomacy', color: 'var(--c2)', icon: '🤝' },
  military_security: { label: 'Military & security', color: 'var(--c7)', icon: '🛡️' },
  elections: { label: 'Elections', color: 'var(--c4)', icon: '🗳️' },
  economics: { label: 'Economics', color: 'var(--c5)', icon: '📈' },
  financial_markets: { label: 'Financial markets', color: 'var(--c8)', icon: '💹' },
  energy: { label: 'Energy', color: 'var(--c3)', icon: '⚡' },
  trade: { label: 'Trade', color: 'var(--c1)', icon: '🚢' },
  natural_disasters: { label: 'Natural disasters', color: '#f97316', icon: '🌋' },
  technology: { label: 'Technology', color: '#3b82f6', icon: '💻' },
  cybersecurity: { label: 'Cybersecurity', color: '#a855f7', icon: '🔐' },
  public_health: { label: 'Public health', color: '#22c55e', icon: '🩺' },
  environment: { label: 'Environment', color: '#16a34a', icon: '🌿' },
  general: { label: 'General', color: 'var(--faint)', icon: '📰' },
}
export const cat = (c?: string | null) => CAT[c || 'general'] || CAT.general

// Verification levels: plain-language names and colours
export const VERIF: Record<string, { label: string; tone: string; tip: string }> = {
  instrument_observation: { label: 'Measured', tone: 'b-ok', tip: 'Recorded by a scientific instrument network (e.g. USGS seismometers).' },
  official_alert: { label: 'Official alert', tone: 'b-ok', tip: 'Issued by a public monitoring body (e.g. GDACS).' },
  official_statement: { label: 'Official source', tone: 'b-info', tip: 'Published by a government or international institution.' },
  multi_source_reporting: { label: 'Several outlets', tone: 'b-acc', tip: 'Reported by two or more different outlets. Not independently verified.' },
  single_source_report: { label: 'One outlet', tone: 'b-warn', tip: 'Reported by a single outlet so far. Treat as an unverified claim.' },
}
export const SIG_STATUS: Record<string, { label: string; tone: string; tip: string }> = {
  statistical_anomaly: { label: 'Statistical anomaly', tone: 'b-acc', tip: 'Data is unusually far from its recent normal. Not a prediction.' },
  observed_measurement: { label: 'Measured', tone: 'b-ok', tip: 'An instrument observation above a threshold.' },
  official_alert: { label: 'Official alert', tone: 'b-ok', tip: 'An alert from a public monitoring body.' },
  measurement_alert: { label: 'Unconfirmed measurement', tone: 'b-warn', tip: 'A measurement system flagged a drop; not confirmed by an operator or authority.' },
}

export function Badge({ tone = 'b-mute', children, title }: { tone?: string; children: ReactNode; title?: string }) {
  return <span className={`badge ${tone}`} title={title}>{children}</span>
}

export function flag(iso2?: string | null) {
  if (!iso2 || iso2.length !== 2 || iso2 === 'XK') return '🏳️'
  return String.fromCodePoint(...[...iso2.toUpperCase()].map((c) => 0x1f1a5 + c.charCodeAt(0)))
}

export function ago(iso?: string | null): string {
  if (!iso) return 'never'
  const t = new Date(iso.length === 10 ? iso + 'T00:00:00Z' : iso).getTime()
  if (isNaN(t)) return iso
  const s = (Date.now() - t) / 1000
  if (s < 0) return 'just now'
  if (s < 3600) return `${Math.max(1, Math.round(s / 60))} min ago`
  if (s < 86400) return `${Math.round(s / 3600)} h ago`
  return `${Math.round(s / 86400)} d ago`
}
export const hoursSince = (iso?: string | null) => (iso ? (Date.now() - new Date(iso).getTime()) / 3.6e6 : Infinity)

export function fmtDate(iso?: string | null, withTime = true) {
  if (!iso) return '—'
  const d = new Date(iso.length === 10 ? iso + 'T00:00:00Z' : iso)
  if (isNaN(d.getTime())) return iso
  return d.toLocaleString(undefined, withTime ? { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' } : { day: 'numeric', month: 'short', year: 'numeric' })
}
export const compact = (n?: number | null) => (n == null ? '—' : Intl.NumberFormat('en', { notation: 'compact', maximumFractionDigits: 1 }).format(n))
export const num = (n?: number | null, d = 2) => (n == null ? '—' : n.toLocaleString(undefined, { maximumFractionDigits: d, minimumFractionDigits: Math.min(d, 2) > 0 && Math.abs(n) < 1000 ? Math.min(d, 2) : 0 }))
export const pct = (n?: number | null) => (n == null ? '—' : `${n > 0 ? '+' : ''}${n.toFixed(2)}%`)

/** Only allow http(s) links from collected data. */
export function safeUrl(u?: string | null): string | undefined {
  if (!u) return undefined
  try {
    const p = new URL(u, location.href)
    return p.protocol === 'https:' || p.protocol === 'http:' ? p.href : undefined
  } catch { return undefined }
}
export function Ext({ href, children }: { href?: string | null; children: ReactNode }) {
  const u = safeUrl(href)
  return u ? <a href={u} target="_blank" rel="noopener noreferrer nofollow">{children}</a> : <>{children}</>
}
export const go = (path: string) => { location.hash = path }
