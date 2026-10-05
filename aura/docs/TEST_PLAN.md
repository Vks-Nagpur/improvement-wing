# AURA — Test Plan

Honesty rule: a test is "done" only when it actually ran. Physical-device results are recorded with device, OS version and date.

## Automated

| Layer | Tool | Covers |
|---|---|---|
| `:core` unit | JUnit (JVM) | Config validation & repair, migrations v1→vN, import of malformed JSON, search ranking, command parser, temporary-app expiry, motion token math, grid/snap/distribute math |
| Repository | Robolectric / instrumented | AppsRepository on package add/remove/update, icon cache invalidation, DataStore round-trip, history retention delete |
| UI | Espresso (Views) + Compose UI test | Home launch, drawer open/search/launch, folder create, Design Mode undo/redo, permission screens |
| Performance | Macrobenchmark | All PERFORMANCE_BUDGET scenarios |
| Static | Android Lint, detekt, manifest review | Exported components, permissions, main-thread I/O |

## Scenario matrix

| Scenario | Auto | Device |
|---|---|---|
| No launchable apps / one app / 400 apps | ✔ | ✔ |
| App installed, updated, removed, disabled, archived | ✔ | ✔ |
| Notification access granted / denied / revoked while running | ✔ | ✔ |
| Process killed in background, then Home pressed | ✔ | ✔ |
| Reboot with AURA as Home | — | ✔ |
| Low memory (`am send-trim-memory`) | ✔ | ✔ |
| Widget provider crashes / config activity never returns | ✔ (fake provider) | ✔ |
| Malformed or hostile imported config | ✔ | — |
| Wallpaper change: static, live, null colors | partial | ✔ |
| Theme / motion profile switch | ✔ | ✔ |
| AURA not default Home | ✔ | ✔ |
| Font scale 100 / 130 / 200 %, display size max | ✔ (screenshots) | ✔ |
| TalkBack navigation of Home, drawer, inbox | partial | ✔ |
| Rotation / large screen (sw ≥ 600dp) | ✔ | emulator |
| Switching back to the original launcher and uninstalling | — | ✔ |

## Device log

| Date | Device | OS | Build | What ran | Result |
|---|---|---|---|---|---|
| — | — | — | — | Nothing run on hardware yet | — |
