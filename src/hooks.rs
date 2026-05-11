use std::collections::HashMap;
use std::process::Command;

#[derive(Clone)]
pub struct HookContext {
    pub event: String,
    pub thread_name: String,
    pub thread_tag: String,
    pub thread_desc: String,
    pub repo_path: String,
    pub worktree_path: String,
    pub branch: String,
}

impl HookContext {
    fn env_vars(&self) -> HashMap<&str, &str> {
        let mut vars = HashMap::new();
        vars.insert("WITSHE_EVENT", self.event.as_str());
        vars.insert("WITSHE_THREAD_NAME", self.thread_name.as_str());
        vars.insert("WITSHE_THREAD_TAG", self.thread_tag.as_str());
        vars.insert("WITSHE_THREAD_DESC", self.thread_desc.as_str());
        vars.insert("WITSHE_REPO_PATH", self.repo_path.as_str());
        vars.insert("WITSHE_WORKTREE_PATH", self.worktree_path.as_str());
        vars.insert("WITSHE_BRANCH", self.branch.as_str());
        vars
    }

    fn repo_basename(&self) -> String {
        std::path::Path::new(&self.repo_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
    }
}

fn hooks_dir() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".witshe").join("hooks"))
}

fn collect_hooks(event: &str, repo_basename: &str) -> Vec<std::path::PathBuf> {
    let Some(dir) = hooks_dir() else { return Vec::new() };
    let mut paths = Vec::new();

    // 1. Single file: ~/.witshe/hooks/post-new
    let single = dir.join(event);
    if single.is_file() && is_executable(&single) {
        paths.push(single);
    }

    // 2. Event directory: ~/.witshe/hooks/post-new.d/*
    let event_dir = dir.join(format!("{}.d", event));
    if event_dir.is_dir() {
        paths.extend(sorted_executables(&event_dir));
    }

    // 3. Convention: ~/.witshe/hooks/init.d/<repo-basename>.sh
    //    Called after post-add and post-new hooks for repo-specific init
    if (event == "post-new" || event == "post-add") && !repo_basename.is_empty() {
        let init_script = dir.join("init.d").join(format!("{}.sh", repo_basename));
        if init_script.is_file() && is_executable(&init_script) {
            paths.push(init_script);
        }
    }

    paths
}

fn sorted_executables(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_executable(p))
        .collect();
    files.sort();
    files
}

fn is_executable(path: &std::path::Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.exists()
    }
}

fn run_hook(path: &std::path::Path, ctx: &HookContext) -> Result<(), String> {
    let result = Command::new(path)
        .envs(ctx.env_vars())
        .output();

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if !stdout.is_empty() {
                eprint!("{}", stdout);
            }
            if !stderr.is_empty() {
                eprint!("{}", stderr);
            }

            if output.status.success() {
                Ok(())
            } else {
                let msg = if !stderr.is_empty() {
                    stderr.trim().to_string()
                } else if !stdout.is_empty() {
                    stdout.trim().to_string()
                } else {
                    "hook failed".to_string()
                };
                Err(msg)
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Run post-* hooks. Continue on error, log warnings.
pub fn run_post(ctx: &HookContext) {
    let hooks = collect_hooks(&ctx.event, &ctx.repo_basename());
    for path in &hooks {
        if let Err(e) = run_hook(path, ctx) {
            eprintln!("  warning: hook {} failed: {}", path.display(), e);
        }
    }
}

/// Run pre-* hooks. Continue on error UNLESS hook exits non-zero — then abort.
pub fn run_pre(ctx: &HookContext) -> Result<(), String> {
    let hooks = collect_hooks(&ctx.event, &ctx.repo_basename());
    for path in &hooks {
        run_hook(path, ctx)?;
    }
    Ok(())
}
