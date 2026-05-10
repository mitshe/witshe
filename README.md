# witshe

tmux + git worktrees = threads.

## Install

```bash
cargo install --path .
```

## Usage

```bash
ws                        # pick a thread
ws new feat/login         # create thread
ws done                   # mark current as done
ws -c                     # jump to last session
```

`ws` is an alias for `witshe`. Add to your shell config:

```bash
alias ws="witshe"
```
