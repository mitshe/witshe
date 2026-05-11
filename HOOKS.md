# witshe hooks

Hooks are executable scripts that run automatically during thread lifecycle events.
They live in `~/.witshe/hooks/`.

## Events

| Event | When | On error |
|-------|------|----------|
| `post-new` | After `witshe new` creates a thread | Continue (warning) |
| `post-add` | After `witshe add` adds a repo | Continue (warning) |
| `pre-done` | Before `witshe done` marks thread done | **Abort** (exit ≠ 0 cancels done) |
| `post-done` | After `witshe done` | Continue (warning) |
| `pre-rm` | Before `witshe rm` removes a thread | **Abort** (exit ≠ 0 cancels rm) |
| `post-rm` | After `witshe rm` | Continue (warning) |

## Environment variables

Every hook receives these env vars:

| Variable | Description | Example |
|----------|-------------|---------|
| `WITSHE_EVENT` | Event name | `post-new` |
| `WITSHE_THREAD_NAME` | Thread name | `feat/login` |
| `WITSHE_THREAD_TAG` | Tag (may be empty) | `epik` |
| `WITSHE_THREAD_DESC` | Description (may be empty) | `JIRA-123 auth rewrite` |
| `WITSHE_REPO_PATH` | Original repo path | `/Users/you/project` |
| `WITSHE_WORKTREE_PATH` | Created worktree path | `/Users/you/.witshe/worktrees/feat-login/project` |
| `WITSHE_BRANCH` | Branch name | `feat/login` |

## Hook resolution order

For each event, witshe looks for hooks in this order:

```
1. ~/.witshe/hooks/<event>              # single file
2. ~/.witshe/hooks/<event>.d/*          # directory, sorted alphabetically
3. ~/.witshe/hooks/init.d/<repo>.sh     # repo-specific init (post-new, post-add only)
```

All found hooks run in sequence. All must be `chmod +x`.

### init.d convention

`init.d/<repo-basename>.sh` runs after `post-new` and `post-add` hooks.
The basename is derived from the repo path (e.g. `/home/you/projects/frontend` → `frontend.sh`).

This is useful for repo-specific setup — e.g. creating an Apache vhost only for
`crm-symfony` but not for `shared-lib`.

## File structure example

```
~/.witshe/hooks/
├── post-new                    # runs after every witshe new
├── post-add.d/
│   ├── 01-log.sh               # runs first after witshe add
│   └── 02-vhost.sh             # runs second
├── init.d/
│   ├── crm-symfony.sh          # runs after post-new/post-add for crm-symfony repo
│   └── crm-frontend.sh         # runs after post-new/post-add for crm-frontend repo
├── pre-rm                      # runs before witshe rm, can abort
└── post-rm.d/
    └── cleanup-vhost.sh        # cleanup after removal
```

## Examples

### Simple logger

`~/.witshe/hooks/post-new`:

```bash
#!/bin/bash
echo "[$(date)] new: $WITSHE_THREAD_NAME tag=$WITSHE_THREAD_TAG repo=$WITSHE_REPO_PATH" >> ~/.witshe/activity.log
```

### Repo-specific init (e.g. Apache vhost for Docker)

`~/.witshe/hooks/init.d/crm-symfony.sh`:

```bash
#!/bin/bash
# Create Apache vhost pointing to new worktree
VHOST_NAME="${WITSHE_BRANCH}.crm-symfony.local"
VHOST_CONF="/etc/apache2/sites-available/${VHOST_NAME}.conf"

cat > "$VHOST_CONF" <<EOF
<VirtualHost *:80>
    ServerName ${VHOST_NAME}
    DocumentRoot ${WITSHE_WORKTREE_PATH}/public
</VirtualHost>
EOF

# Add to /etc/hosts
echo "127.0.0.1 ${VHOST_NAME}" >> /etc/hosts

# Reload Apache in docker container
docker exec apache-ct apachectl graceful

echo "  vhost: http://${VHOST_NAME}"
```

### Pre-rm guard

`~/.witshe/hooks/pre-rm`:

```bash
#!/bin/bash
# Prevent removing threads with uncommitted changes
if [ -d "$WITSHE_WORKTREE_PATH" ]; then
    cd "$WITSHE_WORKTREE_PATH"
    if ! git diff --quiet 2>/dev/null; then
        echo "error: uncommitted changes in $WITSHE_WORKTREE_PATH"
        exit 1  # aborts witshe rm
    fi
fi
```

### Cleanup vhost on rm

`~/.witshe/hooks/post-rm.d/cleanup-vhost.sh`:

```bash
#!/bin/bash
VHOST_NAME="${WITSHE_BRANCH}.crm-symfony.local"
rm -f "/etc/apache2/sites-available/${VHOST_NAME}.conf"
sed -i "/${VHOST_NAME}/d" /etc/hosts
docker exec apache-ct apachectl graceful 2>/dev/null
```

## Notes

- Hooks without `chmod +x` are silently ignored
- `post-*` hooks use continue-on-error — a failing hook prints a warning but witshe proceeds
- `pre-*` hooks abort the action if any script exits non-zero
- Hook stdout/stderr is forwarded to witshe's output
- No timeout — hooks run to completion
