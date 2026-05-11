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

fn hook_path(event: &str) -> Option<std::path::PathBuf> {
    let dir = dirs::home_dir()?.join(".witshe").join("hooks");
    let path = dir.join(event);
    if path.exists() && is_executable(&path) {
        Some(path)
    } else {
        None
    }
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
    let Some(path) = hook_path(&ctx.event) else { return };

    let result = Command::new(&path)
        .envs(ctx.env_vars())
        .output();

    match result {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.is_empty() {
                    eprintln!("  hook {}: {}", ctx.event, stderr.trim());
                }
            }
        }
        Err(e) => {
            eprintln!("  hook {}: {}", ctx.event, e);
        }
    }
}

/// Run a pre-* hook. Returns Err if hook aborts (exit != 0).
pub fn run_pre(ctx: &HookContext) -> Result<(), String> {
    let Some(path) = hook_path(&ctx.event) else { return Ok(()) };

    let result = Command::new(&path)
        .envs(ctx.env_vars())
        .output();

    match result {
        Ok(output) => {
            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                let msg = if !stderr.is_empty() {
                    stderr.trim().to_string()
                } else if !stdout.is_empty() {
                    stdout.trim().to_string()
                } else {
                    "hook aborted".to_string()
                };
                Err(msg)
            }
        }
        Err(e) => Err(e.to_string()),
    }
}
