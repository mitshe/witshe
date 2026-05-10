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

pub fn create_worktree(repo_path: &str, branch_name: &str) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("no home dir")?;
    let wt_dir = home.join(".witshe").join("worktrees");
    std::fs::create_dir_all(&wt_dir).map_err(|e| e.to_string())?;

    let wt_path = wt_dir.join(branch_name);
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

    Ok(wt_str)
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

pub fn has_recent_output(session_name: &str) -> bool {
    // Check if the pane has had activity (cursor moved recently)
    let output = Command::new("tmux")
        .args(["display-message", "-t", session_name, "-p", "#{window_activity}"])
        .output()
        .ok();

    match output {
        Some(o) if o.status.success() => {
            let activity_str = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if let Ok(activity) = activity_str.parse::<u64>() {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                // Activity in last 30 seconds = "working"
                now - activity < 30
            } else {
                false
            }
        }
        _ => false,
    }
}

pub fn capture_pane(session_name: &str, lines: u32) -> Option<String> {
    let output = Command::new("tmux")
        .args(["capture-pane", "-t", session_name, "-p", "-l", &lines.to_string()])
        .output()
        .ok()?;

    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() { None } else { Some(text) }
    } else {
        None
    }
}

pub fn capture_last_line(session_name: &str) -> Option<String> {
    let output = Command::new("tmux")
        .args(["capture-pane", "-t", session_name, "-p", "-l", "1"])
        .output()
        .ok()?;

    if output.status.success() {
        let line = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if line.is_empty() { None } else { Some(line) }
    } else {
        None
    }
}
