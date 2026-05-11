use std::process::Command;

/// Get the current tmux session name (if inside tmux)
pub fn current_session() -> Option<String> {
    let output = Command::new("tmux")
        .args(["display-message", "-p", "#{session_name}"])
        .output()
        .ok()?;

    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if name.is_empty() { None } else { Some(name) }
    } else {
        None
    }
}

/// Extract thread name from witshe session name (e.g. "witshe/feat-login" -> "feat-login")
pub fn current_thread() -> Option<String> {
    current_session().and_then(|s| s.strip_prefix("witshe/").map(|n| n.to_string()))
}

fn resolve_worktree_root(repo_path: &str) -> Result<std::path::PathBuf, String> {
    // 1. WITSHE_WORKTREE_ROOT env var (with tilde expansion)
    if let Ok(val) = std::env::var("WITSHE_WORKTREE_ROOT") {
        let expanded = if val.starts_with("~/") {
            let home = dirs::home_dir().ok_or("no home dir")?;
            home.join(&val[2..])
        } else {
            std::path::PathBuf::from(&val)
        };
        return Ok(expanded);
    }

    // 2. Convention: .worktrees/ in repo's parent dir
    let repo = std::path::Path::new(repo_path);
    if let Some(parent) = repo.parent() {
        let convention = parent.join(".worktrees");
        if convention.is_dir() {
            return Ok(convention);
        }
    }

    // 3. Default
    let home = dirs::home_dir().ok_or("no home dir")?;
    Ok(home.join(".witshe").join("worktrees"))
}

pub fn create_worktree(repo_path: &str, branch_name: &str, thread_name: &str) -> Result<String, String> {
    let root = resolve_worktree_root(repo_path)?;
    let repo_basename = std::path::Path::new(repo_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "repo".to_string());

    let wt_dir = root.join(thread_name);
    std::fs::create_dir_all(&wt_dir).map_err(|e| e.to_string())?;

    let wt_path = wt_dir.join(&repo_basename);
    let wt_str = wt_path.to_string_lossy().to_string();

    let output = Command::new("git")
        .args(["worktree", "add", "-b", branch_name, &wt_str])
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Branch exists? Try without -b
        if stderr.contains("already exists") {
            let output2 = Command::new("git")
                .args(["worktree", "add", &wt_str, branch_name])
                .current_dir(repo_path)
                .output()
                .map_err(|e| e.to_string())?;

            if !output2.status.success() {
                return Err(String::from_utf8_lossy(&output2.stderr).to_string());
            }
        } else {
            return Err(stderr.to_string());
        }
    }

    // Copy untracked files to new worktree
    copy_untracked(repo_path, &wt_str);

    Ok(wt_str)
}

fn copy_untracked(repo_path: &str, worktree_path: &str) {
    let repo = std::path::Path::new(repo_path);
    let wt = std::path::Path::new(worktree_path);

    // Read .witshe.copy from repo root, or use default patterns
    let copy_file = repo.join(".witshe.copy");
    let patterns: Vec<String> = if copy_file.exists() {
        std::fs::read_to_string(&copy_file)
            .unwrap_or_default()
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect()
    } else {
        // Default: copy .env files
        vec![".env".to_string(), ".env.local".to_string()]
    };

    for pattern in &patterns {
        let src = repo.join(pattern);
        if src.exists() && src.is_file() {
            let dst = wt.join(pattern);
            if let Some(parent) = dst.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::copy(&src, &dst);
        }
    }
}

pub fn remove_worktree(repo_path: &str, worktree_path: &str) -> Result<(), String> {
    let output = Command::new("git")
        .args(["worktree", "remove", worktree_path, "--force"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

pub fn create_session(session_name: &str, cwd: &str) -> Result<(), String> {
    let output = Command::new("tmux")
        .args(["new-session", "-d", "-s", session_name, "-c", cwd])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

pub fn add_window(session_name: &str, window_name: &str, cwd: &str) -> Result<(), String> {
    let output = Command::new("tmux")
        .args(["new-window", "-t", session_name, "-n", window_name, "-c", cwd])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

pub fn rename_session(old_name: &str, new_name: &str) -> Result<(), String> {
    let output = Command::new("tmux")
        .args(["rename-session", "-t", old_name, new_name])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

pub fn kill_session(session_name: &str) -> Result<(), String> {
    let output = Command::new("tmux")
        .args(["kill-session", "-t", session_name])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

pub fn switch_to(session_name: &str) -> Result<(), String> {
    let output = Command::new("tmux")
        .args(["switch-client", "-t", session_name])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

pub fn attach(session_name: &str) -> Result<(), String> {
    let status = Command::new("tmux")
        .args(["attach-session", "-t", session_name])
        .status()
        .map_err(|e| e.to_string())?;

    if !status.success() {
        return Err("failed to attach".to_string());
    }
    Ok(())
}

pub fn capture_last_lines(session_name: &str, n: u32) -> Option<String> {
    let output = Command::new("tmux")
        .args(["capture-pane", "-t", session_name, "-p", "-l", &n.to_string()])
        .output()
        .ok()?;

    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() { None } else { Some(text) }
    } else {
        None
    }
}

pub struct SessionStatus {
    pub last_line: String,
    pub needs_attention: bool,
}

const ATTENTION_PATTERNS: &[&str] = &[
    "do you want",
    "y/n",
    "yes/no",
    "(y)",
    "proceed?",
    "continue?",
    "confirm",
    "approve",
    "allow",
    "deny",
    "press enter",
    "waiting for",
];

pub fn get_session_status(session_name: &str) -> Option<SessionStatus> {
    let text = capture_last_lines(session_name, 5)?;
    let lines: Vec<&str> = text.lines().collect();
    let last_line = lines.last()?.trim().to_string();
    if last_line.is_empty() { return None; }

    let lower = text.to_lowercase();
    let needs_attention = ATTENTION_PATTERNS.iter().any(|p| lower.contains(p));

    // Truncate for display
    let display = if last_line.len() > 50 {
        format!("{}…", &last_line[..50])
    } else {
        last_line
    };

    Some(SessionStatus {
        last_line: display,
        needs_attention,
    })
}

pub fn list_sessions() -> Vec<String> {
    let output = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}"])
        .output()
        .ok();

    match output {
        Some(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|s| s.to_string())
                .collect()
        }
        _ => Vec::new(),
    }
}

