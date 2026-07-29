<div align="center">

# FREENET

### Панель управления для zapret и tg-ws-proxy

![Tauri](https://img.shields.io/badge/Tauri-v2-blue?logo=tauri)
![React](https://img.shields.io/badge/React-19-61DAFB?logo=react)
![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?logo=typescript)
![Rust](https://img.shields.io/badge/Rust-2021-CE422B?logo=rust)
![License](https://img.shields.io/badge/License-MIT-green)

</div>

---

## Что это

Desktop-приложение для управления сетевыми сервисами обхода блокировок. Один клик — и все сервисы запущены.

- **zapret** — DPI desync для обхода блокировок Discord, YouTube и других сервисов
- **tg-ws-proxy** — WebSocket-прокси для Telegram

## Возможности

| Функция | Описание |
|---------|----------|
| **Однокнопочный запуск** | Включение/выключение всех сервисов одной кнопкой |
| **Авто-загрузка** | Скачивание последних релизов прямо с GitHub |
| **Тихий запуск** | winws.exe запускается напрямую — никаких окон CMD |
| **Мониторинг** | Отслеживание статуса процессов каждые 3 секунды |
| **Системный трей** | Сворачивается в трей, а не закрывается |
| **Авто-повышение прав** | Запрос UAC при старте для поддержки WinDivert |
| **Обход через hosts** | Опциональная модификация файла hosts |

## Стек технологий

```
Бэкенд:     Rust + Tauri v2
Фронтенд:   React 19 + TypeScript + Tailwind CSS 3
Сборка:     Vite 6
Дизайн:     "Ultraviolet Vision" — стеклянный градиент, шрифт Sora
```

## Структура проекта

```
src-tauri/src/lib.rs          — Вся логика бэкенда
src/App.tsx                   — Главный layout, 3 вкладки
src/components/
  ├── FreenetPage.tsx         — Кнопка питания, индикаторы статуса
  ├── DownloadsPage.tsx       — Карточки скачивания сервисов
  ├── SettingsPage.tsx        — Настройки: bat-файл, версия, hosts
  ├── NavBar.tsx              — Навигация со стеклянной анимацией
  ├── TitleBar.tsx            — Кастомный заголовок окна
  └── StatusBar.tsx           — Нижняя панель со статусом
```

## Сборка

```bash
# Установка зависимостей
npm install

# Режим разработки
npm run tauri dev

# Продакшн-сборка
npm run tauri build
```

Результат: `src-tauri/target/release/bundle/`

## Сервисы

| Сервис | Репозиторий | Назначение |
|--------|-------------|------------|
| zapret | [Flowseal/zapret-discord-youtube](https://github.com/Flowseal/zapret-discord-youtube) | DPI desync для Discord/YouTube |
| tg-ws-proxy | [Flowseal/tg-ws-proxy](https://github.com/Flowseal/tg-ws-proxy) | WebSocket-прокси для Telegram |

## Лицензия

MIT
