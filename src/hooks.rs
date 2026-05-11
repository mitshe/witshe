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
}

fn hook_paths(event: &str) -> Vec<std::path::PathBuf> {
    let Some(dir) = dirs::home_dir().map(|h| h.join(".witshe").join("hooks")) else {
        return Vec::new();
    };

    let mut paths = Vec::new();

    // Single file: ~/.witshe/hooks/post-new
    let single = dir.join(event);
    if single.is_file() && is_executable(&single) {
        paths.push(single);
    }

    // Directory: ~/.witshe/hooks/post-new.d/*
    let dir_path = dir.join(format!("{}.d", event));
    if dir_path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&dir_path) {
            let mut files: Vec<_> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_file() && is_executable(p))
                .collect();
            files.sort();
            paths.extend(files);
        }
    }

    paths
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

/// Run a post-* hook. Errors are warnings, never abort.
pub fn run_post(ctx: &HookContext) {
    for path in hook_paths(&ctx.event) {
        let result = Command::new(&path)
            .envs(ctx.env_vars())
            .output();

        match result {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.is_empty() {
                        eprintln!("  hook {}: {}", path.display(), stderr.trim());
                    }
                }
            }
            Err(e) => {
                eprintln!("  hook {}: {}", path.display(), e);
            }
        }
    }
}

/// Run a pre-* hook. Returns Err if any hook aborts (exit != 0).
pub fn run_pre(ctx: &HookContext) -> Result<(), String> {
    for path in hook_paths(&ctx.event) {
        let result = Command::new(&path)
            .envs(ctx.env_vars())
            .output();

        match result {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let msg = if !stderr.is_empty() {
                        stderr.trim().to_string()
                    } else if !stdout.is_empty() {
                        stdout.trim().to_string()
                    } else {
                        "hook aborted".to_string()
                    };
                    return Err(msg);
                }
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}
