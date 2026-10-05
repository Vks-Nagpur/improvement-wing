# AURA — Motion System

All motion comes from one place: `MotionTokens` in `:core`, read through `MotionEngine` in `:app`. No literal durations elsewhere (lint check in review).

## 1. Principles

1. **Touch first.** Visual response starts on ACTION_DOWN, in the same frame.
2. **The finger owns the surface.** Pages, drawer and sheets follow the finger 1:1 while dragging.
3. **Release velocity decides.** Settle with a spring seeded by the release velocity.
4. **Interruptible always.** A new touch catches a moving surface where it is; reversing direction reverses motion. No queued animations.
5. **Never block intent.** App launch starts immediately; decoration never delays it.
6. **Stop when done.** Animators end, callbacks are removed, no idle frame loops.

## 2. Tokens (Balanced profile)

| Token | Value | Use |
|---|---|---|
| `press.scale` | 0.94 | Icon press |
| `press.in` | 60 ms, linear-out | Down |
| `press.out` | spring stiffness 1500, damping 0.75 | Up |
| `quick` | 120 ms, fast-out-slow-in | Menus, Action Dock open/close |
| `standard` | 220 ms | Fades, Space switch cross-fade |
| `large` | 320 ms | Design Mode enter (workspace scales to 0.8) |
| `spring.page` | stiffness 700, damping 0.85 | Page settle |
| `spring.sheet` | stiffness 500, damping 0.9 | Drawer, Inbox, sheets |
| `spring.folder` | stiffness 600, damping 0.82 | Folder grows from its icon bounds |
| `fling.minVelocity` | 1000 dp/s | Page change threshold |
| `drag.threshold` | system touch slop | Start of drag |

## 3. Profiles (user-facing words)

| Profile | Effect |
|---|---|
| Off | No movement except finger tracking; changes are instant cuts. Also used when the system "Remove animations" setting is on. |
| Gentle | Durations ×1.2, damping 0.95 (no bounce) |
| Balanced | Table above |
| Playful | Damping −0.1 (small overshoot), Action Dock items stagger 15 ms |

Advanced (hidden under "More"): Speed (0.75×–1.5×), Bounce (0–1). Never shows stiffness/damping words.

## 4. Gesture-driven motion

- **Page swipe:** translationX = finger delta; on release, target = nearest page or next page if |v| > fling.minVelocity; spring.page from current velocity. Wallpaper offset updated per frame via `setWallpaperOffsets`.
- **Drawer:** progress = dragY / screenHeight; release up with v>0 or progress>0.35 → open. Home content fades/scales 1→0.96 behind it.
- **Folder:** opens from the folder icon's bounds (shared bounds transform), closes back to them; if the folder moved, closes to the new position.
- **Action Dock:** press = open in `quick`; slide finger to an action; release launches it. Lift without choosing = close.

## 5. Reduced motion

Respect `Settings.Global.ANIMATOR_DURATION_SCALE` = 0 and the Off profile: switch to cuts and opacity-only fades ≤ 100 ms where a cut would confuse location.

## 6. Haptics

Only for meaningful events, using system constants (`HapticFeedbackConstants`): enter Design Mode (LONG_PRESS), snap to guide (CLOCK_TICK / SEGMENT_TICK), drop complete (CONFIRM), Space switched (CONTEXT_CLICK), slider crosses a step (SEGMENT_FREQUENT_TICK where available). Respects the system haptics setting. Never on simple taps.

## 7. Performance fallback

If JankStats sees > 2 janky frames in one interaction on this device (debug builds: logged; release: counted locally), the next run of that interaction drops optional layers (parallax, shadows). Eco mode = this fallback always on.
