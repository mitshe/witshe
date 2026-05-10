use crossterm::{
    cursor, execute,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{self, ClearType},
    style::{Print, SetForegroundColor, ResetColor, Color},
};
use std::io::{stdout, Write};

pub struct PickerItem {
    pub label: String,
    pub hint: String,
    pub desc: Option<String>,
    pub is_done: bool,
}

pub struct PickResult {
    pub index: usize,
    pub is_done: bool,
}

pub fn pick(title: &str, items: &[PickerItem]) -> Option<PickResult> {
    if items.is_empty() {
        return None;
    }

    let mut selected: usize = 0;
    let mut search = String::new();
    let mut show_done = false;
    let mut stdout = stdout();
    let mut prev_lines = 0u16;

    terminal::enable_raw_mode().ok()?;
    execute!(stdout, cursor::Hide).ok();

    let filtered = filter(items, &search, show_done);
    render(&mut stdout, title, items, &filtered, selected, &search, show_done, &mut prev_lines);

    loop {
        if let Ok(Event::Key(KeyEvent { code, modifiers, .. })) = event::read() {
            match code {
                KeyCode::Up => {
                    if selected > 0 { selected -= 1; }
                }
                KeyCode::Down => {
                    let f = filter(items, &search, show_done);
                    if !f.is_empty() && selected < f.len() - 1 { selected += 1; }
                }
                KeyCode::Enter => {
                    let f = filter(items, &search, show_done);
                    if let Some(&idx) = f.get(selected) {
                        cleanup(&mut stdout, prev_lines);
                        return Some(PickResult { index: idx, is_done: items[idx].is_done });
                    }
                }
                KeyCode::Tab => {
                    show_done = !show_done;
                    selected = 0;
                }
                KeyCode::Esc => {
                    if !search.is_empty() {
                        search.clear();
                        selected = 0;
                    } else {
                        cleanup(&mut stdout, prev_lines);
                        return None;
                    }
                }
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                    cleanup(&mut stdout, prev_lines);
                    return None;
                }
                KeyCode::Backspace => {
                    search.pop();
                    selected = 0;
                }
                KeyCode::Char(c) if !modifiers.contains(KeyModifiers::CONTROL) => {
                    search.push(c);
                    selected = 0;
                }
                _ => continue,
            }

            let filtered = filter(items, &search, show_done);
            if !filtered.is_empty() && selected >= filtered.len() {
                selected = filtered.len() - 1;
            }

            move_up(&mut stdout, prev_lines);
            render(&mut stdout, title, items, &filtered, selected, &search, show_done, &mut prev_lines);
        }
    }
}

fn filter(items: &[PickerItem], search: &str, show_done: bool) -> Vec<usize> {
    let q = search.to_lowercase();
    items.iter().enumerate()
        .filter(|(_, it)| {
            if show_done != it.is_done { return false; }
            if q.is_empty() { return true; }
            it.label.to_lowercase().contains(&q)
                || it.hint.to_lowercase().contains(&q)
                || it.desc.as_ref().map(|d| d.to_lowercase().contains(&q)).unwrap_or(false)
        })
        .map(|(i, _)| i)
        .collect()
}

fn render(
    stdout: &mut impl Write,
    title: &str,
    items: &[PickerItem],
    filtered: &[usize],
    selected: usize,
    search: &str,
    show_done: bool,
    prev_lines: &mut u16,
) {
    let mut lines = 0u16;

    macro_rules! cl {
        ($out:expr) => { execute!($out, terminal::Clear(ClearType::CurrentLine)).ok(); }
    }
    macro_rules! nl {
        ($out:expr, $lines:expr) => { write!($out, "\r\n").ok(); $lines += 1; }
    }

    // Title
    cl!(stdout);
    write!(stdout, "\r\n").ok(); lines += 1;
    cl!(stdout);
    write!(stdout, "  \x1b[1m{}\x1b[0m", title).ok();
    nl!(stdout, lines);

    // Search box
    cl!(stdout);
    write!(stdout, "  \x1b[90m╭────────────────────────────────────────╮\x1b[0m").ok();
    nl!(stdout, lines);

    cl!(stdout);
    if search.is_empty() {
        write!(stdout, "  \x1b[90m│ ⌕ Search…                              │\x1b[0m").ok();
    } else {
        write!(stdout, "  \x1b[90m│\x1b[0m \x1b[36m⌕ {}\x1b[0m", search).ok();
        let pad = 38usize.saturating_sub(search.len() + 2);
        write!(stdout, "{}\x1b[90m│\x1b[0m", " ".repeat(pad)).ok();
    }
    nl!(stdout, lines);

    cl!(stdout);
    write!(stdout, "  \x1b[90m╰────────────────────────────────────────╯\x1b[0m").ok();
    nl!(stdout, lines);

    // Section
    cl!(stdout);
    nl!(stdout, lines);

    let section = if show_done { "done" } else { "active" };
    let active_count = items.iter().filter(|i| !i.is_done).count();
    let done_count = items.iter().filter(|i| i.is_done).count();

    cl!(stdout);
    if show_done {
        write!(stdout, "  \x1b[90mactive ({}) ·\x1b[0m \x1b[1mdone ({})\x1b[0m", active_count, done_count).ok();
    } else {
        write!(stdout, "  \x1b[1mactive ({})\x1b[0m \x1b[90m· done ({})\x1b[0m", active_count, done_count).ok();
    }
    nl!(stdout, lines);

    cl!(stdout);
    nl!(stdout, lines);

    // Items
    if filtered.is_empty() {
        cl!(stdout);
        write!(stdout, "  \x1b[90m  no {} threads{}\x1b[0m",
            section,
            if !search.is_empty() { " matching search" } else { "" }
        ).ok();
        nl!(stdout, lines);
    } else {
        for (i, &idx) in filtered.iter().enumerate() {
            let item = &items[idx];

            cl!(stdout);
            if i == selected {
                execute!(stdout, SetForegroundColor(Color::Cyan), Print("  ❯ "), ResetColor).ok();
                write!(stdout, "{}", item.label).ok();
                if !item.hint.is_empty() {
                    execute!(stdout, SetForegroundColor(Color::DarkCyan), Print(format!(" {}", item.hint)), ResetColor).ok();
                }
            } else {
                write!(stdout, "    \x1b[90m{}", item.label).ok();
                if !item.hint.is_empty() {
                    write!(stdout, " {}", item.hint).ok();
                }
                write!(stdout, "\x1b[0m").ok();
            }
            nl!(stdout, lines);

            if let Some(ref desc) = item.desc {
                cl!(stdout);
                if i == selected {
                    write!(stdout, "      {}", desc).ok();
                } else {
                    write!(stdout, "      \x1b[90m{}\x1b[0m", desc).ok();
                }
                nl!(stdout, lines);
            }
        }
    }

    // Bottom hints
    cl!(stdout);
    nl!(stdout, lines);

    cl!(stdout);
    let tab_hint = if show_done { "tab active" } else { "tab done" };
    write!(stdout, "  \x1b[90m↑↓ select · enter switch · {} · type to search · esc quit\x1b[0m", tab_hint).ok();
    nl!(stdout, lines);

    *prev_lines = lines;
    stdout.flush().ok();
}

fn cleanup(stdout: &mut impl Write, lines: u16) {
    move_up(stdout, lines);
    for _ in 0..=lines {
        execute!(stdout, terminal::Clear(ClearType::CurrentLine), cursor::MoveDown(1)).ok();
    }
    move_up(stdout, lines + 1);
    execute!(stdout, cursor::MoveToColumn(0)).ok();
    terminal::disable_raw_mode().ok();
    execute!(stdout, cursor::Show).ok();
}

fn move_up(stdout: &mut impl Write, lines: u16) {
    for _ in 0..lines {
        execute!(stdout, cursor::MoveUp(1), cursor::MoveToColumn(0)).ok();
    }
}
