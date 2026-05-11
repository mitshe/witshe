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
    let mut stdout = stdout();
    let mut prev_lines = 0u16;

    terminal::enable_raw_mode().ok()?;
    execute!(stdout, cursor::Hide).ok();

    let filtered = filter(items, &search);
    render(&mut stdout, title, items, &filtered, selected, &search, &mut prev_lines);

    loop {
        if let Ok(Event::Key(KeyEvent { code, modifiers, .. })) = event::read() {
            match code {
                KeyCode::Up => {
                    if selected > 0 { selected -= 1; }
                }
                KeyCode::Down => {
                    let f = filter(items, &search);
                    if !f.is_empty() && selected < f.len() - 1 { selected += 1; }
                }
                KeyCode::Enter => {
                    let f = filter(items, &search);
                    if let Some(&idx) = f.get(selected) {
                        cleanup(&mut stdout, prev_lines);
                        return Some(PickResult { index: idx, is_done: items[idx].is_done });
                    }
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

            let filtered = filter(items, &search);
            if !filtered.is_empty() && selected >= filtered.len() {
                selected = filtered.len() - 1;
            }

            move_up(&mut stdout, prev_lines);
            render(&mut stdout, title, items, &filtered, selected, &search, &mut prev_lines);
        }
    }
}

fn filter(items: &[PickerItem], search: &str) -> Vec<usize> {
    let q = search.to_lowercase();

    // Active first, then done — sorted order
    let mut active: Vec<usize> = Vec::new();
    let mut done: Vec<usize> = Vec::new();

    for (i, it) in items.iter().enumerate() {
        if !q.is_empty() {
            let matches = it.label.to_lowercase().contains(&q)
                || it.hint.to_lowercase().contains(&q)
                || it.desc.as_ref().map(|d| d.to_lowercase().contains(&q)).unwrap_or(false);
            if !matches { continue; }
        }
        if it.is_done { done.push(i); } else { active.push(i); }
    }

    active.extend(done);
    active
}

fn render(
    stdout: &mut impl Write,
    title: &str,
    items: &[PickerItem],
    filtered: &[usize],
    selected: usize,
    search: &str,
    prev_lines: &mut u16,
) {
    let mut lines = 0u16;

    // Title
    cln(stdout);
    write!(stdout, "\r\n").ok(); lines += 1;
    cln(stdout);
    write!(stdout, "  \x1b[1m{}\x1b[0m", title).ok();
    nl(stdout, &mut lines);

    // Search box
    cln(stdout);
    if search.is_empty() {
        write!(stdout, "  \x1b[90m╭─ ⌕ Search…\x1b[0m").ok();
    } else {
        write!(stdout, "  \x1b[36m╭─ ⌕ {}\x1b[0m", search).ok();
    }
    nl(stdout, &mut lines);

    // Blank line
    cln(stdout);
    nl(stdout, &mut lines);

    if filtered.is_empty() {
        cln(stdout);
        write!(stdout, "  \x1b[90m  no threads{}\x1b[0m",
            if !search.is_empty() { " matching search" } else { "" }
        ).ok();
        nl(stdout, &mut lines);
    } else {
        let max_visible = 8usize;
        let mut shown = 0;
        let mut in_done_section = false;

        for (i, &idx) in filtered.iter().enumerate() {
            if shown >= max_visible { break; }

            let item = &items[idx];

            // Section header when transitioning to done
            if item.is_done && !in_done_section {
                in_done_section = true;
                if shown > 0 {
                    cln(stdout);
                    nl(stdout, &mut lines);
                    cln(stdout);
                    write!(stdout, "  \x1b[90m  done\x1b[0m").ok();
                    nl(stdout, &mut lines);
                }
            }

            cln(stdout);

            // Build one-line: icon label [tag] desc
            let desc_part = item.desc.as_ref()
                .map(|d| format!(" \x1b[90m— {}\x1b[0m", truncate(d, 40)))
                .unwrap_or_default();

            if i == selected {
                execute!(stdout, SetForegroundColor(Color::Cyan), Print("  ❯ "), ResetColor).ok();
                write!(stdout, "{}", item.label).ok();
                if !item.hint.is_empty() {
                    execute!(stdout, SetForegroundColor(Color::DarkCyan), Print(format!(" {}", item.hint)), ResetColor).ok();
                }
                write!(stdout, "{}", desc_part).ok();
            } else {
                write!(stdout, "    \x1b[90m{}", item.label).ok();
                if !item.hint.is_empty() {
                    write!(stdout, " {}", item.hint).ok();
                }
                write!(stdout, "{}\x1b[0m", desc_part).ok();
            }
            nl(stdout, &mut lines);
            shown += 1;
        }

        if filtered.len() > max_visible {
            cln(stdout);
            write!(stdout, "  \x1b[90m  ... {} more\x1b[0m", filtered.len() - max_visible).ok();
            nl(stdout, &mut lines);
        }
    }

    // Bottom hints
    cln(stdout);
    nl(stdout, &mut lines);
    cln(stdout);
    write!(stdout, "  \x1b[90m↑↓ select · enter switch · type to search · esc quit\x1b[0m").ok();
    nl(stdout, &mut lines);

    *prev_lines = lines;
    stdout.flush().ok();
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
}

fn cln(stdout: &mut impl Write) {
    execute!(stdout, terminal::Clear(ClearType::CurrentLine)).ok();
}

fn nl(stdout: &mut impl Write, lines: &mut u16) {
    write!(stdout, "\r\n").ok();
    *lines += 1;
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
