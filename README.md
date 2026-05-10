# witshe

tmux + git worktrees = threads.

Each task gets its own worktree + tmux session. Switch between them like tabs.

## Install

```bash
cargo install --path .
```

Requires: `tmux`, `git`

## Usage

```bash
witshe                                            # interactive picker
witshe -c                                         # jump to last session
witshe new feat/login                             # create thread
witshe new feat/api --tag epik --desc "REST API"  # with metadata
witshe done                                       # mark current as done
witshe reopen feat/login                          # bring back a done thread
witshe set --tag bugfix                           # edit current thread
witshe set --name new-name --thread old-name      # rename
witshe ls                                         # list (non-interactive)
witshe ls --all                                   # include done
witshe rm feat/old                                # delete permanently
witshe rm --done                                  # delete all done
```

## Picker

```
$ witshe

  witshe

  > ● feat/login [code-review]
      review PR #42
    ● feat/api [epik]
    ✗ fix/bug-123

  + 1 done (witshe ls --all)

  ↑↓ select  enter switch  q quit
```

`●` alive `✗` dead `✓` done
