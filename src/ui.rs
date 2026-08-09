use crossterm::cursor;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use std::io::{self, Write};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Default)]
pub struct Dashboard {
    pub world_name: String,
    pub character_line: String,
    pub hp_line: String,
    pub time_display: String,
    pub condition_line: Option<String>,
    pub location_name: Option<String>,
    pub location_description: Option<String>,
    pub danger_line: Option<String>,
    pub people_line: Option<String>,
    pub remains_line: Option<String>,
    pub exits_line: Option<String>,
    pub threat_line: Option<String>,
    pub reputation_line: Option<String>,
    pub action_hint: Option<String>,
}

#[derive(Default)]
struct UiRuntime {
    dashboard: Dashboard,
    log: Vec<String>,
    initialized: bool,
}

static UI: OnceLock<Mutex<UiRuntime>> = OnceLock::new();

pub struct UiGuard;

impl Drop for UiGuard {
    fn drop(&mut self) {
        let _ = restore_terminal();
    }
}

fn runtime() -> &'static Mutex<UiRuntime> {
    UI.get_or_init(|| Mutex::new(UiRuntime::default()))
}

pub fn init() -> io::Result<UiGuard> {
    enter_terminal()?;
    let mut state = runtime().lock().unwrap();
    state.initialized = true;
    render_locked(&state, None);
    Ok(UiGuard)
}

pub fn set_dashboard(dashboard: Dashboard) {
    let mut state = runtime().lock().unwrap();
    state.dashboard = dashboard;
    state.initialized = true;
    render_locked(&state, None);
}

pub fn line(text: &str) {
    let mut state = runtime().lock().unwrap();
    for part in text.split('\n') {
        state.log.push(part.to_string());
    }
    trim_log(&mut state.log);
    if state.initialized {
        render_locked(&state, None);
    } else {
        println!("{text}");
    }
}

pub fn diagnostic(text: &str) {
    line(&format!("[diagnostic] {text}"));
}

pub fn prompt(message: &str) -> io::Result<String> {
    if !runtime().lock().unwrap().initialized {
        println!("{message}");
        print!("> ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        return Ok(input.trim().to_string());
    }

    let mut state = runtime().lock().unwrap();
    let mut buffer = String::new();
    loop {
        let prompt_lines = vec![
            message.to_string(),
            String::new(),
            format!("> {buffer}"),
            "Enter to confirm, Esc to cancel.".to_string(),
        ];
        render_locked(&state, Some(&prompt_lines));
        drop(state);

        match read_key()? {
            KeyCode::Char(c) if !is_ctrl_char(c) => buffer.push(c),
            KeyCode::Backspace => {
                buffer.pop();
            }
            KeyCode::Delete => {}
            KeyCode::Enter => return Ok(buffer.trim().to_string()),
            KeyCode::Esc => return Ok(String::new()),
            KeyCode::Tab => buffer.push('\t'),
            _ => {}
        }

        state = runtime().lock().unwrap();
    }
}

pub fn pause() {
    let _ = wait_for_key("Press any key to continue...");
}

pub fn narrate(message: &str) {
    line(message);
    pause();
}

pub fn choose_from_list(
    title: &str,
    options: &[String],
    zero_label: Option<&str>,
) -> io::Result<Option<usize>> {
    if !runtime().lock().unwrap().initialized {
        let mut lines = Vec::with_capacity(options.len() + 2);
        lines.push(title.to_string());
        for (index, option) in options.iter().enumerate() {
            lines.push(format!("  {}. {}", index + 1, option));
        }
        if let Some(label) = zero_label {
            lines.push(format!("  0. {label}"));
        }
        return choose_via_stdin(&lines.join("\n"), options.len(), zero_label);
    }

    if options.is_empty() {
        return Ok(None);
    }

    let mut selected = 0usize;
    let back_index = options.len();
    let mut state = runtime().lock().unwrap();
    loop {
        let mut prompt_lines = Vec::new();
        prompt_lines.push(title.to_string());
        prompt_lines.push(String::new());
        prompt_lines.push("Use ↑ ↓ or j/k, Enter to confirm, Esc to go back.".to_string());
        prompt_lines.push(String::new());
        for (index, option) in options.iter().enumerate() {
            let marker = if index == selected { '▶' } else { ' ' };
            prompt_lines.push(format!("{marker} {}. {option}", index + 1));
        }
        if let Some(label) = zero_label {
            let marker = if selected == back_index { '▶' } else { ' ' };
            prompt_lines.push(format!("{marker} 0. {label}"));
        }

        render_locked(&state, Some(&prompt_lines));
        drop(state);

        match read_key()? {
            KeyCode::Up | KeyCode::Char('k') => {
                if selected == 0 {
                    selected = if zero_label.is_some() { back_index } else { options.len().saturating_sub(1) };
                } else {
                    selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                selected += 1;
                if selected > back_index {
                    selected = 0;
                }
                if selected == back_index && zero_label.is_none() {
                    selected = 0;
                }
            }
            KeyCode::Home => selected = 0,
            KeyCode::End => {
                selected = if zero_label.is_some() { back_index } else { options.len().saturating_sub(1) };
            }
            KeyCode::Enter => {
                return Ok(if selected == back_index && zero_label.is_some() {
                    None
                } else {
                    Some(selected)
                });
            }
            KeyCode::Esc => {
                if zero_label.is_some() {
                    return Ok(None);
                }
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if c == '0' && zero_label.is_some() {
                    return Ok(None);
                }
                if let Some(digit) = c.to_digit(10) {
                    let choice = digit as usize;
                    if choice >= 1 && choice <= options.len() {
                        return Ok(Some(choice - 1));
                    }
                }
            }
            _ => {}
        }

        state = runtime().lock().unwrap();
    }
}

fn choose_via_stdin(message: &str, option_count: usize, zero_label: Option<&str>) -> io::Result<Option<usize>> {
    println!("{message}");
    loop {
        print!("> ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        match trimmed.parse::<usize>() {
            Ok(0) if zero_label.is_some() => return Ok(None),
            Ok(choice) if choice >= 1 && choice <= option_count => return Ok(Some(choice - 1)),
            _ => line("Enter a valid number."),
        }
    }
}

fn wait_for_key(message: &str) -> io::Result<()> {
    if !runtime().lock().unwrap().initialized {
        println!("{message}");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        return Ok(());
    }

    let state = runtime().lock().unwrap();
    let prompt_lines = vec![message.to_string(), String::new(), "Press any key to continue.".to_string()];
    render_locked(&state, Some(&prompt_lines));
    drop(state);

    loop {
        match read_key()? {
            KeyCode::Char(c) if c.is_ascii_control() => continue,
            _ => return Ok(()),
        }
    }
}

fn read_key() -> io::Result<KeyCode> {
    loop {
        if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
            if modifiers.contains(KeyModifiers::CONTROL) && matches!(code, KeyCode::Char('c')) {
                return Ok(KeyCode::Esc);
            }
            return Ok(code);
        }
    }
}

fn is_ctrl_char(ch: char) -> bool {
    ch.is_control() && ch != '\t'
}

fn enter_terminal() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut out = io::stdout();
    execute!(out, EnterAlternateScreen, cursor::Hide, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
    Ok(())
}

fn restore_terminal() -> io::Result<()> {
    let _ = terminal::disable_raw_mode();
    let mut out = io::stdout();
    execute!(out, cursor::Show, LeaveAlternateScreen)?;
    Ok(())
}

fn trim_log(log: &mut Vec<String>) {
    const MAX_LOG_LINES: usize = 14;
    if log.len() > MAX_LOG_LINES {
        let excess = log.len() - MAX_LOG_LINES;
        log.drain(0..excess);
    }
}

fn render_locked(state: &UiRuntime, prompt: Option<&[String]>) {
    let width = terminal_width().clamp(72, 120);
    let mut out = io::stdout();
    let _ = out.write_all(b"\x1b[?25l\x1b[2J\x1b[H");

    let mut lines = Vec::new();
    lines.push(center_text("The Ashen Chronicle", width));
    lines.push(repeat_char('=', width));
    lines.push(format!("World: {}", state.dashboard.world_name));
    if !state.dashboard.character_line.is_empty() {
        lines.push(state.dashboard.character_line.clone());
    }
    if !state.dashboard.hp_line.is_empty() {
        lines.push(state.dashboard.hp_line.clone());
    }
    if !state.dashboard.time_display.is_empty() {
        lines.push(String::new());
        lines.extend(state.dashboard.time_display.lines().map(|line| line.to_string()));
    }

    push_box(
        &mut lines,
        "Current place",
        vec![
            state.dashboard.location_name.clone().unwrap_or_else(|| "Unknown location".to_string()),
            state.dashboard.location_description.clone().unwrap_or_default(),
        ],
        width,
    );

    let mut status_lines = Vec::new();
    if let Some(line) = &state.dashboard.condition_line {
        status_lines.push(line.clone());
    }
    if let Some(line) = &state.dashboard.danger_line {
        status_lines.push(line.clone());
    }
    if let Some(line) = &state.dashboard.threat_line {
        status_lines.push(line.clone());
    }
    if let Some(line) = &state.dashboard.reputation_line {
        status_lines.push(line.clone());
    }
    if !status_lines.is_empty() {
        push_box(&mut lines, "Status", status_lines, width);
    }

    let mut context_lines = Vec::new();
    if let Some(line) = &state.dashboard.people_line {
        context_lines.push(line.clone());
    }
    if let Some(line) = &state.dashboard.remains_line {
        context_lines.push(line.clone());
    }
    if let Some(line) = &state.dashboard.exits_line {
        context_lines.push(line.clone());
    }
    if !context_lines.is_empty() {
        push_box(&mut lines, "Nearby", context_lines, width);
    }

    push_box(&mut lines, "Log", state.log.clone(), width);

    if let Some(hint) = &state.dashboard.action_hint {
        push_box(&mut lines, "Controls", vec![hint.clone()], width);
    }

    if let Some(prompt_lines) = prompt {
        push_box(&mut lines, "Input", prompt_lines.to_vec(), width);
    }

    for line in lines {
        let _ = writeln!(out, "{}", line);
    }
    let _ = out.flush();
}

fn push_box(lines: &mut Vec<String>, title: &str, content: Vec<String>, width: usize) {
    let inner_width = width.saturating_sub(4).max(20);
    lines.push(format!("+{}+", repeat_char('-', inner_width + 2)));
    lines.push(format!("| {:^inner_width$} |", title, inner_width = inner_width + 1));
    lines.push(format!("+{}+", repeat_char('-', inner_width + 2)));
    for paragraph in content {
        if paragraph.is_empty() {
            lines.push(format!("| {:inner_width$} |", "", inner_width = inner_width + 1));
            continue;
        }
        for wrapped in wrap_text(&paragraph, inner_width + 1) {
            lines.push(format!("| {:inner_width$} |", wrapped, inner_width = inner_width + 1));
        }
    }
    lines.push(format!("+{}+", repeat_char('-', inner_width + 2)));
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            out.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn terminal_width() -> usize {
    terminal::size().map(|(width, _)| width as usize).unwrap_or(80)
}

fn repeat_char(ch: char, count: usize) -> String {
    std::iter::repeat(ch).take(count).collect()
}

fn center_text(text: &str, width: usize) -> String {
    let text_len = text.chars().count();
    if text_len >= width {
        return text.to_string();
    }
    let padding = width - text_len;
    let left = padding / 2;
    let right = padding - left;
    format!("{}{}{}", repeat_char(' ', left), text, repeat_char(' ', right))
}
