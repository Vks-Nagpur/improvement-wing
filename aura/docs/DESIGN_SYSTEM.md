# AURA — Design System

Identity in one line: **an editorial instrument laid over your wallpaper.**
Large serif time, quiet grotesk text, one warm signal color, the wallpaper left visible.

Visual reference: the 8-screen design canvas (Home, Action Dock, Drawer, Search, Inbox, Later, Permission, Design Mode).

## 1. Typography

| Role | Face | Size / line | Use |
|---|---|---|---|
| Display time | Instrument Serif (OFL) | 116 / 0.88 | Home clock only |
| Screen title | Instrument Serif | 44 / 1.0 | Inbox, Design |
| Letter index | Instrument Serif | 30 | Drawer A–Z headers |
| Command hints | Instrument Serif italic | 19 | Search suggestions |
| Title | Schibsted Grotesk 600 | 28–30 | Drawer "Apps", sheets |
| Body | Schibsted Grotesk 400/500 | 16–17 / 1.4 | Rows, messages |
| Label | Schibsted Grotesk 500 | 13 | Icon labels (min 12) |
| Section mark | Schibsted Grotesk 600, +0.14em, caps | 12–13 | NOW, PEOPLE, QUIET |

Both fonts ship inside the APK (offline). All sizes are `sp` and scale with the user's font size; layouts are tested at 130 % and 200 %.

## 2. Spacing

Scale: **4, 8, 12, 16, 24, 32, 48**. Screen side margin 24 (content) / 16 (dock, search field, cards). Optical corrections allowed and noted in code (e.g. the clock is inset 28 because serif numerals carry visual side-bearing).

## 3. Color

| Token | Dark | Light (bright wallpaper) | Use |
|---|---|---|---|
| `ink` | #F6F1E7 | #1B1712 | Primary text on wallpaper / panels |
| `ink2` | #E2D9C9 | #4A4337 | Secondary text |
| `muted` | #A39A8A | #5E564A | Meta text (must still pass 4.5:1 on its surface) |
| `signal` | #F2A93B | #9A5B00 | The one accent: selection, Space dot, Action ring, Best match |
| `panel` | #14110D | #F7F3EC | Drawer, Inbox, Search |
| `raised` | #221E18 | #FFFFFF | Cards, fields |
| `hairline` | #2A241D | #E3DCD0 | Dividers |
| `veil` | rgba(18,14,12,.42) | rgba(255,250,240,.62) | Dock over wallpaper only |

Warm neutrals instead of grey/black; one accent; no neon, no gradients in UI (gradients appear only as the user's wallpaper).
Light/dark on Home follows `WallpaperColors` hints, not the system theme. Panels follow the system theme.

## 4. Icons

- App icons: system adaptive icons, mask **squircle radius ≈ 33 %** of size (19 px on 58 px). Default 58 dp, range 40–72.
- UI glyphs: 1.8 px stroke, round caps, 24 grid. No filled glyph sets, no emoji.

## 5. Corners

Small things are rounder, big things are calmer: icons 33 %, buttons/fields 18, cards 22, sheets 30 (top only). Never a pill unless it is a real toggle.

## 6. Surface hierarchy

1. Wallpaper (always the hero on Home)
2. Text directly on wallpaper (no box) — readability from wallpaper hints
3. Veil (dock only)
4. Panel (full screens: drawer, inbox)
5. Raised (cards, fields, sheet)
No blur by default. Blur, if added, only in Smooth mode and only for the drawer backdrop, measured first.

## 7. Contrast over wallpaper

Text on wallpaper uses `ink` chosen from `HINT_SUPPORTS_DARK_TEXT`. If colors are unknown (null), add a soft 24 % scrim behind the clock and app labels. Target 4.5:1 for labels; the scrim strength is computed from the cached wallpaper primary color.

## 8. Touch targets

Minimum 48 dp for anything tappable (44 px drawn rows in the mockups are padded to 48 dp hit areas). Icon hit area = tile + label.

## 9. States

| State | Look |
|---|---|
| Pressed | Scale 0.94 + slight darken, starts on ACTION_DOWN |
| Focused (keyboard/a11y) | 2 dp `signal` outline, 3 dp offset |
| Selected (Design Mode) | `signal` outline + 4 corner handles |
| Disabled | 40 % opacity, no outline |
| Dragging | Lift: scale 1.06, shadow 8 dp |

## 10. Empty states

Plain sentence + one action. Examples: Drawer search with no result → "No apps match 'xyz'. Search settings instead?" Inbox with access but nothing new → "You're all caught up." Inbox without access → permission screen (design 7).

## 11. Contextual menus (app long-press)

A compact list anchored to the icon, opening toward screen center: app shortcuts first (when default Home), then **Keep on Home…**, Add to folder, Hide, Remove from Home, App info, Uninstall (last, separated). Never more than 7 visible rows.

## 12. Notification presentation

- Sections: **Now** (cards), **People** (rows with initial avatars), **Quiet** (one grouped block, counts per app), **Earlier** (muted rows).
- Primary actions visible: Reply (if the app supports it), Later. Everything else under "•••": Snooze, Silent…, Block…, Notification settings, App info.
- In-app popup (while AURA is in front only): one card, top of screen, swipe up to dismiss, auto-hides after 6 s.

## 13. Self-critique

- *Generic launcher?* No: serif clock, letter-indexed drawer, bottom-anchored search, Action Dock ring and Space label are not found together in stock launchers.
- *Different only to be different?* Each choice has a reason: serif clock = recognisable at a glance; search at bottom = thumb reach + nearest result next to the field; Quiet group = less noise; one accent = calm.
