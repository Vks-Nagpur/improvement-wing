# AURA — OEM Notes

Primary test phone: **Realme Narzo 70 Turbo 5G** (≈1080 × 2400, 120 Hz, per the product brief).
Realme's own site could not be reached from the build environment, so the Android/Realme UI version on the user's phone must be read from *Settings → About phone* and written here.

| Field | Value |
|---|---|
| Android version | _fill from phone_ |
| Realme UI version | _fill from phone_ |
| Display refresh options offered by Realme | _fill from phone_ |

## Rules

- No device-specific hacks in core code. Anything Realme-specific lives behind a small `OemQuirks` interface with a no-op default, and only after a test on a real phone proves it is needed.
- Never claim a Realme test happened unless it did. Every row below starts as **Not tested**.

## Things that must be tested on the Realme phone

| # | Check | Why it matters | Result |
|---|---|---|---|
| 1 | Set AURA as Home via the role prompt and via Settings | Some OEM settings screens differ | Not tested |
| 2 | AURA stays default after reboot | ColorOS-family "phone manager" features have been known to reset defaults — unconfirmed | Not tested |
| 3 | Gesture navigation: swipe up to Home, swipe back, quick switch | Third-party launcher support with gestures varies | Not tested |
| 4 | Back-to-Home animation quality | System-owned; may look plainer than with Realme's launcher | Not tested |
| 5 | Notification listener stays connected overnight | Battery managers may stop it | Not tested |
| 6 | "Allow restricted settings" needed for notification access (sideloaded APK) | Affects Inbox onboarding text | Not tested |
| 7 | Refresh rate: does AURA get 120 Hz while animating, lower when idle? | Measure with frame stats, never assume | Not tested |
| 8 | Wallpaper colors returned for Realme's stock wallpapers / live wallpapers | `getWallpaperColors` may return null | Not tested |
| 9 | Realme icon packs / themed icons in `LauncherActivityInfo.getIcon` | Icon look and memory | Not tested |
| 10 | Switching back to Realme's launcher and uninstalling AURA | Safety rule §107 | Not tested |
| 11 | Battery screen after 24 h idle with AURA as Home | Release-blocking battery test | Not tested |

## Other vendors to test later

Pixel (reference behaviour), Samsung One UI, Xiaomi HyperOS, OnePlus/Oppo ColorOS, Motorola. One device per family before beta.
