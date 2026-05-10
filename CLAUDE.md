# CLAUDE.md - witshe

## Co to jest

Desktop app (Tauri + React) — wrapper na Claude Code z zarządzaniem wątkami/sesjami.
Wygląda jak terminal, jest prosta jak terminal, ale grupuje pracę w wątki.

## Problem

Dev ma 5 tasków, jeden terminal, git stash, git checkout, chaos. Traci kontekst, myli branche.

## Rozwiązanie

Każdy wątek = git worktree + Claude Code session. Przełączasz się jak między tabami. Nic się nie miesza.

## Stack

- **Tauri** (Rust backend, webview frontend, mały binary)
- **React** (UI)
- **xterm.js** (terminal rendering)
- **git worktree** (izolacja branchy)

## Layout

```
┌─────────────────────────────────────────────────────────┐
│  witshe                                      ─  □  ✕   │
├────────────────────────┬────────────────────────────────┤
│ ● JIRA-123 feat/login  │                                │
│   working...           │  $ claude                      │
│ ○ JIRA-456 fix/auth    │                                │
│   paused               │  > Analyzing codebase...       │
│ ✓ JIRA-789 feat/api    │  > Found 3 files to modify     │
│   done → PR #42        │  > Editing src/auth/login.ts   │
│                        │                                │
│────────────────────────│                                │
│ + New thread           │                                │
│                        │  █                             │
└────────────────────────┴────────────────────────────────┘
```

- Sidebar PO LEWEJ — lista wątków
- Terminal PO PRAWEJ — aktywna sesja Claude Code
- Klik na wątek → przełącza sesję w terminalu

## MVP Scope

1. Lista wątków (create / switch / archive)
2. Każdy wątek = `git worktree add` + uruchomiony `claude` CLI w tym worktree
3. xterm.js renderuje aktywną sesję (PTY via Tauri/Rust)
4. Opcjonalnie: ręczne przypisanie JIRA ticket ID do wątku
5. Persystencja wątków (JSON file lub SQLite)

## Czego NIE robimy

- Żadnych kontenerów Docker
- Żadnego własnego AI — używamy Claude Code CLI
- Żadnego CQRS, hexagonal, event-driven — prosty kod
- Żadnego backendu webowego — to desktop app
- Żadnego overengineeringu

## Zasady

- Prostota > features
- Wygląda jak terminal, czarne tło, monospace font
- Minimalna ilość UI chrome
- Działa offline (poza samym Claude Code)
