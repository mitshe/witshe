mod hooks;
mod picker;
mod threads;
mod tmux;

use clap::{Parser, Subcommand};
use hooks::HookContext;
use threads::{Repo, ThreadStatus, Threads};

#[derive(Parser)]
#[command(name = "witshe", about = "tmux + git worktrees = threads", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Jump to last active session
    #[arg(short = 'c', long = "continue")]
    r#continue: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new thread (worktree + tmux session)
    New {
        name: String,
        #[arg(short, long)]
        repo: Option<String>,
        #[arg(long)]
        no_worktree: bool,
        #[arg(short, long)]
        tag: Option<String>,
        #[arg(short, long)]
        desc: Option<String>,
    },
    /// Add a repo to current thread (creates worktree + tmux window)
    Add {
        /// Path to the repo
        repo: String,
        /// Branch name
        #[arg(short, long)]
        branch: String,
        /// Thread to add to (auto-detects if inside witshe session)
        #[arg(short, long)]
        thread: Option<String>,
    },
    /// List threads (non-interactive)
    Ls {
        #[arg(short, long)]
        all: bool,
    },
    /// Mark thread as done
    Done {
        name: Option<String>,
    },
    /// Reopen a done thread
    Reopen {
        name: String,
    },
    /// Edit thread metadata
    Set {
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        tag: Option<String>,
        #[arg(long)]
        desc: Option<String>,
        #[arg(short, long)]
        thread: Option<String>,
    },
    /// Remove thread permanently
    Rm {
        name: Option<String>,
        #[arg(long)]
        keep_worktree: bool,
        #[arg(long)]
        done: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let mut store = Threads::load();

    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            if cli.r#continue {
                continue_last(&store);
            } else {
                interactive(&mut store);
            }
            return;
        }
    };

    match command {
        Commands::New { name, repo, no_worktree, tag, desc } => {
            let mut thread = threads::Thread::new(name.clone(), tag, desc);

            if no_worktree {
                let cwd = repo.unwrap_or_else(|| {
                    std::env::current_dir().unwrap().to_string_lossy().to_string()
                });
                thread.cwd = Some(cwd.clone());

                let session = format!("witshe/{}", name);
                if let Err(e) = tmux::create_session(&session, &cwd) {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            } else {
                let repo_path = repo.unwrap_or_else(|| {
                    std::env::current_dir().unwrap().to_string_lossy().to_string()
                });

                let wt_path = match tmux::create_worktree(&repo_path, &name, &name) {
                    Ok(path) => path,
                    Err(e) => {
                        eprintln!("error: {}", e);
                        std::process::exit(1);
                    }
                };

                let session = format!("witshe/{}", name);
                if let Err(e) = tmux::create_session(&session, &wt_path) {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }

                thread.add_repo(Repo {
                    repo_path,
                    worktree_path: wt_path,
                    branch: name.clone(),
                    has_worktree: true,
                });
            }

            let first_repo = thread.repos.first();
            hooks::run_post(&HookContext {
                event: "post-new".into(),
                thread_name: name.clone(),
                thread_tag: thread.tag.clone().unwrap_or_default(),
                thread_desc: thread.desc.clone().unwrap_or_default(),
                repo_path: first_repo.map(|r| r.repo_path.clone()).unwrap_or_default(),
                worktree_path: first_repo.map(|r| r.worktree_path.clone()).unwrap_or_default(),
                branch: first_repo.map(|r| r.branch.clone()).unwrap_or_default(),
            });

            store.add(thread);
            store.save();
            println!("  created: {}", name);
        }

        Commands::Add { repo, branch, thread } => {
            let thread_name = resolve_thread(thread);

            let repo_path = std::fs::canonicalize(&repo)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or(repo);

            let wt_path = match tmux::create_worktree(&repo_path, &branch, &thread_name) {
                Ok(path) => path,
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            };

            let repo_basename = std::path::Path::new(&repo_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| branch.clone());

            // Add tmux window
            let session = format!("witshe/{}", thread_name);
            let alive = tmux::list_sessions();
            if alive.contains(&session) {
                let _ = tmux::add_window(&session, &repo_basename, &wt_path);
            }

            if let Some(t) = store.get_mut(&thread_name) {
                let tag = t.tag.clone().unwrap_or_default();
                let desc = t.desc.clone().unwrap_or_default();

                t.add_repo(Repo {
                    repo_path: repo_path.clone(),
                    worktree_path: wt_path.clone(),
                    branch: branch.clone(),
                    has_worktree: true,
                });
                store.save();

                hooks::run_post(&HookContext {
                    event: "post-add".into(),
                    thread_name: thread_name.clone(),
                    thread_tag: tag,
                    thread_desc: desc,
                    repo_path,
                    worktree_path: wt_path,
                    branch,
                });

                println!("  added: {} -> {}", repo_basename, thread_name);
            } else {
                eprintln!("thread not found: {}", thread_name);
                std::process::exit(1);
            }
        }

        Commands::Ls { all } => {
            print_threads(&store, all);
        }

        Commands::Done { name } => {
            let name = resolve_thread(name);

            let ctx = make_hook_ctx("", &name, &store);

            if let Err(msg) = hooks::run_pre(&HookContext { event: "pre-done".into(), ..ctx.clone() }) {
                eprintln!("  aborted by hook: {}", msg);
                std::process::exit(1);
            }

            if store.mark_done(&name) {
                store.save();
                println!("  done: {}", name);
                hooks::run_post(&HookContext { event: "post-done".into(), ..ctx });
                let _ = tmux::kill_session(&format!("witshe/{}", name));
            } else {
                eprintln!("thread not found: {}", name);
                std::process::exit(1);
            }
        }

        Commands::Reopen { name } => {
            if store.reopen(&name) {
                if let Some(t) = store.get(&name) {
                    let session = format!("witshe/{}", name);
                    let cwd = t.first_cwd().unwrap_or_else(|| ".".to_string());
                    let _ = tmux::create_session(&session, &cwd);

                    // Re-create windows for additional repos
                    for (i, repo) in t.repos.iter().enumerate() {
                        if i == 0 { continue; } // first repo is the session's initial window
                        let basename = std::path::Path::new(&repo.repo_path)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| repo.branch.clone());
                        let _ = tmux::add_window(&session, &basename, &repo.worktree_path);
                    }
                }
                store.save();
                println!("  reopened: {}", name);
            } else {
                eprintln!("thread not found: {}", name);
                std::process::exit(1);
            }
        }

        Commands::Set { name, tag, desc, thread } => {
            let target = resolve_thread(thread);

            if name.is_none() && tag.is_none() && desc.is_none() {
                eprintln!("nothing to set. use --name, --tag, or --desc");
                std::process::exit(1);
            }

            if let Some(new_name) = name {
                let _ = tmux::rename_session(
                    &format!("witshe/{}", target),
                    &format!("witshe/{}", new_name),
                );
                store.rename(&target, &new_name);
                println!("  name: {} -> {}", target, new_name);
            }

            let target_name = store.get(&target).map(|t| t.name.clone())
                .unwrap_or(target.clone());

            if let Some(tag_val) = tag {
                store.set_tag(&target_name, &tag_val);
                println!("  tag: [{}]", tag_val);
            }

            if let Some(desc_val) = desc {
                store.set_desc(&target_name, &desc_val);
                println!("  desc: {}", desc_val);
            }

            store.save();
        }

        Commands::Rm { name, keep_worktree, done } => {
            if done {
                let done_threads: Vec<_> = store.list().iter()
                    .filter(|t| matches!(t.status, ThreadStatus::Done))
                    .cloned()
                    .collect();

                let count = done_threads.len();
                for t in &done_threads {
                    let ctx = HookContext {
                        event: "pre-rm".into(),
                        thread_name: t.name.clone(),
                        thread_tag: t.tag.clone().unwrap_or_default(),
                        thread_desc: t.desc.clone().unwrap_or_default(),
                        repo_path: t.repos.first().map(|r| r.repo_path.clone()).unwrap_or_default(),
                        worktree_path: t.repos.first().map(|r| r.worktree_path.clone()).unwrap_or_default(),
                        branch: t.repos.first().map(|r| r.branch.clone()).unwrap_or_default(),
                    };
                    // pre-rm can't abort bulk delete
                    let _ = hooks::run_pre(&ctx);

                    if !keep_worktree {
                        for repo in &t.repos {
                            if repo.has_worktree {
                                let _ = tmux::remove_worktree(&repo.repo_path, &repo.worktree_path);
                            }
                        }
                    }
                    store.remove(&t.name);
                    hooks::run_post(&HookContext { event: "post-rm".into(), ..ctx });
                }
                store.save();
                println!("  removed {} done thread(s)", count);
            } else {
                let name = name.unwrap_or_else(|| {
                    eprintln!("usage: witshe rm <name> or witshe rm --done");
                    std::process::exit(1);
                });

                let ctx = make_hook_ctx("", &name, &store);

                if let Err(msg) = hooks::run_pre(&HookContext { event: "pre-rm".into(), ..ctx.clone() }) {
                    eprintln!("  aborted by hook: {}", msg);
                    std::process::exit(1);
                }

                let _ = tmux::kill_session(&format!("witshe/{}", name));

                if let Some(thread) = store.get(&name) {
                    if !keep_worktree {
                        for repo in &thread.repos {
                            if repo.has_worktree {
                                let _ = tmux::remove_worktree(&repo.repo_path, &repo.worktree_path);
                            }
                        }
                    }
                }

                store.remove(&name);
                store.save();
                hooks::run_post(&HookContext { event: "post-rm".into(), ..ctx });
                println!("  removed: {}", name);
            }
        }
    }
}

fn make_hook_ctx(event: &str, thread_name: &str, store: &Threads) -> HookContext {
    let t = store.get(thread_name);
    let first_repo = t.and_then(|t| t.repos.first());
    HookContext {
        event: event.to_string(),
        thread_name: thread_name.to_string(),
        thread_tag: t.and_then(|t| t.tag.clone()).unwrap_or_default(),
        thread_desc: t.and_then(|t| t.desc.clone()).unwrap_or_default(),
        repo_path: first_repo.map(|r| r.repo_path.clone()).unwrap_or_default(),
        worktree_path: first_repo.map(|r| r.worktree_path.clone()).unwrap_or_default(),
        branch: first_repo.map(|r| r.branch.clone()).unwrap_or_default(),
    }
}

fn resolve_thread(name: Option<String>) -> String {
    name.or_else(|| tmux::current_thread()).unwrap_or_else(|| {
        eprintln!("error: not inside a witshe session and no thread name given");
        std::process::exit(1);
    })
}

fn do_switch(session_name: &str) {
    if std::env::var("TMUX").is_ok() {
        if let Err(e) = tmux::switch_to(session_name) {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    } else {
        if let Err(e) = tmux::attach(session_name) {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

fn interactive(store: &mut Threads) {
    let threads = store.list();
    let alive = tmux::list_sessions();

    if threads.is_empty() {
        println!("\n  no threads. create one: witshe new <name>\n");
        return;
    }

    let items: Vec<picker::PickerItem> = threads.iter().map(|t| {
        let session = format!("witshe/{}", t.name);
        let is_alive = alive.contains(&session);
        let is_done = matches!(t.status, ThreadStatus::Done);

        let status = if is_alive { tmux::get_session_status(&session) } else { None };
        let needs_attention = status.as_ref().map(|s| s.needs_attention).unwrap_or(false);

        let icon = if is_done {
            "✓"
        } else if needs_attention {
            "⚠"
        } else if is_alive {
            "●"
        } else {
            "✗"
        };

        let repo_count = t.repos.len();
        let repos_hint = if repo_count > 1 {
            format!(" ({} repos)", repo_count)
        } else {
            String::new()
        };

        let desc = match (&t.desc, &status) {
            (Some(d), Some(s)) => Some(format!("{} │ {}", d, s.last_line)),
            (Some(d), None) => Some(d.clone()),
            (None, Some(s)) => Some(s.last_line.clone()),
            (None, None) => None,
        };

        picker::PickerItem {
            label: format!("{} {}", icon, t.name),
            hint: format!(
                "{}{}",
                t.tag.as_ref().map(|tg| format!("[{}]", tg)).unwrap_or_default(),
                repos_hint,
            ),
            desc,
            is_done,
        }
    }).collect();

    if let Some(result) = picker::pick("witshe", &items) {
        let thread = threads[result.index].clone();
        let session = format!("witshe/{}", thread.name);

        if result.is_done {
            store.reopen(&thread.name);
            store.save();
            println!("  reopened: {}", thread.name);
        }

        // Ensure session exists
        if !alive.contains(&session) {
            let cwd = thread.first_cwd().unwrap_or_else(|| ".".to_string());
            let _ = tmux::create_session(&session, &cwd);

            // Re-create windows for additional repos
            for (i, repo) in thread.repos.iter().enumerate() {
                if i == 0 { continue; }
                let basename = std::path::Path::new(&repo.repo_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| repo.branch.clone());
                let _ = tmux::add_window(&session, &basename, &repo.worktree_path);
            }
        }

        do_switch(&session);
    }
}

fn print_threads(store: &Threads, show_done: bool) {
    let threads = store.list();
    if threads.is_empty() {
        println!("  no threads");
        return;
    }

    let alive = tmux::list_sessions();

    for t in threads {
        let is_done = matches!(t.status, ThreadStatus::Done);
        if !show_done && is_done {
            continue;
        }

        let session = format!("witshe/{}", t.name);
        let is_alive = alive.contains(&session);

        let icon = match (&t.status, is_alive) {
            (ThreadStatus::Done, _) => "\x1b[90m✓\x1b[0m",
            (_, true) => "\x1b[32m●\x1b[0m",
            (_, false) => "\x1b[90m✗\x1b[0m",
        };

        let tag_str = t.tag.as_ref().map(|tg| format!(" \x1b[36m[{}]\x1b[0m", tg)).unwrap_or_default();
        let repo_str = if t.repos.len() > 1 {
            format!(" \x1b[90m({} repos)\x1b[0m", t.repos.len())
        } else {
            String::new()
        };
        println!("  {} {}{}{}", icon, t.name, tag_str, repo_str);
        if let Some(ref desc) = t.desc {
            println!("    {}", desc);
        }
    }
}

fn continue_last(store: &Threads) {
    let alive = tmux::list_sessions();

    if let Some(t) = store.list().iter().rev().find(|t| {
        matches!(t.status, ThreadStatus::Active) && alive.contains(&format!("witshe/{}", t.name))
    }) {
        do_switch(&format!("witshe/{}", t.name));
    } else {
        println!("no active sessions");
    }
}
