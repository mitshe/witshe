mod threads;
mod tmux;

use clap::{Parser, Subcommand};
use threads::{Thread, ThreadStatus, Threads};

#[derive(Parser)]
#[command(name = "witshe", about = "tmux + git worktrees = threads")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Resume: show interactive thread list to pick from
    #[arg(long)]
    resume: bool,

    /// Continue: jump straight to last active session
    #[arg(long, alias = "c")]
    r#continue: bool,

    /// Quick switch: jump to thread by number (from witshe ls)
    #[arg(value_name = "N")]
    number: Option<usize>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new thread (worktree + tmux session)
    Thread {
        /// Branch/thread name
        name: String,
        /// Path to the repo (defaults to current dir)
        #[arg(short, long)]
        repo: Option<String>,
        /// Don't create a git worktree, just tmux session
        #[arg(long)]
        no_worktree: bool,
        /// Tag (e.g. code-review, epik, bugfix)
        #[arg(short, long)]
        tag: Option<String>,
        /// Description
        #[arg(short, long)]
        desc: Option<String>,
    },
    /// List all threads
    Ls {
        /// Show all (including done)
        #[arg(short, long)]
        all: bool,
    },
    /// Switch to a thread
    Switch {
        /// Thread name
        name: String,
    },
    /// Mark thread as done (keeps in history, kills session). Auto-detects current thread if inside tmux.
    Done {
        /// Thread name (optional if inside a witshe tmux session)
        name: Option<String>,
    },
    /// Remove a thread permanently (kills session + removes worktree)
    Rm {
        /// Thread name
        name: String,
        /// Keep the worktree on disk
        #[arg(long)]
        keep_worktree: bool,
    },
    /// Rename current (or specified) thread
    Name {
        /// New name
        new_name: String,
        /// Thread to rename (auto-detects if inside witshe session)
        #[arg(short, long)]
        thread: Option<String>,
    },
    /// Set/change tag on current (or specified) thread
    Tag {
        /// New tag
        value: String,
        /// Thread to update (auto-detects if inside witshe session)
        #[arg(short, long)]
        thread: Option<String>,
    },
    /// Set/change description on current (or specified) thread
    Desc {
        /// New description
        value: String,
        /// Thread to update (auto-detects if inside witshe session)
        #[arg(short, long)]
        thread: Option<String>,
    },
    /// Clear all done threads from history
    Clear,
    /// Show overview of all threads (status, what's running)
    Overview {
        /// Send overview to claude for analysis/suggestions
        #[arg(long)]
        claude: bool,
    },
    /// Attach to witshe tmux (or create if not exists)
    Attach,
}

fn main() {
    let cli = Cli::parse();
    let mut store = Threads::load();

    // No subcommand — interactive mode, --resume, --continue, or quick number
    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            if cli.r#continue {
                continue_last(&store);
            } else if cli.resume {
                resume_interactive(&store);
            } else if let Some(n) = cli.number {
                quick_switch(&store, n);
            } else {
                interactive(&mut store);
            }
            return;
        }
    };

    match command {
        Commands::Thread { name, repo, no_worktree, tag, desc } => {
            let repo_path = repo.unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            });

            // Create worktree
            let worktree_path = if no_worktree {
                repo_path.clone()
            } else {
                match tmux::create_worktree(&repo_path, &name) {
                    Ok(path) => {
                        println!("  worktree: {}", path);
                        path
                    }
                    Err(e) => {
                        eprintln!("error creating worktree: {}", e);
                        std::process::exit(1);
                    }
                }
            };

            // Create tmux session
            let session_name = format!("witshe/{}", name);
            if let Err(e) = tmux::create_session(&session_name, &worktree_path) {
                eprintln!("error creating tmux session: {}", e);
                std::process::exit(1);
            }

            // Save thread
            let thread = Thread::new(name.clone(), repo_path, worktree_path, !no_worktree, tag, desc);
            store.add(thread);
            store.save();

            println!("  thread: {}", name);
            println!("  session: {}", session_name);
            println!("\n  switch: witshe switch {}", name);
        }

        Commands::Ls { all } => {
            let threads = store.list();
            if threads.is_empty() {
                println!("no threads. create one: witshe thread <name>");
                return;
            }

            // Check which tmux sessions are alive
            let alive_sessions = tmux::list_sessions();

            for t in threads {
                if !all && matches!(t.status, ThreadStatus::Done) {
                    continue;
                }
                let session_name = format!("witshe/{}", t.name);
                let is_alive = alive_sessions.contains(&session_name);
                let has_activity = is_alive && tmux::has_recent_output(&session_name);

                let icon = match (&t.status, is_alive, has_activity) {
                    (ThreadStatus::Done, _, _) => "✓",
                    (_, true, true) => "●",
                    (_, true, false) => "○",
                    (_, false, _) => "✗",
                };

                let extra = if has_activity { " \x1b[1m← activity\x1b[0m" } else { "" };
                let tag_str = t.tag.as_ref().map(|t| format!(" \x1b[36m[{}]\x1b[0m", t)).unwrap_or_default();

                println!("  {} {}{}{}", icon, t.name, tag_str, extra);
                if let Some(ref desc) = t.desc {
                    println!("    {}", desc);
                }
                if let Some(ref jira) = t.jira_id {
                    println!("    {}", jira);
                }
            }
        }

        Commands::Switch { name } => {
            let session_name = format!("witshe/{}", name);
            if std::env::var("TMUX").is_ok() {
                if let Err(e) = tmux::switch_to(&session_name) {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            } else {
                if let Err(e) = tmux::attach(&session_name) {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Done { name } => {
            let name = name.or_else(|| tmux::current_thread()).unwrap_or_else(|| {
                eprintln!("error: not inside a witshe session and no thread name given");
                std::process::exit(1);
            });

            let session_name = format!("witshe/{}", name);
            let _ = tmux::kill_session(&session_name);

            if store.mark_done(&name) {
                store.save();
                println!("  done: {}", name);
            } else {
                eprintln!("thread not found: {}", name);
                std::process::exit(1);
            }
        }

        Commands::Name { new_name, thread } => {
            let name = thread.or_else(|| tmux::current_thread()).unwrap_or_else(|| {
                eprintln!("error: not inside a witshe session and no --thread given");
                std::process::exit(1);
            });

            // Rename tmux session too
            let old_session = format!("witshe/{}", name);
            let new_session = format!("witshe/{}", new_name);
            let _ = tmux::rename_session(&old_session, &new_session);

            if store.rename(&name, &new_name) {
                store.save();
                println!("  renamed: {} → {}", name, new_name);
            } else {
                eprintln!("thread not found: {}", name);
                std::process::exit(1);
            }
        }

        Commands::Tag { value, thread } => {
            let name = thread.or_else(|| tmux::current_thread()).unwrap_or_else(|| {
                eprintln!("error: not inside a witshe session and no --thread given");
                std::process::exit(1);
            });

            if store.set_tag(&name, &value) {
                store.save();
                println!("  tag: {} → [{}]", name, value);
            } else {
                eprintln!("thread not found: {}", name);
                std::process::exit(1);
            }
        }

        Commands::Desc { value, thread } => {
            let name = thread.or_else(|| tmux::current_thread()).unwrap_or_else(|| {
                eprintln!("error: not inside a witshe session and no --thread given");
                std::process::exit(1);
            });

            if store.set_desc(&name, &value) {
                store.save();
                println!("  desc: {} → {}", name, value);
            } else {
                eprintln!("thread not found: {}", name);
                std::process::exit(1);
            }
        }

        Commands::Clear => {
            let count = store.clear_done();
            store.save();
            println!("  cleared {} done thread(s)", count);
        }

        Commands::Rm { name, keep_worktree } => {
            let session_name = format!("witshe/{}", name);
            let _ = tmux::kill_session(&session_name);

            if let Some(thread) = store.get(&name) {
                if thread.has_worktree && !keep_worktree {
                    if let Err(e) = tmux::remove_worktree(&thread.repo_path, &thread.worktree_path) {
                        eprintln!("warning: could not remove worktree: {}", e);
                    }
                }
            }

            store.remove(&name);
            store.save();
            println!("  removed: {}", name);
        }

        Commands::Overview { claude } => {
            let threads = store.list();
            if threads.is_empty() {
                println!("no threads.");
                return;
            }

            let alive_sessions = tmux::list_sessions();

            let mut summary = String::new();
            let mut active = Vec::new();
            let mut idle = Vec::new();
            let mut dead = Vec::new();

            for t in threads {
                let session_name = format!("witshe/{}", t.name);
                let is_alive = alive_sessions.contains(&session_name);
                let has_activity = is_alive && tmux::has_recent_output(&session_name);

                if !is_alive {
                    dead.push(t);
                } else if has_activity {
                    active.push(t);
                } else {
                    idle.push(t);
                }
            }

            if !active.is_empty() {
                summary.push_str("WORKING:\n");
                println!("  \x1b[32m● working:\x1b[0m");
                for t in &active {
                    let session_name = format!("witshe/{}", t.name);
                    let last_line = tmux::capture_last_line(&session_name);
                    let context = tmux::capture_pane(&session_name, 20);
                    let line_str = last_line.map(|l| format!(" — {}", l)).unwrap_or_default();
                    println!("    {}{}", t.name, line_str);
                    summary.push_str(&format!("  {} (branch: {}, repo: {})\n", t.name, t.name, t.repo_path));
                    if let Some(ctx) = context {
                        summary.push_str(&format!("    last output:\n{}\n", indent(&ctx, "      ")));
                    }
                }
                println!();
                summary.push('\n');
            }

            if !idle.is_empty() {
                summary.push_str("IDLE (waiting/done):\n");
                println!("  \x1b[33m○ idle:\x1b[0m");
                for t in &idle {
                    let session_name = format!("witshe/{}", t.name);
                    let context = tmux::capture_pane(&session_name, 10);
                    println!("    {}", t.name);
                    summary.push_str(&format!("  {} (branch: {}, repo: {})\n", t.name, t.name, t.repo_path));
                    if let Some(ctx) = context {
                        summary.push_str(&format!("    last output:\n{}\n", indent(&ctx, "      ")));
                    }
                }
                println!();
                summary.push('\n');
            }

            if !dead.is_empty() {
                summary.push_str("DEAD (session gone):\n");
                println!("  \x1b[90m✗ dead:\x1b[0m");
                for t in &dead {
                    println!("    {}", t.name);
                    summary.push_str(&format!("  {}\n", t.name));
                }
                println!();
                summary.push('\n');
            }

            println!("  total: {} threads", active.len() + idle.len() + dead.len());

            if claude {
                let prompt = format!(
                    "Here's my current witshe threads overview. Tell me: which threads look done? Any issues? What should I focus on next?\n\n{}",
                    summary
                );
                println!("\n  sending to claude...\n");
                let status = std::process::Command::new("claude")
                    .args(["-p", &prompt])
                    .status();

                if let Err(e) = status {
                    eprintln!("error running claude: {}", e);
                }
            }
        }

        Commands::Attach => {
            // If inside tmux, switch to first witshe session; otherwise attach
            let sessions = tmux::list_sessions();
            let witshe_sessions: Vec<_> = sessions.iter().filter(|s| s.starts_with("witshe/")).collect();

            if witshe_sessions.is_empty() {
                println!("no witshe sessions. create one: witshe thread <name>");
                return;
            }

            let target = witshe_sessions[0];
            if std::env::var("TMUX").is_ok() {
                let _ = tmux::switch_to(target);
            } else {
                let _ = tmux::attach(target);
            }
        }
    }
}

fn continue_last(store: &Threads) {
    let alive = tmux::list_sessions();
    let witshe_sessions: Vec<_> = alive.iter().filter(|s| s.starts_with("witshe/")).collect();

    if witshe_sessions.is_empty() {
        if let Some(t) = store.list().iter().rev().find(|t| matches!(t.status, ThreadStatus::Active)) {
            let session_name = format!("witshe/{}", t.name);
            if alive.contains(&session_name) {
                do_switch(&session_name);
                return;
            }
        }
        println!("no active threads to continue");
        return;
    }

    let target = witshe_sessions.last().unwrap();
    do_switch(target);
}

fn resume_interactive(store: &Threads) {
    let threads = store.list();
    let alive = tmux::list_sessions();

    let active: Vec<_> = threads.iter()
        .filter(|t| matches!(t.status, ThreadStatus::Active))
        .filter(|t| alive.contains(&format!("witshe/{}", t.name)))
        .collect();

    if active.is_empty() {
        println!("no active sessions to resume");
        return;
    }

    println!("\x1b[1m  pick a thread:\x1b[0m\n");
    for (i, t) in active.iter().enumerate() {
        let tag_str = t.tag.as_ref().map(|tg| format!(" \x1b[36m[{}]\x1b[0m", tg)).unwrap_or_default();
        println!("  {}) {}{}", i + 1, t.name, tag_str);
        if let Some(ref desc) = t.desc {
            println!("     {}", desc);
        }
    }

    println!("\n  enter number (or q): ");

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return;
    }
    let input = input.trim();

    if input == "q" || input.is_empty() {
        return;
    }

    if let Ok(n) = input.parse::<usize>() {
        if n >= 1 && n <= active.len() {
            let target = format!("witshe/{}", active[n - 1].name);
            do_switch(&target);
        } else {
            eprintln!("  invalid choice");
        }
    } else {
        // Try as name
        let target = format!("witshe/{}", input);
        if alive.contains(&target) {
            do_switch(&target);
        } else {
            eprintln!("  thread not found: {}", input);
        }
    }
}

fn interactive(store: &mut Threads) {
    let threads = store.list();
    let alive = tmux::list_sessions();

    println!("\x1b[1mwitshe\x1b[0m — tmux + git worktrees\n");

    if threads.is_empty() {
        println!("  no threads yet\n");
        println!("  create one:  witshe thread <name>");
        println!("  with worktree: witshe thread <name> --tag <tag>");
        return;
    }

    // Show active threads
    let active: Vec<_> = threads.iter()
        .filter(|t| matches!(t.status, ThreadStatus::Active))
        .collect();

    let done: Vec<_> = threads.iter()
        .filter(|t| matches!(t.status, ThreadStatus::Done))
        .collect();

    if !active.is_empty() {
        println!("  \x1b[1mactive threads:\x1b[0m\n");
        for (i, t) in active.iter().enumerate() {
            let session_name = format!("witshe/{}", t.name);
            let is_alive = alive.contains(&session_name);
            let icon = if is_alive { "\x1b[32m●\x1b[0m" } else { "\x1b[90m✗\x1b[0m" };
            let tag_str = t.tag.as_ref().map(|tg| format!(" \x1b[36m[{}]\x1b[0m", tg)).unwrap_or_default();
            println!("  {}  {}) {}{}", icon, i + 1, t.name, tag_str);
            if let Some(ref desc) = t.desc {
                println!("        {}", desc);
            }
        }
        println!();
    }

    if !done.is_empty() {
        println!("  \x1b[90mdone: {} thread(s) (witshe ls --all)\x1b[0m\n", done.len());
    }

    println!("  commands:");
    println!("    witshe thread <name>   create new thread");
    println!("    witshe switch <name>   switch to thread");
    println!("    witshe done [name]     mark as done");
    println!("    witshe --resume        pick thread interactively");
    println!("    witshe --continue      jump to last active session");
    println!("    witshe overview        show status + last output");
}

fn quick_switch(store: &Threads, n: usize) {
    let threads = store.list();
    let active: Vec<_> = threads.iter()
        .filter(|t| matches!(t.status, ThreadStatus::Active))
        .collect();

    if n == 0 || n > active.len() {
        eprintln!("  invalid thread number: {} (have {} active)", n, active.len());
        std::process::exit(1);
    }

    let target = format!("witshe/{}", active[n - 1].name);
    do_switch(&target);
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

fn indent(text: &str, prefix: &str) -> String {
    text.lines().map(|l| format!("{}{}", prefix, l)).collect::<Vec<_>>().join("\n")
}
