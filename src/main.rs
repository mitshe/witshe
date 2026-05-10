mod threads;
mod tmux;

use clap::{Parser, Subcommand};
use threads::{ThreadStatus, Threads};

#[derive(Parser)]
#[command(name = "witshe", about = "tmux + git worktrees = threads")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Pick thread interactively
    #[arg(long)]
    resume: bool,

    /// Jump to last active session
    #[arg(long, alias = "c")]
    r#continue: bool,

    /// Quick switch by number
    #[arg(value_name = "N")]
    number: Option<usize>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new thread (worktree + tmux session)
    Thread {
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
    /// List all threads
    Ls {
        #[arg(short, long)]
        all: bool,
    },
    /// Switch to a thread
    Switch { name: String },
    /// Mark thread as done (auto-detects current thread)
    Done { name: Option<String> },
    /// Remove thread permanently
    Rm {
        name: String,
        #[arg(long)]
        keep_worktree: bool,
    },
    /// Rename current (or specified) thread
    Name {
        new_name: String,
        #[arg(short, long)]
        thread: Option<String>,
    },
    /// Set/change tag
    Tag {
        value: String,
        #[arg(short, long)]
        thread: Option<String>,
    },
    /// Set/change description
    Desc {
        value: String,
        #[arg(short, long)]
        thread: Option<String>,
    },
    /// Clear all done threads from history
    Clear,
}

fn main() {
    let cli = Cli::parse();
    let mut store = Threads::load();

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
                interactive(&store);
            }
            return;
        }
    };

    match command {
        Commands::Thread { name, repo, no_worktree, tag, desc } => {
            let repo_path = repo.unwrap_or_else(|| {
                std::env::current_dir().unwrap().to_string_lossy().to_string()
            });

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

            let session_name = format!("witshe/{}", name);
            if let Err(e) = tmux::create_session(&session_name, &worktree_path) {
                eprintln!("error creating tmux session: {}", e);
                std::process::exit(1);
            }

            let thread = threads::Thread::new(name.clone(), repo_path, worktree_path, !no_worktree, tag, desc);
            store.add(thread);
            store.save();

            println!("  thread: {}", name);
            println!("  switch: witshe switch {}", name);
        }

        Commands::Ls { all } => {
            let threads = store.list();
            if threads.is_empty() {
                println!("no threads. create one: witshe thread <name>");
                return;
            }

            let alive = tmux::list_sessions();

            for t in threads {
                if !all && matches!(t.status, ThreadStatus::Done) {
                    continue;
                }
                let session = format!("witshe/{}", t.name);
                let is_alive = alive.contains(&session);

                let icon = match (&t.status, is_alive) {
                    (ThreadStatus::Done, _) => "✓",
                    (_, true) => "●",
                    (_, false) => "✗",
                };

                let tag_str = t.tag.as_ref().map(|tg| format!(" \x1b[36m[{}]\x1b[0m", tg)).unwrap_or_default();
                println!("  {} {}{}", icon, t.name, tag_str);
                if let Some(ref desc) = t.desc {
                    println!("    {}", desc);
                }
            }
        }

        Commands::Switch { name } => do_switch(&format!("witshe/{}", name)),

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

        Commands::Name { new_name, thread } => {
            let name = resolve_thread(thread);
            let _ = tmux::rename_session(&format!("witshe/{}", name), &format!("witshe/{}", new_name));

            if store.rename(&name, &new_name) {
                store.save();
                println!("  renamed: {} -> {}", name, new_name);
            } else {
                eprintln!("thread not found: {}", name);
                std::process::exit(1);
            }
        }

        Commands::Tag { value, thread } => {
            let name = resolve_thread(thread);
            if store.set_tag(&name, &value) {
                store.save();
                println!("  tag: {} -> [{}]", name, value);
            } else {
                eprintln!("thread not found: {}", name);
                std::process::exit(1);
            }
        }

        Commands::Desc { value, thread } => {
            let name = resolve_thread(thread);
            if store.set_desc(&name, &value) {
                store.save();
                println!("  desc: {} -> {}", name, value);
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

fn interactive(store: &Threads) {
    let threads = store.list();
    let alive = tmux::list_sessions();

    println!("\x1b[1mwitshe\x1b[0m — tmux + git worktrees\n");

    if threads.is_empty() {
        println!("  no threads yet\n");
        println!("  witshe thread <name>    create one");
        return;
    }

    let active: Vec<_> = threads.iter().filter(|t| matches!(t.status, ThreadStatus::Active)).collect();
    let done_count = threads.iter().filter(|t| matches!(t.status, ThreadStatus::Done)).count();

    for (i, t) in active.iter().enumerate() {
        let session = format!("witshe/{}", t.name);
        let icon = if alive.contains(&session) { "\x1b[32m●\x1b[0m" } else { "\x1b[90m✗\x1b[0m" };
        let tag_str = t.tag.as_ref().map(|tg| format!(" \x1b[36m[{}]\x1b[0m", tg)).unwrap_or_default();
        println!("  {}  {}) {}{}", icon, i + 1, t.name, tag_str);
        if let Some(ref desc) = t.desc {
            println!("        {}", desc);
        }
    }

    if done_count > 0 {
        println!("\n  \x1b[90m+ {} done (witshe ls --all)\x1b[0m", done_count);
    }
}

fn quick_switch(store: &Threads, n: usize) {
    let active: Vec<_> = store.list().iter()
        .filter(|t| matches!(t.status, ThreadStatus::Active))
        .cloned()
        .collect();

    if n == 0 || n > active.len() {
        eprintln!("invalid thread number: {}", n);
        std::process::exit(1);
    }

    do_switch(&format!("witshe/{}", active[n - 1].name));
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

fn resume_interactive(store: &Threads) {
    let alive = tmux::list_sessions();
    let active: Vec<_> = store.list().iter()
        .filter(|t| matches!(t.status, ThreadStatus::Active) && alive.contains(&format!("witshe/{}", t.name)))
        .cloned()
        .collect();

    if active.is_empty() {
        println!("no active sessions");
        return;
    }

    println!("\x1b[1m  pick a thread:\x1b[0m\n");
    for (i, t) in active.iter().enumerate() {
        let tag_str = t.tag.as_ref().map(|tg| format!(" \x1b[36m[{}]\x1b[0m", tg)).unwrap_or_default();
        println!("  {}) {}{}", i + 1, t.name, tag_str);
    }
    println!();

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() { return; }
    let input = input.trim();
    if input == "q" || input.is_empty() { return; }

    if let Ok(n) = input.parse::<usize>() {
        if n >= 1 && n <= active.len() {
            do_switch(&format!("witshe/{}", active[n - 1].name));
        }
    }
}
