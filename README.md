<div align="center">

# ⚡ FREENET

### Панель управления для обхода блокировок: zapret, GoodbyeDPI, ByeDPI и tg-ws-proxy

![Banner](https://img.shields.io/badge/СДЕЛАНО_НА-ULTRAVIOLET-7C3AED?style=for-the-badge&labelColor=0a0118)

</div>

<div align="center">

[![Release](https://img.shields.io/github/v/release/impersonalll/FreeNet?style=for-the-badge&logo=github&color=7C3AED)](https://github.com/impersonalll/FreeNet/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/impersonalll/FreeNet/total?style=for-the-badge&logo=download&color=8B5CF6)](https://github.com/impersonalll/FreeNet/releases)
[![Stars](https://img.shields.io/github/stars/impersonalll/FreeNet?style=for-the-badge&logo=star&color=a78bfa&labelColor=1e1b2e)](https://github.com/impersonalll/FreeNet)
[![Forks](https://img.shields.io/github/forks/impersonalll/FreeNet?style=for-the-badge&logo=git-fork&color=6d28d9&labelColor=1e1b2e)](https://github.com/impersonalll/FreeNet/fork)
[![Last commit](https://img.shields.io/github/last-commit/impersonalll/FreeNet?style=for-the-badge&logo=git&color=7C3AED)](https://github.com/impersonalll/FreeNet/commits/main)

</div>

<div align="center">

[![Tauri](https://img.shields.io/badge/Tauri-v2-24C8DB?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-2021-CE422B?style=flat-square&logo=rust&logoColor=white)](https://rust-lang.org)
[![React](https://img.shields.io/badge/React-19-61DAFB?style=flat-square&logo=react&logoColor=white)](https://react.dev)
[![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?style=flat-square&logo=typescript&logoColor=white)](https://typescriptlang.org)
[![Tailwind](https://img.shields.io/badge/Tailwind_CSS-3-06B6D4?style=flat-square&logo=tailwindcss&logoColor=white)](https://tailwindcss.com)
[![Vite](https://img.shields.io/badge/Vite-6-646CFF?style=flat-square&logo=vite&logoColor=white)](https://vitejs.dev)
[![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)](LICENSE)

</div>

---

## ✨ Возможности

<div align="center">

| | |
|:---:|:---|
| 🚀 **Один клик — всё работает** | Включение и остановка всей системы обхода одной кнопкой с главного экрана |
| 🔧 **BYPASS Hub** | Установка, выбор и запуск zapret / GoodbyeDPI / ByeDPI — они взаимно исключают друг друга, tg-ws-proxy работает параллельно |
| 🌐 **Свои домены для zapret** | Добавляй свои домены в `list-general-user.txt` прямо из интерфейса |
| 🔄 **Авто-обновление** | Приложение само проверяет и устанавливает новые версии с GitHub |
| 🤫 **Тихий запуск** | `winws.exe` стартует без окон CMD |
| 📊 **Мониторинг** | Живой статус всех процессов каждые 3 секунды |
| 🔔 **Системный трей** | Сворачивается в трей, а не закрывается |
| 👑 **UAC** | Авто-повышение прав при старте для поддержки WinDivert |
| 🎹 **Горячие клавиши** | Глобальные медиа-кнопки для музыки из любого окна |
| 📋 **Clipboard Manager** | Буфер обмена с историей и быстрой вставкой |
| 🛡 **Обход через hosts** | Опциональная модификация файла hosts |

</div>

## 🧰 Обходы

| Сервис | Тип | Репозиторий | Примечание |
|--------|-----|-------------|------------|
| **zapret** | DPI desync | [Flowseal/zapret-discord-youtube](https://github.com/Flowseal/zapret-discord-youtube) | Стратегии, свои списки доменов, выбор версии |
| **GoodbyeDPI** | DPI bypass | [ValdikSS/GoodbyeDPI](https://github.com/ValdikSS/GoodbyeDPI) | Простой запуск одной кнопкой |
| **ByeDPI** | DPI bypass | [hufrea/byedpi](https://github.com/hufrea/byedpi) | Простой запуск одной кнопкой |
| **tg-ws-proxy** | WebSocket-прокси | [Flowseal/tg-ws-proxy](https://github.com/Flowseal/tg-ws-proxy) | Для Telegram, работает вместе с любым DPI |

> ⚠️ zapret, GoodbyeDPI и ByeDPI обходят DPI-фильтрацию (Discord, YouTube, Spotify...) и не могут работать одновременно — выбирается только один.

## 🗂 Структура проекта

```
freenet-app/
├── src-tauri/
│   └── src/lib.rs            — вся логика бэкенда на Rust
└── src/
    ├── App.tsx               — главный layout и маршрутизация вкладок
    └── components/
        ├── FreenetPage.tsx   — кнопка питания, индикаторы сервисов
        ├── BypassPage.tsx    — BYPASS Hub: установка/выбор/запуск обходов
        ├── SettingsPage.tsx  — hosts-обход и пути
        ├── PluginsPage.tsx   — медиа-клавиши, буфер обмена и плагины
        ├── NavBar.tsx        — навигация со стеклянной анимацией
        ├── TitleBar.tsx      — кастомный заголовок окна
        ├── StatusBar.tsx     — нижняя панель со статусом
        └── Toast.tsx         — уведомления
```

## 🔨 Сборка из исходников

Требования: [Node.js](https://nodejs.org) 18+, [Rust](https://rustup.rs) (stable), [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/).

```bash
# Установка зависимостей
npm install

# Режим разработки
npm run tauri dev

# Продакшн-сборка
npm run tauri build
```

Результат: `src-tauri/target/release/bundle/`

## 📥 Установка

Скачай последний релиз со [страницы релизов](https://github.com/impersonalll/FreeNet/releases/latest) и запусти `freenet-app.exe`.

> Приложение запросит права администратора — это необходимо для работы WinDivert и модификации hosts.

## 🧪 Стек

```
Бэкенд:     Rust + Tauri v2
Фронтенд:   React 19 + TypeScript + Tailwind CSS 3
Сборка:     Vite 6
Дизайн:     «Ultraviolet Vision» — стеклянный градиент, шрифт Sora
```

## 📜 Лицензия

MIT — свободно используй, модифицируй и распространяй.

---

<div align="center">

**FREENET** — сделано с 💜

[![GitHub](https://img.shields.io/badge/GitHub-impersonalll/FreeNet-7C3AED?style=social&logo=github)](https://github.com/impersonalll/FreeNet)

</div>
