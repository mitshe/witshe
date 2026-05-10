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
```

## Dashboard

```
$ witshe

witshe — tmux + git worktrees

  ●  1) feat/login [code-review]
        review PR #42
  ●  2) feat/api [epik]
  ✗  3) fix/bug-123

  + 1 done (witshe ls --all)
```

`●` alive `✗` dead `✓` done

## Commands

| Command | What it does |
|---------|-------------|
| `witshe` | Dashboard |
| `witshe <N>` | Quick switch by number |
| `witshe thread <name>` | Create thread (worktree + tmux) |
| `witshe switch <name>` | Switch by name |
| `witshe --resume` | Pick from list |
| `witshe --continue` | Jump to last session |
| `witshe done` | Mark done, kill session |
| `witshe name/tag/desc <val>` | Edit current thread |
| `witshe ls --all` | Include done threads |
| `witshe rm <name>` | Delete permanently |
| `witshe clear` | Remove all done |
