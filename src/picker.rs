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
}

/// Interactive picker. Returns index of selected item, or None if cancelled.
pub fn pick(title: &str, items: &[PickerItem], footer: Option<&str>) -> Option<usize> {
    if items.is_empty() {
        return None;
    }

    let mut selected = 0;
    let mut stdout = stdout();

    terminal::enable_raw_mode().ok()?;
    execute!(stdout, cursor::Hide).ok();

    draw(&mut stdout, title, items, selected, footer);

    loop {
        if let Ok(Event::Key(KeyEvent { code, modifiers, .. })) = event::read() {
            match code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if selected > 0 { selected -= 1; }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if selected < items.len() - 1 { selected += 1; }
                }
                KeyCode::Enter => {
                    cleanup(&mut stdout, items, footer);
                    terminal::disable_raw_mode().ok();
                    execute!(stdout, cursor::Show).ok();
                    return Some(selected);
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    cleanup(&mut stdout, items, footer);
                    terminal::disable_raw_mode().ok();
                    execute!(stdout, cursor::Show).ok();
                    return None;
                }
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                    cleanup(&mut stdout, items, footer);
                    terminal::disable_raw_mode().ok();
                    execute!(stdout, cursor::Show).ok();
                    return None;
                }
                _ => {}
            }
            draw(&mut stdout, title, items, selected, footer);
        }
    }
}

fn line_count(items: &[PickerItem], footer: Option<&str>) -> u16 {
    let mut count = 2u16; // title + empty line
    for item in items {
        count += 1;
        if item.desc.is_some() { count += 1; }
    }
    if footer.is_some() { count += 2; } // empty line + footer
    count += 2; // empty line + controls
    count
}

fn draw(stdout: &mut impl Write, title: &str, items: &[PickerItem], selected: usize, footer: Option<&str>) {
    let lines = line_count(items, footer);
    execute!(stdout, cursor::MoveToColumn(0)).ok();

    for _ in 0..lines {
        execute!(stdout, terminal::Clear(ClearType::CurrentLine), cursor::MoveUp(0)).ok();
    }
    execute!(stdout, cursor::MoveToColumn(0)).ok();

    // Title
    execute!(stdout, Print(format!("\x1b[1m  {}\x1b[0m\r\n\r\n", title))).ok();

    // Items
    for (i, item) in items.iter().enumerate() {
        if i == selected {
            execute!(
                stdout,
                SetForegroundColor(Color::Green),
                Print(format!("  > {}", item.label)),
                ResetColor,
            ).ok();
            if !item.hint.is_empty() {
                execute!(
                    stdout,
                    SetForegroundColor(Color::DarkCyan),
                    Print(format!(" {}", item.hint)),
                    ResetColor,
                ).ok();
            }
        } else {
            execute!(
                stdout,
                SetForegroundColor(Color::DarkGrey),
                Print(format!("    {}", item.label)),
            ).ok();
            if !item.hint.is_empty() {
                execute!(
                    stdout,
                    Print(format!(" {}", item.hint)),
                ).ok();
            }
            execute!(stdout, ResetColor).ok();
        }
        execute!(stdout, Print("\r\n")).ok();

        if let Some(ref desc) = item.desc {
            let color = if i == selected { Color::White } else { Color::DarkGrey };
            execute!(
                stdout,
                SetForegroundColor(color),
                Print(format!("      {}", desc)),
                ResetColor,
                Print("\r\n"),
            ).ok();
        }
    }

    if let Some(f) = footer {
        execute!(
            stdout,
            Print("\r\n"),
            SetForegroundColor(Color::DarkGrey),
            Print(format!("  {}", f)),
            ResetColor,
        ).ok();
    }

    execute!(
        stdout,
        Print("\r\n\r\n"),
        SetForegroundColor(Color::DarkGrey),
        Print("  ↑↓ select  enter switch  q quit"),
        ResetColor,
    ).ok();

    stdout.flush().ok();
}

fn cleanup(stdout: &mut impl Write, items: &[PickerItem], footer: Option<&str>) {
    let lines = line_count(items, footer);
    for _ in 0..lines {
        execute!(stdout, terminal::Clear(ClearType::CurrentLine), cursor::MoveUp(1)).ok();
    }
    execute!(stdout, terminal::Clear(ClearType::CurrentLine), cursor::MoveToColumn(0)).ok();
}
