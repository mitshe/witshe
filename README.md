# witshe

You're working on a feature. Urgent bugfix comes in. You `git stash`, switch branches, lose your terminal history, forget where you were. Sound familiar?

witshe gives each task its own git worktree and tmux session. Switch between them instantly — your code, branch, and terminal history stay exactly where you left them.

## Install

Download binary from [releases](https://github.com/mitshe/witshe/releases):

```bash
tar xzf witshe-*.tar.gz
sudo mv witshe /usr/local/bin/
```

On macOS you may need to remove the quarantine flag:

```bash
xattr -d com.apple.quarantine /usr/local/bin/witshe
```

Or build from source: `cargo install --git https://github.com/mitshe/witshe`

Requires: `tmux` and `git`

## Usage

```bash
# Create a thread — gets its own branch, worktree, and tmux session
witshe new feat/login

# Create another one — both run in parallel
witshe new fix/urgent-bug --tag hotfix --desc "prod is down"

# Switch between them — interactive picker with search
witshe

# Jump back to last session
witshe -c

# Done with a task
witshe done
```

### Multi-repo (epics)

Working across multiple repos? One thread, multiple worktrees:

```bash
witshe new auth-rewrite --no-worktree --tag epik
witshe add ~/projects/frontend --branch feat/auth
witshe add ~/projects/backend --branch feat/auth
# → 1 tmux session, 2 windows — Ctrl+B N to switch between repos
```

### All commands

```
witshe                  interactive picker
witshe new <name>       create thread
witshe add <repo>       add repo to current thread
witshe done             mark current as done
witshe reopen <name>    bring back a done thread
witshe set              edit name/tag/desc
witshe ls               list threads
witshe rm <name>        delete permanently
witshe -c               jump to last session
```

## Configuration

### Custom worktree location

By default worktrees are created in `~/.witshe/worktrees/<thread>/<repo>/`. To change this:

```bash
export WITSHE_WORKTREE_ROOT=~/my/custom/path
```

Useful when working with Docker bind mounts — place worktrees inside the mounted directory so containers can see them.

Convention: if a `.worktrees/` directory exists in the repo's parent directory, witshe will use it automatically (no env var needed).

### Copying files to new worktrees

Git worktrees don't include untracked files like `.env`. By default witshe copies `.env` and `.env.local` from your repo to each new worktree.

To customize, create `.witshe.copy` in your repo root:

```
.env
.env.local
docker-compose.override.yml
config/local.php
```

### tmux keybind

Add to `~/.tmux.conf` for a picker popup on `Ctrl+B T`:

```
bind T display-popup -E "witshe"
```

Then reload: `tmux source-file ~/.tmux.conf`

Make sure `witshe` is in your PATH. If installed with cargo, you may need the full path:

```
bind T display-popup -E "$HOME/.cargo/bin/witshe"
```

## Hooks

Shell scripts in `~/.witshe/hooks/` run automatically during thread lifecycle. Continue-on-error — a failing post-hook warns but never breaks witshe.

See **[HOOKS.md](HOOKS.md)** for full documentation, all events, env vars, and examples.

## How it works

`witshe new feat/login` does three things:

1. `git worktree add` — isolated copy of your repo on a new branch
2. `tmux new-session` — terminal in that worktree
3. Saves thread metadata to `~/.witshe/threads.json`

No daemon, no background process, no config files. Just tmux and git worktrees glued together.
