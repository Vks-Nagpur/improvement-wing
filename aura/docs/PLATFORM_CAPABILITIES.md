# AURA — Platform Capabilities

What a normal, installable Android app can and cannot do as a Home launcher.
Researched on 2026-10-05 against developer.android.com (links per row).

**Status key**

| Status | Meaning |
|---|---|
| SUPPORTED | Public API, no special permission |
| WITH PERMISSION | Public API, user must grant something |
| OEM DEPENDENT | Works on stock Android, vendors may change it — test on device |
| LIMITED | Partly possible; the full idea is not |
| NOT POSSIBLE | Not available to a normal third-party launcher |
| VERIFY | Could not confirm from an official page in this session — must be checked before we rely on it |

## 0. SDK baseline

| Fact | Source |
|---|---|
| Latest **stable** SDK: Android 16, API 36 ("Released to the stable channel … Platform Stability") | developer.android.com/tools/releases/platforms |
| Android 17 (API 37) is in **Beta** (partner devices, emulator images) | developer.android.com/about/versions/17/get |
| Google Play from 31 Aug 2026: new apps/updates must target API 36 | developer.android.com/google/play/requirements/target-sdk |

**Decision:** `compileSdk 36`, `targetSdk 36`, `minSdk 29` (Android 10, the level where `RoleManager.ROLE_HOME` exists). Keep an Android 17 test pass in the plan; do not target 37 until it is stable.

## 1. Feature-by-feature

| Area | Status | What is actually possible | Source |
|---|---|---|---|
| Become the Home app | SUPPORTED | Activity with `ACTION_MAIN` + `CATEGORY_HOME` + `CATEGORY_DEFAULT`. Ask with `RoleManager.createRequestRoleIntent(ROLE_HOME)` (API 29+); fallback `Settings.ACTION_HOME_SETTINGS`. User can switch back any time. | reference/android/app/role/RoleManager, provider/Settings |
| List installed apps | SUPPORTED | `LauncherApps.getActivityList(null, user)` for each profile from `getProfiles()`. Declare `<queries>` for `MAIN`/`LAUNCHER` intents (package visibility, API 30+). `QUERY_ALL_PACKAGES` not needed. | content/pm/LauncherApps, training/package-visibility |
| Package changes (install/update/remove/disable) | SUPPORTED | `LauncherApps.Callback` — event-driven, covers work profile too. | LauncherApps |
| Launch an app | SUPPORTED | `LauncherApps.startMainActivity(component, user, sourceBounds, opts)` | LauncherApps |
| Launch animation from the icon | SUPPORTED | `ActivityOptions.makeClipRevealAnimation` / `makeScaleUpAnimation` from the icon bounds. | app/ActivityOptions |
| Return-to-Home / swipe-up animation into the icon | LIMITED / OEM DEPENDENT | With gesture navigation the system owns the app-to-home transition. The rich "window shrinks into icon" effect of the built-in launcher is not offered to third-party launchers through a public API. Android 16 adds predictive back-to-home for apps targeting 36. Test on Realme. | about/versions/16/behavior-changes-16 (predictive back) |
| Recents / overview screen | NOT POSSIBLE (VERIFY on device) | No public API lets a third-party launcher provide or restyle Recents; it stays the system's. AURA's "Recent" dock uses AURA's own launch history (see Dock). | — (no public API found) |
| App shortcuts (long-press menu items) | SUPPORTED while default | `LauncherApps.getShortcuts` only works when `hasShortcutHostPermission()` is true (current default launcher). | LauncherApps |
| Pinned shortcuts / widgets from apps | SUPPORTED while default | Handle `LauncherApps.ACTION_CONFIRM_PIN_SHORTCUT` / `ACTION_CONFIRM_PIN_APPWIDGET`. | LauncherApps |
| App info, uninstall | SUPPORTED | App info: `LauncherApps.startAppDetailsActivity`. Uninstall: `Intent.ACTION_DELETE` with `package:` URI; `REQUEST_DELETE_PACKAGES` (normal permission) needed for the uninstall APIs on API 28+. System always shows its own confirm. | content/Intent, Manifest.permission |
| Private space (Android 15+) | WITH PERMISSION | Needs normal permission `ACCESS_HIDDEN_PROFILES` **and** the Home role. V1: support listing it only when both hold; otherwise hide. | LauncherApps, Manifest.permission |
| Archived apps (Android 15+) | SUPPORTED | `LauncherApps.ArchiveCompatibilityParams` exists; archived apps can appear — show them with an "unarchive" state. Design later. | LauncherApps |
| Widgets: host, add, resize, remove | SUPPORTED (bind needs one-time user OK) | `AppWidgetHost` + `AppWidgetManager.bindAppWidgetIdIfAllowed`; if false, launch `ACTION_APPWIDGET_BIND` so the user allows. Launch config activity with `startAppWidgetConfigureActivityForResult`. Call `deleteAppWidgetId` on removal. Handle config activities that never return. | guide/topics/appwidgets/host, appwidget/* |
| Widget crash isolation | SUPPORTED (our job) | Widget views are `RemoteViews` inflated in our process; we must catch inflation/apply errors per widget and show a "This widget stopped working" tile. Android 17 adds a RemoteViews bitmap memory limit for apps targeting 37. | about/versions/17/behavior-changes-17 |
| Wallpaper shown behind Home | SUPPORTED | Theme with `windowShowWallpaper`; parallax via `WallpaperManager.setWallpaperOffsets`. | app/WallpaperManager |
| Read wallpaper colors | SUPPORTED | `WallpaperManager.getWallpaperColors(which)` + `addOnColorsChangedListener`; gives primary/secondary/tertiary color and hints `HINT_SUPPORTS_DARK_TEXT`/`DARK_THEME`. Can be null for some live wallpapers. | WallpaperManager, WallpaperColors |
| Read the wallpaper **image** | NOT POSSIBLE | `getDrawable()` returns the default wallpaper on Android 13 and **always throws SecurityException from Android 14**; the new `getDrawable(which)` is meant for `MANAGE_EXTERNAL_STORAGE` holders only. **Spec §48 changes:** AURA uses `WallpaperColors`, not pixel analysis. | WallpaperManager |
| Display refresh rate | SUPPORTED (request only) | `Surface.setFrameRate` (API 30+) states an intended rate; the system decides. No forcing. | view/Surface |
| Clock updates | SUPPORTED | `ACTION_TIME_TICK` (every minute, runtime-registered only). Register while visible, unregister when not. | content/Intent |
| Battery level | SUPPORTED | `ACTION_BATTERY_CHANGED` is sticky, runtime-registered; register while visible only. | content/Intent |
| Read notifications | WITH PERMISSION | `NotificationListenerService`, user enables in Settings (`ACTION_NOTIFICATION_LISTENER_DETAIL_SETTINGS`, API 30+). Not bound on low-RAM devices running Android 10 or lower. | service/notification/NotificationListenerService |
| One-time codes in notifications | LIMITED | Android 15: untrusted listeners get **redacted** content when an OTP is detected. AURA shows "Code hidden by Android". | about/versions/15/behavior-changes-all |
| Dismiss a notification | WITH PERMISSION | `cancelNotification(key)`. Call it when the user taps a notification that has `FLAG_AUTO_CANCEL`. Ongoing ones cannot be cleared. | NotificationListenerService |
| Snooze ("Later") | WITH PERMISSION | `snoozeNotification(key, durationMs)` (API 26+): system removes it and re-posts it after the time. **No AURA alarm or service needed.** `getSnoozedNotifications()` lists them. Behaviour across reboot: VERIFY. | NotificationListenerService |
| Reply | WITH PERMISSION, app-dependent | Use the notification's own `Notification.Action` + `RemoteInput`. Only when the app provides one; otherwise "This notification doesn't support replies." | app/Notification |
| Other notification actions | WITH PERMISSION | Fire the app's own `PendingIntent`s. | app/Notification |
| Make an app/channel silent | LIMITED → open settings | `getNotificationChannels` needs a companion device association — not us. Open `Settings.ACTION_CHANNEL_NOTIFICATION_SETTINGS` / `ACTION_APP_NOTIFICATION_SETTINGS` instead. Never claim "Silenced". | NotificationListenerService, Settings |
| Block an app's notifications | LIMITED → open settings | Same as silent: route the user to the app's notification settings. | Settings |
| Do Not Disturb | LIMITED | Targeting API 35+, `requestInterruptionFilter` no longer changes the global filter; it toggles an app-owned `AutomaticZenRule`. Not in V1. | NotificationListenerService |
| Replace the system notification shade / heads-up | NOT POSSIBLE | AURA can only show its own in-app popup while AURA is in front. | — |
| Open the system notification shade from a gesture | NOT POSSIBLE via public SDK (VERIFY) | `EXPAND_STATUS_BAR` is a normal permission, but no public SDK method uses it; the method that does is hidden. We do not use reflection. Swipe down → AURA Search or Inbox. | Manifest.permission |
| Post AURA's own notifications | WITH PERMISSION | `POST_NOTIFICATIONS` runtime permission on API 33+. Only needed if we add our own reminders (not needed for Later). | Manifest.permission |
| Exact alarms | Avoid | `SCHEDULE_EXACT_ALARM` is denied by default for new installs targeting 33+; `USE_EXACT_ALARM` is for alarm/calendar apps only. AURA uses no exact alarms. | about/versions/14/changes/schedule-exact-alarms |
| Lock the screen (double-tap) | LIMITED — not in V1 | `DevicePolicyManager.lockNow` needs device admin with force-lock policy and then **requires PIN/pattern unlock (no fingerprint)**. `AccessibilityService.GLOBAL_ACTION_LOCK_SCREEN` (API 28) exists but the spec forbids Accessibility for this. V1: no double-tap lock. | app/admin/DevicePolicyManager, accessibilityservice |
| Status bar, Quick Settings, lock screen | NOT POSSIBLE | Owned by SystemUI. AURA only controls status-bar icon color (light/dark) over its own window. | — |
| System navigation bar / gestures | NOT POSSIBLE to change | Edge-to-edge is mandatory for API 36 apps. AURA must handle insets. | about/versions/16/behavior-changes-16 |
| Predictive back | SUPPORTED, required | Targeting 36: `onBackPressed` not called; use `OnBackPressedCallback`/`OnBackInvokedCallback`. Home must handle Back to close drawer/folders. | behavior-changes-16 |
| App usage (true "recent apps") | WITH PERMISSION (special access) | `UsageStatsManager` needs `PACKAGE_USAGE_STATS`, granted in Settings. Optional, not in V1. | app/usage/UsageStatsManager |
| Split screen launch | LIMITED | Can request `FLAG_ACTIVITY_LAUNCH_ADJACENT`; whether it works depends on device and window mode. Later. | — |
| Large screens | SUPPORTED, required | API 36: orientation/resize limits ignored on sw ≥ 600dp; Android 17 removes the opt-out. Layout must be adaptive. | behavior-changes-16, -17 |
| Memory | Watch | Android 17 adds per-app memory limits (`REASON_OTHER`, "MemoryLimiter"). Icon cache must be bounded. | behavior-changes-all (17) |

## 2. Sideloaded installs (APK, not Play)

| Item | Status |
|---|---|
| Installing an APK, choosing AURA as Home | SUPPORTED. Standard "install unknown apps" flow. |
| Enabling notification access for a sideloaded app | **VERIFY.** Android 13+ is widely reported to block "restricted settings" (notification access, accessibility) for apps installed outside an app store until the user taps *Allow restricted settings* in App info. The official help page could not be fetched from this environment. If confirmed, the Inbox permission screen must show these steps. |

## 3. Open questions to settle on the real phone

1. Realme UI: does gesture navigation work smoothly with a third-party Home? (Android 10 initially limited this; current behaviour on Realme UI must be tested.)
2. Does Realme UI keep AURA as default Home after reboot and after "phone manager" cleanups?
3. Does Realme's battery manager kill the notification listener? (Affects Inbox only, not Home.)
4. Snooze across reboot.
5. Restricted-settings flow for the sideloaded APK.
