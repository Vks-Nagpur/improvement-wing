// Data access. Static mode reads the JSON published by the pipeline
// (./data/*.json). If VITE_API_URL is set (self-hosted mode) the Ask and
// Search features also call the local API.
import { useEffect, useState } from 'react'

export const API = (import.meta.env.VITE_API_URL as string | undefined) || ''
const BASE = `${import.meta.env.BASE_URL}data/`

export type Ref = { t: string; u?: string | null; p?: string | null; d?: string | null }
export type Rel = { to: string; type: string; evidence: string; method: string; confidence: number; observed: boolean }
export type Event = {
  id: string; title: string; category: string; description?: string | null; country?: string | null; countries: string[]
  lat?: number | null; lon?: number | null; started_at?: string | null; first_detected: string; last_updated: string
  origin: string; verification: string; confidence: number; magnitude?: number | null; revision: number; outlets: number
  refs: Ref[]; related: Rel[]
}
export type Article = { id: string; t: string; u: string; p?: string; x?: string | null; d: string; g: string; c: string; k: string[]; cl: string; n: number; o: boolean; s: string; l?: string }
export type Evidence = { type: string; title: string; url?: string; publisher?: string; published_at?: string; detail?: string }
export type Signal = {
  id: string; signal_type: string; category: string; country?: string | null; region?: string | null; title: string; detected_at: string
  observed?: number | null; baseline?: number | null; baseline_std?: number | null; deviation?: number | null; pct_change?: number | null
  score?: number | null; unit?: string | null; evidence: Evidence[]; data_freshness?: string | null; confidence: string; limitations: string; status: string
}
export type Indicator = { label: string; value: number; unit: string; period: string; source: string; url: string; retrieved: string }
export type CountrySummary = {
  iso2: string; iso3: string; name: string; region: string; wb_region?: string; income?: string; capital?: string; lat?: number; lon?: number
  population?: { value: number; period: string } | null; gdp?: { value: number; period: string } | null
  events_24h: number; events_7d: number; news_72h: number; signals_30d: number; top_category?: string | null
}
export type BriefingItem = { text: string; refs: number[]; event_id?: string; signal_id?: string; verification?: string }
export type Briefing = {
  kind: string; scope?: string | null; period_hours: number; generated_at: string; mode: string
  sections: { heading: string; items: BriefingItem[] }[]
  references: { id: number; title: string; url?: string | null; publisher?: string | null; date?: string | null; kind: string }[]
  limitations: string[]; ai_summary?: string; verification_legend: Record<string, string>
}
export type CountryProfile = {
  generated_at: string; country: CountrySummary; meta_source?: string; meta_updated?: string
  indicators: Record<string, Indicator>; events: Event[]; news: Article[]; signals: Signal[]; volume: [string, number][]
  briefings: Record<string, Briefing>
}
export type Source = {
  id: string; name: string; kind: string; type?: string; category?: string | null; country?: string | null; url: string; enabled: boolean
  interval_minutes: number; last_success?: string | null; last_attempt?: string | null; last_error?: string | null; failures: number
  last_items?: number | null; last_new?: number | null; status: string
}
export type Market = { id: string; name: string; kind: string; unit: string; country?: string | null; source?: string | null; points: [string, number][] }
export type Meta = {
  generated_at: string; app_version: string; processing_version: string; stale_after_hours: number
  counts: Record<string, number>; sources: { total: number; ok: number; failing: number; disabled: number }
  category_labels: Record<string, string>
}
export type History = {
  articles_per_day: [string, number, number][]; category_per_day: [string, string, number][]
  quakes_per_day: [string, number, number, number][]; signals_per_day: [string, string, number][]
  topic_volume: Record<string, [string, number][]>; country_volume: Record<string, [string, number][]>; collection_days: string[]
}

const cache = new Map<string, Promise<unknown>>()

export function load<T>(file: string): Promise<T> {
  if (!cache.has(file)) {
    cache.set(file, fetch(BASE + file, { cache: 'no-cache' }).then((r) => {
      if (!r.ok) throw new Error(`${file}: HTTP ${r.status}`)
      return r.json()
    }))
  }
  return cache.get(file) as Promise<T>
}

export function useData<T>(file: string | null): { data?: T; error?: string; loading: boolean } {
  const [state, setState] = useState<{ data?: T; error?: string; loading: boolean }>({ loading: true })
  useEffect(() => {
    if (!file) return
    let live = true
    setState({ loading: true })
    load<T>(file).then((data) => live && setState({ data, loading: false }), (e) => live && setState({ error: String(e.message || e), loading: false }))
    return () => { live = false }
  }, [file])
  return state
}

export async function apiQuery(question: string) {
  const r = await fetch(`${API}/api/query`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ question }) })
  if (!r.ok) throw new Error(`API ${r.status}`)
  return r.json()
}
