# Changelog

## v1.1.0 - 2026-08-05

### New: Bypass Hub
- New **BYPASS** tab that replaces the old Downloads tab — install, select and run zapret, GoodbyeDPI and ByeDPI from one place.
- zapret, GoodbyeDPI and ByeDPI are mutually exclusive: you can run only one DPI bypass at a time. tg-ws-proxy runs alongside any of them.
- Zapret is selected by default.
- One-button start/stop for the whole system from the main page.

### New: Zapret custom domains
- Added the ability to add your own domains to zapret's user list (`list-general-user.txt`).
- Added and removed domains are applied automatically via zapret's own list loading.

### UI / Polish
- Downloads tab removed — everything moved into the BYPASS tab.
- Settings tab reduced to the essentials (hosts + paths).
- Removed the plain white input borders — replaced with a stylistic purple accent.
- Status bar now shows the currently active bypass.
- Bypass cards restyled like the old Downloads widgets (INSTALL / UPDATE / REINSTALL actions).
- Various visual cleanup and consistency fixes across all tabs.

### Fixes
- Fixed duplicate tg-ws-proxy indicator on the main page.

## v1.0.0 - 2026-07-31

- Initial Tauri v2 rewrite.
- Auto-update on startup, download + restart when a new version is available.
- Single-instance guard, double-click maximize, custom title bar.
- Custom download directory + loading screen.
