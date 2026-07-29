# FREENET

Desktop panel for managing **zapret** and **tg-ws-proxy** services. Built with Tauri v2 + React + TypeScript.

## Features

- **One-click toggle** — start/stop all services from a single power button
- **Auto-download** — fetches latest releases from GitHub (zapret v1.10.0, tg-ws-proxy v1.9.0)
- **Silent launch** — winws.exe runs directly without CMD windows
- **Process monitoring** — real-time status polling every 3 seconds
- **Tray minimization** — closes to system tray, not exit
- **Admin elevation** — auto-requests UAC on startup for WinDivert support
- **Hosts bypass** — optional hosts file modification for blocked domains

## Tech Stack

- **Backend:** Rust (Tauri v2)
- **Frontend:** React 19, TypeScript, Tailwind CSS 3, Vite 6
- **Design:** "Ultraviolet Vision" — deep violet glassmorphism, Sora font

## Building

```bash
# Install dependencies
npm install

# Dev mode
npm run tauri dev

# Production build
npm run tauri build
```

Output: `src-tauri/target/release/bundle/`

## Structure

```
src-tauri/src/lib.rs   — All backend logic (commands, process mgmt, downloads)
src/App.tsx            — Main layout, 3 pages
src/components/
  FreenetPage.tsx      — Power button, status indicators, bat file selector
  DownloadsPage.tsx    — Service download cards with progress
  SettingsPage.tsx     — Config: bat file, release version, hosts bypass
  NavBar.tsx           — Telegram-style sliding glass border
  TitleBar.tsx         — Custom window titlebar
  StatusBar.tsx        — Footer with connection status
```

## Services

| Service | Repo | Purpose |
|---------|------|---------|
| zapret | [Flowseal/zapret-discord-youtube](https://github.com/Flowseal/zapret-discord-youtube) | DPI desync for Discord/YouTube |
| tg-ws-proxy | [Flowseal/tg-ws-proxy](https://github.com/Flowseal/tg-ws-proxy) | Telegram WebSocket proxy |

## License

MIT
