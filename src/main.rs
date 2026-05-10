mod picker;
mod threads;
mod tmux;

use clap::{Parser, Subcommand};
use threads::{ThreadStatus, Threads};

#[derive(Parser)]
#[command(name = "witshe", about = "tmux + git worktrees = threads")]
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
        /// Which thread to edit (auto-detects if inside witshe session)
        #[arg(short, long)]
        thread: Option<String>,
    },
    /// Remove thread permanently
    Rm {
        name: Option<String>,
        #[arg(long)]
        keep_worktree: bool,
        /// Remove all done threads
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
            let repo_path = repo.unwrap_or_else(|| {
                std::env::current_dir().unwrap().to_string_lossy().to_string()
            });

            let worktree_path = if no_worktree {
                repo_path.clone()
            } else {
                match tmux::create_worktree(&repo_path, &name) {
                    Ok(path) => path,
                    Err(e) => {
                        eprintln!("error: {}", e);
                        std::process::exit(1);
                    }
                }
            };

            let session_name = format!("witshe/{}", name);
            if let Err(e) = tmux::create_session(&session_name, &worktree_path) {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }

            let thread = threads::Thread::new(
                name.clone(), repo_path, worktree_path, !no_worktree, tag, desc,
            );
            store.add(thread);
            store.save();

            println!("  created: {}", name);
        }

        Commands::Ls { all } => {
            print_threads(&store, all);
        }

        Commands::Done { name } => {
            let name = resolve_thread(name);
            let _ = tmux::kill_session(&format!("witshe/{}", name));

            if store.mark_done(&name) {
                store.save();
                println!("  done: {}", name);
            } else {
                eprintln!("thread not found: {}", name);
                std::process::exit(1);
            }
        }

        Commands::Reopen { name } => {
            if store.reopen(&name) {
                // Re-create tmux session if thread has worktree
                if let Some(t) = store.get(&name) {
                    let session_name = format!("witshe/{}", name);
                    let _ = tmux::create_session(&session_name, &t.worktree_path);
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
                    if t.has_worktree && !keep_worktree {
                        let _ = tmux::remove_worktree(&t.repo_path, &t.worktree_path);
                    }
                    store.remove(&t.name);
                }
                store.save();
                println!("  removed {} done thread(s)", count);
            } else {
                let name = name.unwrap_or_else(|| {
                    eprintln!("usage: witshe rm <name> or witshe rm --done");
                    std::process::exit(1);
                });

                let _ = tmux::kill_session(&format!("witshe/{}", name));

                if let Some(thread) = store.get(&name) {
                    if thread.has_worktree && !keep_worktree {
                        let _ = tmux::remove_worktree(&thread.repo_path, &thread.worktree_path);
                    }
                }

                store.remove(&name);
                store.save();
                println!("  removed: {}", name);
            }
        }
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
        let icon = if is_done { "✓" } else if is_alive { "●" } else { "✗" };

        picker::PickerItem {
            label: format!("{} {}", icon, t.name),
            hint: t.tag.as_ref().map(|tg| format!("[{}]", tg)).unwrap_or_default(),
            desc: t.desc.clone(),
            is_done,
        }
    }).collect();

    if let Some(result) = picker::pick("witshe", &items) {
        let thread = threads[result.index].clone();
        if result.is_done {
            store.reopen(&thread.name);
            let session = format!("witshe/{}", thread.name);
            let _ = tmux::create_session(&session, &thread.worktree_path);
            store.save();
            println!("  reopened: {}", thread.name);
            do_switch(&session);
        } else {
            do_switch(&format!("witshe/{}", thread.name));
        }
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
        println!("  {} {}{}", icon, t.name, tag_str);
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
