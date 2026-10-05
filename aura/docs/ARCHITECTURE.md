# AURA — Architecture

"Extraordinary UX. Conservative engineering."

## 1. Shape of the app

```
┌───────────────────────────── HomeActivity (single Activity, singleTask) ─────────────────────────────┐
│  Workspace (custom View)   Dock (View)   Drawer + Search (RecyclerView)   Folder overlay   Design Mode   │
│  Inbox screen (Compose)    Settings, Onboarding, Privacy (Compose, separate Activity)                    │
└───────────────┬──────────────────────────────────────┬──────────────────────────────────┬────────────┘
                │ StateFlow                            │ StateFlow                        │ StateFlow
        ┌───────▼────────┐                    ┌────────▼─────────┐               ┌────────▼─────────┐
        │ AppsRepository │                    │ LayoutRepository │               │ InboxRepository  │
        │ LauncherApps   │                    │ DataStore (JSON) │               │ in-memory +      │
        │ + Callback     │                    │ AuraConfig v1    │               │ Room (history,   │
        │ IconCache (LRU)│                    │ validate→default │               │ only if enabled) │
        └────────────────┘                    └──────────────────┘               └────────▲─────────┘
                                                                                          │ events
                                                                          AuraNotificationListener
                                                                          (NotificationListenerService,
                                                                           optional, never required by Home)
```

## 2. Rendering choice (spec §8: measure, don't assume)

| Surface | Technology | Reason |
|---|---|---|
| Workspace pages, icons, drag & drop, page swipe | Android Views (custom `ViewGroup`) | Direct touch tracking, `VelocityTracker`, `RenderNode`-level transforms, no recomposition on every frame. Well-proven for launchers. |
| Dock, Action Dock arc | Views | Must open in the same frame as the touch. |
| App drawer list + search results | `RecyclerView` | Mature recycling, fast scroller, predictable memory. |
| Inbox, Settings, Privacy, Onboarding, Design Mode panel | Jetpack Compose | Form-heavy screens; Compose speeds development. Measured with Macrobenchmark; moved to Views only if numbers say so. |

## 3. Modules (start small — spec §93)

| Gradle module | Contents | Why separate |
|---|---|---|
| `:app` | Activities, Views, Compose screens, services, repositories | The product |
| `:core` | Pure Kotlin: `AuraConfig` model, validation, migrations, search ranking, command parser, motion tokens, temporary-app expiry logic | Fast JVM unit tests, no Android needed |
| `:benchmark` | Macrobenchmark + Baseline Profile generator | Required by the performance plan |

Packages inside `:app`: `home`, `drawer`, `search`, `folders`, `widgets`, `notifications`, `spaces`, `themes`, `settings`, `permissions`, `performance`, `design` (tokens). Split into modules only when build time or ownership demands it.

## 4. Data

- **One configuration object**: `AuraConfig(schemaVersion, spaces[], activeSpaceId, theme, motion, batteryMode, gestures, hiddenApps, temporaryApps[], prefs)`. Each Space owns pages, dock, folders, widget placements.
- Stored with **DataStore** as JSON (kotlinx.serialization). Writes are debounced and happen only on user change — never on a timer.
- On read: parse → validate (bounds, unknown IDs, duplicate positions) → repair or fall back to safe default. A bad file can never stop Home from drawing.
- Same model is the backup/export and "Share my setup" format (with notification data never included).
- **Room** only for notification history (needs search + retention deletes). Not created until the user turns history on.

## 5. Event-driven by design

| Signal | Source | Who listens | When |
|---|---|---|---|
| Apps added/removed/updated | `LauncherApps.Callback` | AppsRepository | Always registered while process alive (no polling; cost only on change) |
| Minute changed | `ACTION_TIME_TICK` | Clock | Only between `onStart`/`onStop` |
| Battery changed | `ACTION_BATTERY_CHANGED` | Battery widget | Only while widget visible |
| Wallpaper colors | `OnColorsChangedListener` | Theme engine | Result cached in config |
| Notifications | `NotificationListenerService` callbacks | InboxRepository | Only if user granted access |
| Temporary apps expiry | Checked on `onStart` and on each minute tick while visible | LayoutRepository | No alarm, no job |
| "Later" | `snoozeNotification` — Android brings it back | — | No AURA timer at all |

## 6. Launcher-specific behaviour

- `HomeActivity`: `launchMode="singleTask"`, `stateNotNeeded="true"`, `clearTaskOnLaunch="true"`, `resumeWhilePausing="true"`, `excludeFromRecents="true"`, theme with `windowShowWallpaper`. Pressing Home while on Home → go to page 1 / close overlays.
- Back handled with `OnBackPressedCallback` (required for target 36).
- Edge-to-edge with window insets everywhere.
- Process death: all visible state is derived from `AuraConfig` + repositories; transient UI state (open folder, drawer scroll) in `SavedStateHandle`.

## 7. Icon cache

- Key: component + user + package `lastUpdateTime` + icon size.
- In-memory `LruCache` sized in **bytes**, starting budget 1/8 of app memory class, tuned by measurement (PERFORMANCE_BUDGET).
- Decode off the main thread; placeholder tint from cached dominant color so nothing "pops".
- Invalidate per package on `LauncherApps.Callback`.

## 8. Failure isolation

- Each widget wrapped in a host view that catches inflation/update errors and shows a recover tile.
- Notification parsing wrapped per item; malformed extras skip that item only.
- Config import: validate in `:core`, preview, then apply; undo available.
- `Thread.setDefaultUncaughtExceptionHandler` records a local crash note (never uploaded) and offers "Reset layout" on next start if Home crashes twice in a row (safe mode).

## 9. Security

- Only `HomeActivity` exported (required for Home). Listener service exported only as the system requires, guarded by `BIND_NOTIFICATION_LISTENER_SERVICE`.
- No INTERNET permission in V1. (Clear statement for users and an easy audit.)
- No notification text in logs; release builds strip verbose logging.
