# AURA

*Your phone. Your way.*

AURA is an Android Home screen (launcher) app: a normal app you install and choose as your Home screen. It does not need root, an unlocked bootloader, flashing or a custom ROM, and it never changes system files.

**Current status:** planning and design (Milestone 0). There is no APK yet. See `docs/PROJECT_PLAN.md`.

## Documents

| File | What it covers |
|---|---|
| docs/PROJECT_PLAN.md | Milestones, changes from the brief, repo structure |
| docs/ARCHITECTURE.md | How the app is built |
| docs/DESIGN_SYSTEM.md | Type, color, spacing, states |
| docs/MOTION_SYSTEM.md | Motion tokens, gestures, haptics |
| docs/SCREENS.md | Home, Drawer/Search, Inbox, Design Mode in detail |
| docs/PERFORMANCE_BUDGET.md | Measurable speed targets |
| docs/BATTERY_BUDGET.md | Every piece of background work, justified |
| docs/PLATFORM_CAPABILITIES.md | What Android allows, with sources |
| docs/PRIVACY_MODEL.md | Permissions, stored data, deletion |
| docs/TEST_PLAN.md | Tests and device log |
| docs/OEM_NOTES.md | Realme and other vendor checks |
| docs/FEATURE_MATRIX.md | Every feature: benefit, cost, limits, V1 or later |

## Installing (when an APK exists)

1. Copy `app-debug.apk` to the phone and open it. Allow "Install unknown apps" for the app you opened it with when Android asks.
2. Open AURA → **Make AURA your Home screen** → choose AURA. Your current launcher is not deleted.

## Switching back

Settings → Apps → Default apps → Home app → choose your previous launcher. (Menu names vary by phone maker.)

## Uninstalling

Switch back first (above), then Settings → Apps → AURA → Uninstall. If AURA is still the Home app, Android picks another Home app automatically.

## Optional permissions

| Permission | Used for | If you say no |
|---|---|---|
| Notification access | Inbox, history, Later | Everything else works |
| Allow widgets | Adding widgets | No widgets |
| Calendar (later) | Next event on Home | Card hidden |

## Known limits

Recents, status bar, Quick Settings and the lock screen belong to Android and cannot be replaced by AURA. Details: `docs/PLATFORM_CAPABILITIES.md`.
