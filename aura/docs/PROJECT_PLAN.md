# AURA — Project Plan

Status: **Milestone 0 (research + design) complete. Waiting for approval to start Milestone 1.**

## What changed from the brief after research

| Brief said | Platform reality | Plan |
|---|---|---|
| Analyse the wallpaper image when it changes (§48) | Android 14+ does not let apps read the wallpaper image | Use `WallpaperColors` + hints (official API), cache it |
| Later should "surface the reminder" (§43) | `snoozeNotification` makes Android re-post the notification at the time | Use it; no AURA alarm, no service, no extra permission |
| Silent / Block (§42) | Changing another app's channels needs a companion-device association | Open the exact Android settings page instead |
| Swipe down → notification shade option (§52) | No public SDK method to open the shade | Swipe down opens AURA Search or Inbox |
| Double-tap to lock (§52) | Only device admin `lockNow` (forces PIN, no fingerprint) or Accessibility (excluded) | Not in V1; documented |
| "Recent" dock mode (§30) | Real recent-app list needs Usage Access special permission | V1 uses apps opened from AURA; optional Usage Access later |
| Notification access for an APK installed outside a store | Probably needs "Allow restricted settings" on Android 13+ (to verify on the phone) | Inbox permission screen will guide if confirmed |

## Milestones

| # | Name | Deliverable | Exit check |
|---|---|---|---|
| 0 | Research + design | These docs + design canvas | **Done — your approval needed** |
| 1 | Foundation | Home activity, app repository, icon cache, Home grid, dock, drawer, search, app launch, DataStore config | Debug APK installs, can be default Home, launches apps, survives process death |
| 2 | Performance foundation | Motion tokens, JankStats (debug), Macrobenchmark + Baseline Profile module, diagnostics panel | Benchmarks run; numbers recorded in PERFORMANCE_BUDGET |
| 3 | Premium motion | Icon press, page physics, drawer, folder, Design Mode transition, wallpaper-aware text | Budgets met on device |
| 4 | Design Mode | Grid, icon size, spacing, labels, dock; undo/redo/reset; then guides, snap, align, distribute | Unit tests on all layout math |
| 5 | Folders + widgets | Folders, drag & drop, AppWidgetHost, resize, crash isolation, AURA clock/battery widgets | Faulty-widget test passes |
| 6 | Spaces | Personal/Work/Study/Travel, switching, independent layouts | Space switch benchmark |
| 7 | Special features | Temporary Home apps, Clean Home, Reachability, Action Dock, Hide apps, more commands | Each can be turned off |
| 8 | Notifications | Inbox, history + search, Later, Reply, Clear, settings routing, Privacy Center | Revocation tests pass; Home unaffected without access |
| 9 | Polish | Themes, backup/import, accessibility, font scale, copy review, onboarding | Audit checklist |
| 10 | Release candidate | Clean build, tests, benchmarks, battery check, release APK | Your approval |

Each milestone ends with: compile → tests → lint → APK → report (works / tested / remaining / limits / APK path).

## Build environment note

This cloud environment can reach Maven Central, Google Maven and Gradle, but **not dl.google.com**, where the Android SDK platform packages are downloaded. So:
- **Recommended:** build the APK on GitHub Actions and download the APK artifact from the run (GitHub's Ubuntu runner images list an Android SDK among preinstalled tools — confirm when the workflow is added; otherwise the workflow installs it); or
- allow `dl.google.com` in this environment's network settings so builds can run here too.

## Repository structure (proposed)

```
aura/
├── README.md                 install, set as Home, switch back, uninstall, permissions, OEM limits
├── settings.gradle.kts
├── build.gradle.kts
├── gradle/libs.versions.toml
├── app/                      :app  (Activities, Views, Compose, services)
│   └── src/main/java/app/aura/
│       ├── home/  drawer/  search/  folders/  widgets/  notifications/
│       ├── spaces/  themes/  settings/  permissions/  performance/  design/
│       └── AuraApplication.kt
├── core/                     :core (pure Kotlin model, validation, search, commands, motion tokens)
├── benchmark/                :benchmark (Macrobenchmark + Baseline Profile)
└── docs/                     the documents in this folder
```
