# AURA — Performance Budget

Targets are starting points. They are replaced by measured baselines after Milestone 2. No number here is a measurement until the "Measured" column says so.

## How we measure

| Tool | Used for |
|---|---|
| Macrobenchmark (`StartupTimingMetric`, `FrameTimingMetric`) | Startup and every listed interaction, on a physical phone, release-like build |
| Baseline Profiles | Ship a profile for startup + drawer + search paths |
| JankStats (debug build) | Live jank tagging per screen state |
| Android Studio profiler / Perfetto | Memory, CPU, main-thread work |
| StrictMode (debug) | Disk/network on main thread |

Frame budget: 8.33 ms at 120 Hz, 11.1 ms at 90 Hz, 16.7 ms at 60 Hz. We report frame **durations** and jank counts, never "FPS" claims.

## Targets (Realme phone, release build, after Baseline Profile)

| Scenario | Metric | Target | Measured |
|---|---|---|---|
| Cold start of Home (process killed) | timeToInitialDisplay | ≤ 400 ms | — |
| Warm return to Home (Home button) | timeToInitialDisplay | ≤ 120 ms | — |
| Home page swipe | frameDurationCpuMs P90 / P99 | ≤ 6 / ≤ 8 ms | — |
| Drawer open/close | P90 / P99 | ≤ 6 / ≤ 8 ms | — |
| Drawer fast scroll, 200 apps | P99, jank count | ≤ 8 ms, 0 janky frames per 2 s | — |
| Folder open/close | P90 / P99 | ≤ 6 / ≤ 8 ms | — |
| Search open (tap → keyboard focused) | time to first frame with field | ≤ 1 frame after touch-up for the panel; keyboard is system-timed | — |
| Search results per keystroke, 300 apps + settings | compute time | ≤ 2 ms | — |
| Design Mode enter | P99 | ≤ 8 ms | — |
| Space switch | full re-layout + first frame | ≤ 50 ms, no janky frames | — |
| Icon touch response | first visual change | same frame as ACTION_DOWN | — |
| App list load (cold, 200 apps) | to first drawer frame | ≤ 150 ms, icons may stream in | — |
| Memory, Home idle | PSS | ≤ 120 MB | — |
| Icon cache | bytes | ≤ 1/8 of memory class, hit rate ≥ 95 % after warm-up | — |

## Rules

- Any interaction over target → bug, fixed before new features (spec §100).
- An effect that cannot hold the budget degrades (Eco look) or is removed (spec §106).
- No allocation in `onDraw` / touch move paths (checked by review + allocation tracking).
