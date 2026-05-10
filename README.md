# witshe

tmux + git worktrees = threads.

Stop juggling branches. Each task gets its own worktree + tmux session. Switch between them like tabs.

## Install

```bash
cargo install --path .
```

Requires: `tmux`, `git`

## Usage

```bash
# See your threads
witshe

# Create a thread (worktree + tmux session)
witshe thread feat/login
witshe thread feat/api --tag epik --desc "REST API endpoints"

# Quick switch by number
witshe 1
witshe 2

# Switch by name
witshe switch feat/login

# Resume interactively (pick from list)
witshe --resume

# Jump to last active session
witshe --continue

# Mark as done (kills session, keeps in history)
witshe done

# From inside a thread, change metadata
witshe name feat/login-v2
witshe tag code-review
witshe desc "review PR #42"

# Overview with last output from each session
witshe overview

# Send overview to claude for analysis
witshe overview --claude

# List all (including done)
witshe ls --all

# Clean up
witshe rm feat/old-stuff
witshe clear              # remove all done from history
```

## How it works

```
witshe thread feat/login
```

1. `git worktree add ~/.witshe/worktrees/feat/login -b feat/login`
2. `tmux new-session -d -s witshe/feat/login -c <worktree-path>`
3. Saves thread metadata to `~/.witshe/threads.json`

That's it. No daemon, no magic. Just tmux + worktrees glued together.

## Thread lifecycle

```
create  →  active  →  done  →  clear
            ↑                     │
            └── witshe rm ────────┘ (permanent delete)
```

- `witshe done` — session killed, stays in history
- `witshe rm` — everything deleted (session + worktree + history)
- `witshe clear` — bulk remove all done threads

## Dashboard

```
$ witshe

witshe — tmux + git worktrees

  active threads:

  ●  1) feat/login [code-review]
        review PR #42 od Marka
  ●  2) feat/api [epik]
        REST API endpoints
  ✗  3) fix/bug-123

  done: 1 thread(s) (witshe ls --all)

  commands:
    witshe thread <name>   create new thread
    witshe switch <name>   switch to thread
    witshe done [name]     mark as done
    witshe --resume        pick thread interactively
    witshe --continue      jump to last active session
    witshe overview        show status + last output
```

## Status icons

| Icon | Meaning |
|------|---------|
| ● | Session alive, recent activity |
| ○ | Session alive, idle |
| ✗ | Session dead |
| ✓ | Done |

## Overview + Claude

```bash
$ witshe overview --claude
```

Captures last N lines from each tmux pane, sends to `claude -p` asking:
- Which threads look done?
- Any issues?
- What to focus on next?

## Config

Everything lives in `~/.witshe/`:
- `threads.json` — thread metadata
- `worktrees/` — git worktrees
