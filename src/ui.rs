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
    let mut state = runtime().lock().unwrap();
    if state.initialized {
        render_locked(&state, Some(&[message.to_string()]));
        let mut out = io::stdout();
        out.write_all(b"\x1b[?25h> ")?;
        out.flush()?;
    } else {
        println!("{message}");
        print!("> ");
        io::stdout().flush()?;
    }
    drop(state);

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

pub fn pause() {
    let _ = prompt("Press Enter to continue...");
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
    loop {
        let mut lines = Vec::with_capacity(options.len() + 2);
        lines.push(title.to_string());
        for (index, option) in options.iter().enumerate() {
            lines.push(format!("  {}. {}", index + 1, option));
        }
        if let Some(label) = zero_label {
            lines.push(format!("  0. {label}"));
        }

        let input = prompt(&lines.join("\n"))?;
        match input.parse::<usize>() {
            Ok(0) if zero_label.is_some() => return Ok(None),
            Ok(choice) if choice >= 1 && choice <= options.len() => return Ok(Some(choice - 1)),
            _ => line("Enter a valid number."),
        }
    }
}

fn enter_terminal() -> io::Result<()> {
    let mut out = io::stdout();
    out.write_all(b"\x1b[?1049h\x1b[?25l\x1b[2J\x1b[H")?;
    out.flush()?;
    Ok(())
}

fn restore_terminal() -> io::Result<()> {
    let mut out = io::stdout();
    out.write_all(b"\x1b[?25h\x1b[?1049l")?;
    out.flush()?;
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

    push_box(&mut lines, "Current place", vec![
        state.dashboard.location_name.clone().unwrap_or_else(|| "Unknown location".to_string()),
        state.dashboard.location_description.clone().unwrap_or_default(),
    ], width);

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

fn repeat_char(ch: char, count: usize) -> String {
    std::iter::repeat(ch).take(count).collect()
}

fn center_text(text: &str, width: usize) -> String {
    let len = text.chars().count();
    if len >= width {
        return text.chars().take(width).collect();
    }
    let left = (width - len) / 2;
    format!("{}{}", repeat_char(' ', left), text)
}

fn terminal_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value >= 40)
        .unwrap_or(100)
}
