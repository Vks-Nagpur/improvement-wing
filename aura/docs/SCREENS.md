# AURA — First Four Core Screens

Mockups: the "AURA Launcher — Core Screens" design canvas (8 artboards, 390 × 844 dp reference frame). All measurements in dp; text in sp.

---

## 1. Home

**Layout (top → bottom)**
1. Status-bar inset (system).
2. Today block, left-aligned, inset 28: date line (13 sp caps, +0.14em) → time (Instrument Serif 116) → context line "Clear, 24° — Standup at 10:00" (15 sp). Context line comes from local rules; empty when nothing useful.
3. **Breathing room** — wallpaper, untouched. Flexible height.
4. Space row: "● Personal" (tap = switch Space) left; page ticks right (active tick 18 wide, others 6).
5. Reachable apps: 4 × 2 grid, 58 dp icons, 13 sp labels, 18 row gap. Lives in the lower half by default (thumb zone).
6. **Action Dock** (72 tall, 16 side margin, veil): Search | Action ring | Recent (last 3 apps opened from AURA).
7. Gesture inset.

**Interactions**: swipe left/right pages (finger-tracked); swipe up anywhere below the clock → drawer; swipe down → Search (default) or Inbox (setting); long-press empty space → Design Mode (first time: one-line hint); pinch → Design Mode; two-finger horizontal → next Space; tap clock → system clock app; tap context line → the event or weather source app if installed.

**Action Dock**: press the ring → arc of six actions appears above it (design 2). Slide to one and release to launch; or tap the ring to open, tap an action. Close: release on nothing, tap ring, Back. Opens in 120 ms; the arc keeps 56 dp targets, ≥ 8 dp apart.

**Motion**: icon press 0.94 on down; launch uses clip-reveal from icon bounds; page settle spring; Space switch = 220 ms cross-fade + 8 dp slide.

**Empty state**: first run puts Phone, Messages, Camera, Browser (whatever resolves for those intents) in the grid; the rest of Home is wallpaper — intentional.

**Accessibility**: every icon is a button with the app name; Space button reads "Space: Personal. Double-tap to switch Space"; page ticks read "Page 1 of 3"; non-gesture alternatives: Search button, "Customize" in the long-press menu and in Settings, Space switcher button.

**Light / dark / wallpaper**: Home text color comes from `WallpaperColors` (`HINT_SUPPORTS_DARK_TEXT` → dark ink). Unknown colors → soft scrim behind clock and labels. The design canvas Home artboard has a Dusk/Daylight switch showing both.

---

## 2. App Drawer / Search

**Drawer layout** (design 3): sheet over Home; handle → "Apps 84" + options → tabs All / Recent / Favorites (underline in signal color) → "OFTEN OPENED" row (4) → A–Z list with serif letter headers and a right letter rail → **search field anchored at the bottom**, above the gesture area.

**Why bottom search**: it is where the thumb is, and results grow upward so the best match sits right above the field (design 4). The keyboard rises under it; no reach to the top.

**Search** (design 4): typing filters instantly (in-memory index). Order from bottom up: **Best match** (highlighted, Enter launches it) → other apps → AURA settings → actions/commands (e.g. "Open battery settings", "Battery & performance", "Battery widget"). With an empty field, shows command hints in serif italic: "Switch to Work", "Turn on Clean Home", "Show hidden apps".

**Interactions**: swipe down on list top or Back closes; letter rail drag jumps with haptic tick per letter; long-press an app → quick actions menu; drag an app upward out of the drawer → place it on Home.

**Motion**: sheet follows finger; spring.sheet on release; Home scales 1 → 0.96 behind.

**Empty states**: no match → "No apps match "xyz"." + "Search settings for "xyz"" action. Hidden apps never appear unless the user types the command "Show hidden apps".

**Accessibility**: list rows 52 dp; letter rail also reachable as a "Jump to letter" control; search field has a label; results announced as a count when they change (polite live region, not an announcement event — deprecated in Android 16).

**Theme**: panel colors follow the system light/dark setting; signal color adjusts for contrast.

---

## 3. Notification Inbox

**Layout** (design 5): "Inbox" (serif 44) + history search + settings. Sections:
- **NOW** — cards for the newest important items. Conversation card shows sender, app, time, message, then **Reply** (only if supported) and **Later**; "•••" holds the rest.
- **PEOPLE** — conversation rows with initial avatars, last line, time.
- **QUIET** — one block: app name + count rows ("Shopping 4", "Video 2", "Social 3"); tap expands.
- **EARLIER** — muted rows (scroll).
- Footer: "History is kept on this phone for 7 days. Nothing is uploaded."

**Later** (design 6): bottom sheet "Bring this back…" with the notification preview, choices with exact times (30 min, 1 hour, This evening, Tomorrow morning, Pick a time). Uses Android's snooze, so it truly leaves the shade and returns. If the notification can't be snoozed (ongoing), the button isn't shown.

**Permission** (design 7): shown on first open without access. Title "Let AURA organize your notifications", why, what happens if you say no, **Not now** / **Continue** → Android's notification-access page for AURA. If the sideload "restricted settings" block is confirmed on the phone, this screen gets a short 2-step guide.

**Interactions**: swipe a card right → Later sheet; swipe left → clear (only if clearable); tap → open via the notification's own intent (and clear if auto-cancel); long-press → "•••" menu: Snooze, Silent…, Block…, Notification settings, App info — Silent/Block open Android's page, wording "Opens Android settings".

**Empty states**: access but nothing → "You're all caught up." + "Search history" link. History off → search icon hidden.

**Accessibility**: each card is one focusable item with actions exposed as accessibility actions (Reply, Later, Clear); counts read as "Shopping, 4 notifications".

**Theme**: panel follows system theme; OTP-redacted content shows "Code hidden by Android".

---

## 4. Design Mode

**Enter**: long-press empty Home or pinch. Workspace scales to 0.8 and moves up (320 ms), a thin signal outline marks the editable area, haptic LONG_PRESS (design 8).

**Layout**: top bar Undo / Redo / "Design" / **Done**. Scaled live Home. Bottom panel tabs: Layout, Icons, Dock, Clock, Widgets, Wallpaper, Motion.
Layout tab: Apps per row (− 4 +), Icon size (Small–Large, live), Space between apps (live, numeric), Show app names.

**Direct manipulation**: tap an item → selected (outline + 4 handles); drag → smart guides (dashed signal lines) when centers or edges align, live distance label ("16") to neighbours; snap with tick haptic; multi-select with a second finger tap; precision row appears above the panel when 2+ items are selected: Align left/center/right, Distribute horizontally/vertically, Equal spacing.

**Undo / Redo**: every change is a command in a `:core` stack; Done saves; Back asks "Keep changes?" only if something changed. Reset is under the panel's "•••".

**Motion**: all edits animate with `quick`; guides appear instantly (no fade) so they never lag the finger.

**Empty states**: Widgets tab with nothing added → "Add a widget" button.

**Accessibility**: every selected item has accessibility actions "Move left/right/up/down", "Make bigger/smaller"; steppers instead of only sliders.

**Theme**: panel uses dark panel tokens in both themes so the preview reads as the focus.

---

## Self-critique

| Question | Answer |
|---|---|
| Does this look like a generic Android launcher? | The grid and drawer are familiar on purpose. What makes it AURA: serif editorial clock with a context line, wallpaper breathing room, Space label + ticks, Action Dock ring, bottom-anchored search with best match next to the thumb, serif A–Z index, sectioned Inbox with a Quiet group. A screenshot is recognisable without a logo. |
| Is anything different only for the sake of it? | Checked each unusual choice for a practical reason. The arc menu is used only in the Action Dock, where it serves thumb reach; the drawer stays a plain, fast-to-scan A–Z list. Bottom search shortens reach and puts the best match beside the field. A circular or radial drawer was rejected for being slow to scan. |
| One-week test risks | Serif clock size may feel large — Clock tab offers Compact. Action Dock may feel unnecessary for some — one switch replaces it with a normal dock. |
