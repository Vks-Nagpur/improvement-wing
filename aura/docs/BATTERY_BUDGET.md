# AURA — Battery Budget

Rule: **at idle, AURA does nothing.** No periodic background work unless a row below justifies it.
"Expected impact" is a design estimate, not a measurement. Measurements go in the last column when they exist.

## Component audit

| Component | Purpose | Trigger | Frequency | Stops when | Screen off | AURA not default Home | Expected impact | Measured |
|---|---|---|---|---|---|---|---|---|
| LauncherApps.Callback | Keep app list correct | App installed/updated/removed | Only on change | Process ends | No work unless a package changes | Same (cheap) | Negligible | — |
| Clock (`TIME_TICK`) | Show the minute | System broadcast | 1/min | `onStop` → unregistered | Unregistered | Unregistered (Home not shown) | Negligible | — |
| Battery widget | Show % and charging | `ACTION_BATTERY_CHANGED` | On change | Widget hidden → unregistered | Unregistered | Unregistered | Negligible | — |
| Wallpaper colors | Readable text over wallpaper | `OnColorsChangedListener` | On wallpaper change | Result cached | None | None | Negligible | — |
| Temporary Home apps | Remove expired shortcuts | Home shown / minute tick while visible | Only while visible | — | None | None | None | — |
| Notification listener | Inbox, history | System callbacks | Per notification | Access revoked / user disables Inbox | Receives events (system binds it); work kept tiny: update in-memory list, write history row only if history is on | Keeps running if access granted (user's choice); can be turned off in AURA | Low; must measure | — |
| History cleanup | Enforce retention | Inbox opened / listener connected | At most once per day, piggybacking on those events | — | None | None | Negligible | — |
| "Later" | Bring back notification | `snoozeNotification` (Android does the timing) | — | — | None from AURA | None | None | — |
| Animations | Motion | User touch | Only while running | Animation end / window hidden | Stopped | Stopped | Per use | — |
| Widgets (third-party) | User's widgets | Provider updates | Provider's choice | `AppWidgetHost.stopListening()` in `onStop` | Not listening | Not listening | Provider-dependent | — |

## Audit list (kept at zero unless a row above says otherwise)

| Kind | V1 count | Notes |
|---|---|---|
| Foreground services | 0 | |
| Background services (our own) | 0 | The notification listener is bound by the system, not started by us |
| WorkManager jobs | 0 | |
| Alarms | 0 | Later uses Android's snooze |
| Wake locks | 0 | |
| Network | 0 | No INTERNET permission in V1 |
| Sensors | 0 | No location, no motion sensors |
| Manifest broadcast receivers | 0 | Runtime registration only |

## Battery modes

| Mode | Animations | Effects | Frame-rate request |
|---|---|---|---|
| Eco | Shortest durations, no parallax, no blur | Solid surfaces only | None (system default) |
| Balanced (default) | Full motion tokens | Wallpaper parallax | None |
| Smooth | Full motion, richer springs | Parallax | May call `Surface.setFrameRate` with the display's max during gestures only; never forced |

## Release gate

Before any beta: 24 h idle with AURA as Home on the Realme phone, screen off; AURA must not appear as a notable consumer in the system battery screen. If it does, that is a bug (spec §105).
