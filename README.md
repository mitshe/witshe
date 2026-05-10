# witshe

tmux + git worktrees = threads.

Each task gets its own worktree + tmux session. Switch between them like tabs.

## Install

```bash
cargo install --path .
```

Requires: `tmux`, `git`

## Quick start

```bash
witshe thread feat/login                          # create thread
witshe thread feat/api --tag epik --desc "REST"   # with metadata
witshe 1                                          # switch by number
witshe switch feat/login                          # switch by name
witshe done                                       # mark current as done
witshe overview --claude                          # ask claude what's next
```

## Dashboard

```
$ witshe

witshe — tmux + git worktrees

  active threads:

  ●  1) feat/login [code-review]
        review PR #42
  ●  2) feat/api [epik]
  ✗  3) fix/bug-123
```

`●` active `○` idle `✗` dead `✓` done

## All commands

| Command | What it does |
|---------|-------------|
| `witshe` | Dashboard |
| `witshe thread <name>` | Create thread (worktree + tmux) |
| `witshe <N>` | Quick switch by number |
| `witshe switch <name>` | Switch by name |
| `witshe --resume` | Pick from list |
| `witshe --continue` | Jump to last session |
| `witshe done` | Mark done, kill session |
| `witshe name/tag/desc <val>` | Edit current thread |
| `witshe overview` | Status + last output |
| `witshe overview --claude` | Send status to Claude |
| `witshe ls --all` | Include done threads |
| `witshe rm <name>` | Delete permanently |
| `witshe clear` | Remove all done |
