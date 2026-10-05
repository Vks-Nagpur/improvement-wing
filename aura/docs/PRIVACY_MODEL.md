# AURA — Privacy Model

Default: **private data stays on the phone.** V1 has no INTERNET permission, no account, no ads, no analytics.

## Permissions and data

| Permission / access | Why AURA wants it (user words) | If you say no | Data stored | Retention | Leaves device? | How to delete | After revoking |
|---|---|---|---|---|---|---|---|
| Home role | "Make AURA your Home screen" | AURA works as a normal app only | — | — | No | Switch Home in Settings | Your old Home comes back |
| Notification access | "Let AURA organize your notifications" | Home, apps and search work; Inbox stays empty | Active list in memory; history rows only if History is on: app, title, text, time, key | Off / 24 h / **7 days (default when turned on)** / 30 days | Never | Delete one, Clear history, Turn off history (deletes all) | Listener disconnects; in-memory list cleared; history kept until user clears it, with a banner offering to delete |
| Widget binding | "Allow AURA to add widgets" | Widgets can't be added | Widget IDs + positions | Until removed | No | Remove widget | — |
| Calendar (later) | "Show your next event on Home" | Calendar card hidden | Nothing persisted; read on demand | — | No | — | Card hidden |
| Contacts (later) | "Find people from search" | Search skips contacts | Nothing persisted | — | No | — | Search skips contacts |
| Usage access (maybe later) | "Use your real recent apps in the dock" | Dock uses apps opened from AURA | Nothing persisted | — | No | — | Falls back |
| `REQUEST_DELETE_PACKAGES` (normal, auto-granted) | Lets the "Uninstall" button open Android's uninstall prompt | — | — | — | — | — | — |

## Other stored data

| Data | Where | Notes |
|---|---|---|
| Layout, Spaces, theme, hidden apps, temporary apps | DataStore (app-private) | Included in backup export |
| AURA launch history (for "Recent" dock) | DataStore, last 20 entries | Not exported |
| Local crash note | App-private file, last 3 | Never uploaded; user can view/delete in About |

## Rules

- Notification content is never logged (release or debug).
- Backup export never contains notification history unless a future, explicit, separate option is added.
- Hidden apps: UI always states "This hides the app from AURA. It does not uninstall or secure the app."
- Android 15+ hides one-time codes from AURA; we show "Code hidden by Android" and do not try to work around it.
