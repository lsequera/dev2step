# Dev2Step

> A local-first, zero-friction project board for solo developers.

Dev2Step is a lightweight task-management tool that pairs a **plain-text `todo.txt` file** (your Single Source of Truth) with a **SQLite cache** for fast queries and history — all wrapped in a keyboard-driven Tauri desktop app and a full-featured Rust CLI.

---

## Features

- **Text-first sync** — edit `todo.txt` in any editor; the app reloads automatically
- **Kanban board UI** — Icebox → Todo → In Progress → Done with WIP-limit warnings
- **Keyboard-driven** — navigate and move tasks without touching the mouse
- **Omnibar** — quick-add tasks with inline tags (`+project`, `status:todo`, `(A)` priority)
- **Drag & drop** — move cards between columns with the mouse too
- **Rust CLI** — manage tasks from the terminal (`dev2step status`, `add`, `transition`, etc.)
- **Metrics sidebar** — cycle time (Progress→Done) and status distribution at a glance
- **Debounced file watcher** — external `todo.txt` edits sync to the UI in ~150 ms
- **State history** — SQLite logs every status transition for velocity tracking

---

## Tech Stack

| Layer | Technology |
|---|---|
| Desktop shell | [Tauri v2](https://tauri.app) |
| Backend logic | Rust 2021 |
| Persistence | SQLite via [rusqlite](https://github.com/rusqlite/rusqlite) |
| File watcher | [notify](https://github.com/notify-rs/notify) |
| CLI parser | [clap](https://clap.rs) |
| Frontend | Vanilla TypeScript + CSS (Vite) |

---

## Getting Started

### Prerequisites

- [Rust + Cargo](https://rustup.rs)
- [Node.js](https://nodejs.org) (v18+) and [pnpm](https://pnpm.io)
- Linux: `webkit2gtk-4.1`, `libsoup-3.0` (see [Tauri prerequisites](https://tauri.app/start/prerequisites/))

### Install dependencies

```bash
pnpm install
```

### Run in development mode

```bash
pnpm run tauri dev
```

### Build for production

```bash
pnpm run tauri build
```

---

## CLI Usage

```bash
# Show the current board grouped by status
dev2step status

# Add a new task (lands in Icebox by default)
dev2step add "Implement OAuth flow" --project Backend --todo

# Move task #3 to In Progress
dev2step transition --id 3 --to progress

# Mark task #3 as done
dev2step complete --id 3

# Remove a task permanently
dev2step remove --id 3
```

---

## Keyboard Shortcuts

| Key | Action |
|---|---|
| `/` or `N` | Focus the omnibar (quick-add) |
| `Escape` | Dismiss the omnibar |
| `↑` / `↓` or `Tab` | Navigate between task cards |
| `1` | Move focused task → Icebox |
| `2` | Move focused task → Todo |
| `3` | Move focused task → In Progress |
| `4` | Move focused task → Done |
| `Delete` / `Backspace` | Delete focused task (with confirmation) |
| `Ctrl + M` | Toggle Metrics sidebar |

---

## Data Storage

Dev2Step resolves paths by **walking up the directory tree** from the current working directory, looking for `.git/` or `todo.txt`. If neither is found, it falls back to `~/.dev2step/`.

| File | Purpose |
|---|---|
| `todo.txt` | Single Source of Truth — human-readable, always up-to-date |
| `dev2step.db` | SQLite cache for queries, WIP counts, and history |

### todo.txt format

Dev2Step extends the [todo.txt standard](http://todotxt.org/) with extra key-value tags:

```
(A) 2026-06-17 Implement auth flow +Backend status:progress due:2026-06-30 id:1
x 2026-06-18 2026-06-15 Write unit tests +Backend status:done id:2
2026-06-17 Refactor DB layer +Backend status:icebox id:3 parent:1
```

| Tag | Meaning |
|---|---|
| `status:` | `icebox` \| `todo` \| `progress` \| `done` |
| `id:` | Unique integer assigned on first load |
| `due:` | ISO date (`YYYY-MM-DD`) |
| `parent:` | ID of a parent task (sub-task support) |

---

## WIP Limits

Soft limits are enforced visually — exceeding them highlights the column in amber:

| Column | Default limit |
|---|---|
| Todo | 5 |
| In Progress | 2 |

---

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- [WebStorm](https://www.jetbrains.com/webstorm/) for the TypeScript frontend

---

## License

MIT
