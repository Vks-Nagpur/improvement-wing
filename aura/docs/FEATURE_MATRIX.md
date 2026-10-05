# AURA — Feature Matrix

Battery / performance risk: L = low, M = medium, H = high. Status words from PLATFORM_CAPABILITIES.md.

| Feature | Customer benefit | Implementation approach | Permission | Battery | Perf | Android limitation | V1 / Later |
|---|---|---|---|---|---|---|---|
| Default Home | AURA becomes the phone's Home | HOME intent filter + RoleManager prompt | Home role (user choice) | L | L | Return-to-home animation system-owned | V1 (M1) |
| App drawer + A–Z | Find any app fast | LauncherApps + RecyclerView, letter rail | — | L | M | — | V1 (M1) |
| Search (apps, AURA settings, Spaces, actions) | One place to find things | In-memory index in `:core`, prefix + initials + fuzzy ranking | — | L | L | — | V1 (M1) |
| Local commands | "Switch to Work" without menus | Deterministic grammar in `:core` | — | L | L | Only actions with public APIs/intents | V1 (M1 basic, M7 more) |
| Home pages + dock | Familiar base | Custom workspace View | — | L | M | — | V1 (M1) |
| Icon cache | Instant icons, low memory | Byte-bounded LRU, invalidate on package change | — | L | M | — | V1 (M1) |
| Motion Engine | Smooth, consistent feel | Tokens + springs, interruptible | — | L | M | No hacks on refresh rate | V1 (M2–M3) |
| Design Mode + precision tools | Arrange Home exactly | Edit layer over workspace, undo stack in `:core` | — | L | M | — | V1 (M4) |
| Grid / icon / spacing controls | Personal look | Config fields, live preview | — | L | L | — | V1 (M4) |
| Folders (sized, colored, grows from origin) | Better organisation | Overlay View with shared-bounds motion | — | L | M | — | V1 (M5) |
| Widgets (add, move, resize, remove) | Real Android widgets | AppWidgetHost, per-widget error isolation | Bind consent (system dialog) | Provider-dependent | M | Third-party widget quality varies | V1 (M5) |
| AURA widgets: Clock ×4, Battery | Beautiful defaults | Native views, TIME_TICK / BATTERY_CHANGED while visible | — | L | L | — | V1 (M5) |
| Calendar card | Next event on Home | CalendarContract on demand | Calendar (optional) | L | L | — | Later |
| Spaces | Different Home for Work/Travel | Config per Space, instant swap | — | L | M | Not Android user profiles | V1 (M6) |
| Temporary Home apps | "Keep until tomorrow" | Expiry in config, checked when Home shows | — | none | L | — | V1 (M7) |
| Clean Home | One tap to calm | Config overlay flag | — | none | L | — | V1 (M7) |
| Reachability / One-handed Home | Thumb-friendly | Layout preset + suggestions only | — | none | L | Only AURA's own UI | V1 (M7) |
| Action Dock | Quick actions near the thumb | Press-slide-release arc menu | — | none | L | Actions limited to public intents (camera, dial, etc.) | V1 (M7) |
| Hide apps | Less clutter | Config list; clear "not security" message | — | none | L | Not a security feature | V1 (M7) |
| Notification Inbox | Calmer notifications | NotificationListenerService → in-memory sections | Notification access | M | L | OTP redaction (A15+); restricted-settings step for sideload (VERIFY) | V1 (M8) |
| Notification history + search | Find an old notification | Room, retention, FTS | Notification access | L | L | Only what AURA saw while it had access | V1 (M8) |
| Later | Bring a notification back when wanted | `snoozeNotification` — Android re-posts | Notification access | none | L | Only clearable notifications | V1 (M8) |
| Reply | Answer without opening app | App's RemoteInput action | Notification access | L | L | Only if the app offers reply | V1 (M8) |
| Silent / Block | Control noisy apps | Open the right Android settings page | Notification access | none | L | Can't change channels directly | V1 (M8) |
| In-app notification popup | See messages while on Home | Small AURA card, only while AURA is visible | Notification access | L | L | Not system-wide heads-up | Later |
| Focus profiles | Fewer distractions | Space + Clean Home presets | — | none | L | Can't block system notifications | Later |
| Adaptive Home (time rules) | Right apps at the right time | Local rules evaluated when Home shows | — | none | L | No location tracking | Later |
| Gestures (swipe, pinch, two-finger) | Speed | Gesture detector on workspace | — | none | L | No public "open shade"; no legit double-tap lock in V1 | V1 basic (M3), more later |
| Double-tap to lock | Convenience | — | Device admin or Accessibility | — | — | lockNow forces PIN; Accessibility excluded by spec | Not in V1 |
| Themes (Aura, Minimal) | Instant look change | Serializable theme object | — | none | L | — | V1 (M9) |
| Wallpaper-aware text | Always readable | WallpaperColors hints, cached | — | none | L | Can't read wallpaper pixels (A14+) | V1 (M3) |
| Privacy Center | Trust | Compose screen from PRIVACY_MODEL | — | none | L | — | V1 (M8) |
| Battery & performance settings | Control | Eco / Balanced / Smooth | — | none | L | No forced refresh rate | V1 (M2) |
| Backup export/import | Safe switching phones | JSON of AuraConfig, validated | Storage Access Framework (no permission) | none | L | — | V1 (M9) |
| Share my setup | Share a Home design | Same JSON, minus private data | — | none | L | — | Later |
| Diagnostics panel (debug) | Find jank | Debug-only overlay | — | L (debug only) | L | — | V1 debug (M2) |
