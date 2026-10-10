import { StrictMode, useEffect, useState } from 'react'
import { createRoot } from 'react-dom/client'
import './styles.css'
import type { Meta } from './data'
import { useData } from './data'
import { Countries, Country } from './pages/countries'
import { Monitor, Overview } from './pages/overview'
import { EventDetail, Markets, Quakes, SignalDetail, Signals } from './pages/signals'
import { Alerts, Ask, Briefings, evaluateAlerts, Search, Sources } from './pages/tools'
import { ago, hoursSince } from './util'

function useRoute() {
  const [h, setH] = useState(location.hash.slice(1) || '/')
  useEffect(() => {
    const f = () => { setH(location.hash.slice(1) || '/'); window.scrollTo(0, 0) }
    addEventListener('hashchange', f)
    return () => removeEventListener('hashchange', f)
  }, [])
  return h
}

function useTheme() {
  const pref = () => { try { return localStorage.getItem('worldpulse.theme') } catch { return null } }
  const [t, setT] = useState<string>(pref() || 'light')
  useEffect(() => {
    document.documentElement.dataset.theme = t
    try { localStorage.setItem('worldpulse.theme', t) } catch { /* ignore */ }
  }, [t])
  return [t, () => setT(t === 'dark' ? 'light' : 'dark')] as const
}

const NAV: [string, string, string, string?][] = [
  ['Monitor', '/', '🏠', 'Overview'], ['Monitor', '/monitor', '🗞️', 'World Monitor'], ['Monitor', '/countries', '🌐', 'Countries'],
  ['Monitor', '/signals', '⚡', 'OpenSignals'], ['Data', '/markets', '💹', 'Markets'], ['Data', '/quakes', '🌋', 'Earthquakes'],
  ['Intelligence', '/briefings', '📋', 'Briefings'], ['Intelligence', '/ask', '💬', 'Ask WorldPulse'], ['Intelligence', '/search', '🔎', 'Search'],
  ['Intelligence', '/alerts', '🔔', 'Alerts'], ['About', '/sources', '🧪', 'Sources & method'],
]

function App() {
  const route = useRoute()
  const [theme, toggle] = useTheme()
  const [open, setOpen] = useState(false)
  const [q, setQ] = useState('')
  const meta = useData<Meta>('meta.json')
  useEffect(() => { evaluateAlerts().catch(() => undefined) }, [])
  useEffect(() => setOpen(false), [route])
  const [path, query] = route.split('?')
  const parts = path.split('/').filter(Boolean)
  let page
  switch (parts[0]) {
    case undefined: page = <Overview />; break
    case 'monitor': page = <Monitor />; break
    case 'countries': page = <Countries />; break
    case 'country': page = <Country code={parts[1] || ''} key={parts[1]} />; break
    case 'signals': page = <Signals />; break
    case 'signal': page = <SignalDetail id={decodeURIComponent(parts[1] || '')} />; break
    case 'event': page = <EventDetail id={decodeURIComponent(parts[1] || '')} />; break
    case 'markets': page = <Markets />; break
    case 'quakes': page = <Quakes />; break
    case 'briefings': page = <Briefings />; break
    case 'ask': page = <Ask />; break
    case 'search': page = <Search initial={new URLSearchParams(query).get('q') || ''} />; break
    case 'alerts': page = <Alerts />; break
    case 'sources': page = <Sources />; break
    default: page = <div className="empty">Page not found. <a href="#/">Go home</a></div>
  }
  const stale = meta.data && hoursSince(meta.data.generated_at) > meta.data.stale_after_hours
  let group = ''
  return (
    <div className="app">
      <aside className={`side ${open ? 'open' : ''}`}>
        <div className="brand">
          <div className="brand-logo"><svg width="20" height="20" viewBox="0 0 24 24"><path d="M2 13h4l3-7 5 13 3-6h5" stroke="white" strokeWidth="2.4" fill="none" strokeLinecap="round" strokeLinejoin="round" /></svg></div>
          <div><h1>WorldPulse</h1><small>Open global intelligence</small></div>
        </div>
        <nav className="nav">
          {NAV.map(([g, href, icon, label]) => {
            const head = g !== group ? <div className="nav-group">{(group = g)}</div> : null
            const active = href === '/' ? path === '/' : path.startsWith(href) || (href === '/countries' && parts[0] === 'country') || (href === '/signals' && parts[0] === 'signal')
            return <div key={href}>{head}<a href={`#${href}`} className={active ? 'active' : ''}><span>{icon}</span>{label}
              {href === '/signals' && meta.data ? <span className="count">{meta.data.counts.signals_30d}</span> : null}</a></div>
          })}
        </nav>
        <div className="small faint" style={{ padding: '18px 10px 0' }}>
          {meta.data ? <>Data updated {ago(meta.data.generated_at)}<br />v{meta.data.app_version} · processing {meta.data.processing_version}</> : 'Loading data…'}
          <br />Free &amp; open source · public data only
        </div>
      </aside>
      <main className="main">
        <div className="topbar">
          <button className="icon-btn menu-btn" onClick={() => setOpen(!open)} aria-label="Menu">☰</button>
          <div className="search">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
            <input placeholder="Search countries, events, headlines…  (press Enter)" value={q} onChange={(e) => setQ(e.target.value)}
              onKeyDown={(e) => { if (e.key === 'Enter' && q.trim()) location.hash = `/search?q=${encodeURIComponent(q.trim())}` }} />
          </div>
          <a className="icon-btn" href="#/ask" style={{ textDecoration: 'none', color: 'inherit' }}>💬 Ask</a>
          <button className="icon-btn" onClick={toggle} aria-label="Toggle theme">{theme === 'dark' ? '☀️ Light' : '🌙 Dark'}</button>
        </div>
        {stale && <div className="banner warn"><span>⏳</span><div><b>This data is {ago(meta.data!.generated_at)} old.</b> <span className="small">The scheduled update may be delayed (GitHub runs schedules on a best-effort basis) or failing. Check Sources &amp; method.</span></div></div>}
        {page}
      </main>
    </div>
  )
}

createRoot(document.getElementById('root')!).render(<StrictMode><App /></StrictMode>)
