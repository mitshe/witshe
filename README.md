# witshe

Work on multiple tasks without losing context. Each thread gets its own git worktree and tmux session — switch between them instantly.

No more `git stash`, no more branch juggling, no more lost terminal history.

## Install

### From releases (recommended)

Download the latest binary from [releases](https://github.com/mitshe/witshe/releases), extract and move to your PATH:

```bash
tar xzf witshe-*.tar.gz
sudo mv witshe /usr/local/bin/
```

### From source

```bash
cargo install --git https://github.com/mitshe/witshe
```

Requires: `tmux`, `git`, `cargo`

### Shortcut (optional)

```bash
echo 'alias ws="witshe"' >> ~/.zshrc
```

## Usage

```bash
witshe                                           # interactive picker
witshe new feat/login                            # create thread
witshe new feat/api --tag epik --desc "REST API" # with metadata
witshe done                                      # mark current as done
witshe -c                                        # jump to last session
witshe set --tag bugfix                          # edit current thread
witshe reopen feat/login                         # bring back a done thread
witshe ls                                        # list (non-interactive)
witshe rm feat/old                               # delete permanently
```

## How it works

`witshe new feat/login` does three things:

1. `git worktree add` — isolated copy of your repo
2. `tmux new-session` — terminal session in that worktree
3. Saves metadata to `~/.witshe/threads.json`

That's it. No daemon, no background process. Just tmux + worktrees glued together.
