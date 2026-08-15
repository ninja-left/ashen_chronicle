use crossterm::cursor;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect, Spacing};
use ratatui::prelude::{Color, Modifier, Style};
use ratatui::symbols::merge::MergeStrategy;
use ratatui::widgets::{Block, Borders, Clear, LineGauge, Paragraph, Wrap};
use ratatui::Terminal;
use std::io::{self, Stdout, Write};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Default)]
pub struct Dashboard {
    pub world_name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub enemy_name: Option<String>,
    pub enemy_hp: Option<i32>,
    pub enemy_max_hp: Option<i32>,
    pub time_display: String,
    pub condition_line: Option<String>,
    pub location_name: Option<String>,
    pub location_description: Option<String>,
    pub danger_line: Option<String>,
    pub threat_line: Option<String>,
    pub action_hint: Option<String>,
}

#[derive(Clone, Default)]
struct MenuScreen {
    title: String,
    subtitle: Option<String>,
    art: Option<String>,
}

#[derive(Default)]
struct UiRuntime {
    dashboard: Dashboard,
    menu_screen: Option<MenuScreen>,
    location_scene: Vec<String>,
    log: Vec<String>,
    initialized: bool,
    terminal: Option<Terminal<CrosstermBackend<Stdout>>>,
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

fn is_compact_area(area: Rect) -> bool {
    area.width <= 112 || area.height <= 36 || area.width <= area.height.saturating_mul(2)
}

fn bottom_panel_height(area: Rect, compact: bool, content_lines: usize) -> u16 {
    let base_height = if compact { 7 } else { 6 };
    let max_height = if compact {
        area.height.saturating_mul(40) / 100
    } else {
        area.height.saturating_mul(34) / 100
    };
    let desired = content_lines as u16 + 4;
    desired.clamp(base_height, max_height.max(base_height))
}

pub fn init() -> io::Result<UiGuard> {
    enter_terminal()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut state = runtime().lock().unwrap();
    state.initialized = true;
    state.terminal = Some(terminal);
    render_locked(&mut state, None, None)?;
    Ok(UiGuard)
}

pub fn set_menu_screen(title: impl Into<String>, subtitle: Option<String>, art: Option<String>) {
    let mut state = runtime().lock().unwrap();
    state.menu_screen = Some(MenuScreen {
        title: title.into(),
        subtitle,
        art,
    });
    state.log.clear();
    if state.initialized {
        let _ = render_locked(&mut state, None, None);
    }
}

pub fn set_dashboard(dashboard: Dashboard) {
    let mut state = runtime().lock().unwrap();
    state.dashboard = dashboard;
    state.menu_screen = None;
    state.initialized = true;
    let _ = render_locked(&mut state, None, None);
}

pub fn set_player_health(current: i32, maximum: i32) {
    let mut state = runtime().lock().unwrap();
    state.dashboard.hp = current;
    state.dashboard.max_hp = maximum.max(1);
    if state.initialized {
        let _ = render_locked(&mut state, None, None);
    }
}

pub fn set_combat_health(enemy_name: impl Into<String>, enemy_hp: i32, enemy_max_hp: i32) {
    let mut state = runtime().lock().unwrap();
    state.dashboard.enemy_name = Some(enemy_name.into());
    state.dashboard.enemy_hp = Some(enemy_hp.max(0));
    state.dashboard.enemy_max_hp = Some(enemy_max_hp.max(1));
    if state.initialized {
        let _ = render_locked(&mut state, None, None);
    }
}

pub fn clear_combat_health() {
    let mut state = runtime().lock().unwrap();
    state.dashboard.enemy_name = None;
    state.dashboard.enemy_hp = None;
    state.dashboard.enemy_max_hp = None;
    if state.initialized {
        let _ = render_locked(&mut state, None, None);
    }
}

pub fn set_location_scene(lines: Vec<String>) {
    let mut state = runtime().lock().unwrap();
    state.location_scene = lines;
    if state.initialized {
        let _ = render_locked(&mut state, None, None);
    }
}

pub fn line(text: &str) {
    let mut state = runtime().lock().unwrap();
    for part in text.split('\n') {
        state.log.push(part.to_string());
    }
    trim_log(&mut state.log);
    if state.initialized {
        let _ = render_locked(&mut state, None, None);
    } else {
        println!("{text}");
    }
}

pub fn clear_log() {
    let mut state = runtime().lock().unwrap();
    state.log.clear();
    if state.initialized {
        let _ = render_locked(&mut state, None, None);
    }
}

pub fn diagnostic(text: &str) {
    line(&format!("[diagnostic] {text}"));
}

pub fn prompt(message: &str) -> io::Result<String> {
    if !runtime().lock().unwrap().initialized {
        if !message.is_empty() {
            println!("{message}");
        };
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
        let _ = render_locked(&mut state, Some(&prompt_lines), None);
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
        let mut lines = vec![title.to_string()];
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
        let (term_width, term_height) = terminal::size().unwrap_or((100, 40));
        let compact = is_compact_area(Rect {
            x: 0,
            y: 0,
            width: term_width,
            height: term_height,
        });
        let popup_height = if compact { 42 } else { 34 };
        let inner_height = ((term_height as u32 * popup_height as u32) / 100) as usize;
        let mut available_option_rows = inner_height.saturating_sub(6);
        if zero_label.is_some() {
            available_option_rows = available_option_rows.saturating_sub(1);
        }
        let visible_rows = available_option_rows.max(1);
        let total_rows = options.len() + usize::from(zero_label.is_some());
        let mut start_index = selected.saturating_sub(visible_rows / 2);
        let max_start = total_rows.saturating_sub(visible_rows);
        if start_index > max_start {
            start_index = max_start;
        }
        let end_index = (start_index + visible_rows).min(total_rows);

        let mut prompt_lines = vec![
            title.to_string(),
            String::new(),
            "Use ↑ ↓ or j/k, Enter to confirm, Esc to go back.".to_string(),
            String::new(),
        ];
        if start_index > 0 {
            prompt_lines.push("⋯ more above ⋯".to_string());
        }
        for row in start_index..end_index {
            if row < options.len() {
                let marker = if row == selected { '▶' } else { ' ' };
                prompt_lines.push(format!("{marker} {}. {}", row + 1, options[row]));
            } else if zero_label.is_some() {
                let marker = if row == selected { '▶' } else { ' ' };
                prompt_lines.push(format!("{marker} 0. {}", zero_label.unwrap()));
            }
        }
        if end_index < total_rows {
            prompt_lines.push("⋯ more below ⋯".to_string());
        }

        let _ = render_locked(&mut state, Some(&prompt_lines), None);
        drop(state);

        match read_key()? {
            KeyCode::Up | KeyCode::Char('k') => {
                if selected == 0 {
                    selected = if zero_label.is_some() {
                        back_index
                    } else {
                        options.len().saturating_sub(1)
                    };
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
                selected = if zero_label.is_some() {
                    back_index
                } else {
                    options.len().saturating_sub(1)
                };
            }
            KeyCode::Enter => {
                let choice = if selected == back_index && zero_label.is_some() {
                    None
                } else {
                    Some(selected)
                };
                clear_log();
                return Ok(choice);
            }
            KeyCode::Esc => {
                if zero_label.is_some() {
                    clear_log();
                    return Ok(None);
                }
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if c == '0' && zero_label.is_some() {
                    clear_log();
                    return Ok(None);
                }
                if let Some(digit) = c.to_digit(10) {
                    let choice = digit as usize;
                    if choice >= 1 && choice <= options.len() {
                        clear_log();
                        return Ok(Some(choice - 1));
                    }
                }
            }
            _ => {}
        }

        state = runtime().lock().unwrap();
    }
}

fn choose_via_stdin(
    message: &str,
    option_count: usize,
    zero_label: Option<&str>,
) -> io::Result<Option<usize>> {
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

    let mut state = runtime().lock().unwrap();
    let _ = render_locked(&mut state, None, Some(message));
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
        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event::read()?
        {
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
    execute!(out, EnterAlternateScreen, cursor::Hide)?;
    Ok(())
}

fn restore_terminal() -> io::Result<()> {
    let _ = terminal::disable_raw_mode();
    let mut out = io::stdout();
    execute!(out, cursor::Show, LeaveAlternateScreen)?;
    Ok(())
}

fn trim_log(log: &mut Vec<String>) {
    const MAX_LOG_LINES: usize = 48;
    if log.len() > MAX_LOG_LINES {
        let excess = log.len() - MAX_LOG_LINES;
        log.drain(0..excess);
    }
}

fn render_locked(
    state: &mut UiRuntime,
    prompt: Option<&[String]>,
    notice: Option<&str>,
) -> io::Result<()> {
    let dashboard = state.dashboard.clone();
    let menu_screen = state.menu_screen.clone();
    let scene = state.location_scene.clone();
    let log = state.log.clone();
    let Some(terminal) = state.terminal.as_mut() else {
        return Ok(());
    };

    terminal.draw(|frame| {
        let area = frame.area();
        frame.render_widget(Clear, area);
        if let Some(menu_screen) = menu_screen.as_ref() {
            draw_menu_screen(frame, area, menu_screen, prompt);
        } else {
            draw_dashboard(frame, area, &dashboard, &scene, &log, prompt, notice);
        }
    })?;
    Ok(())
}

fn draw_menu_screen(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    menu: &MenuScreen,
    prompt: Option<&[String]>,
) {
    let compact = is_compact_area(area);
    let horizontal_margin = if compact { 1 } else { area.width / 10 };
    let vertical_margin = if compact { 1 } else { area.height / 10 };
    let outer = Rect {
        x: area.x + horizontal_margin.min(area.width.saturating_sub(1)),
        y: area.y + vertical_margin.min(area.height.saturating_sub(1)),
        width: area
            .width
            .saturating_sub(horizontal_margin.saturating_mul(2))
            .max(1),
        height: area
            .height
            .saturating_sub(vertical_margin.saturating_mul(2))
            .max(1),
    };

    let title = Paragraph::new(menu.title.clone())
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(border_style(compact))
                .merge_borders(MergeStrategy::Exact),
        );
    frame.render_widget(title, outer);

    let inner = outer.inner(ratatui::layout::Margin {
        vertical: 2,
        horizontal: 3,
    });
    let mut lines = Vec::new();
    if let Some(art) = &menu.art {
        lines.extend(art.lines().map(str::to_string));
        lines.push(String::new());
    }
    if let Some(subtitle) = &menu.subtitle {
        lines.extend(subtitle.lines().map(str::to_string));
        lines.push(String::new());
    }
    if let Some(prompt_lines) = prompt {
        lines.extend(prompt_lines.iter().cloned());
    }

    let paragraph = Paragraph::new(lines.join("\n"))
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, inner);
}

fn draw_dashboard(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    dashboard: &Dashboard,
    scene: &[String],
    log: &[String],
    prompt: Option<&[String]>,
    notice: Option<&str>,
) {
    let compact = is_compact_area(area);
    let bottom_lines = prompt
        .map(|lines| lines.len())
        .or_else(|| notice.map(|text| text.lines().count()))
        .unwrap_or(0);
    let bottom_height = if bottom_lines == 0 {
        3
    } else {
        bottom_panel_height(area, compact, bottom_lines)
    };
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(bottom_height)])
        .spacing(Spacing::Overlap(1))
        .split(area);

    if compact {
        let body = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(34),
                Constraint::Percentage(38),
                Constraint::Percentage(28),
            ])
            .spacing(Spacing::Overlap(1))
            .split(root[0]);
        render_status_panel(frame, body[0], dashboard, compact);
        render_panel(
            frame,
            body[1],
            "Location",
            location_lines(dashboard, scene),
            compact,
        );
        render_log(frame, body[2], log, compact);
    } else {
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(34), Constraint::Percentage(66)])
            .spacing(Spacing::Overlap(1))
            .split(root[0]);
        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(9), Constraint::Min(4)])
            .spacing(Spacing::Overlap(1))
            .split(body[0]);
        render_status_panel(frame, left[0], dashboard, compact);
        render_panel(
            frame,
            left[1],
            "Controls",
            vec![dashboard
                .action_hint
                .clone()
                .unwrap_or_else(|| "Use arrows, Enter, and Esc.".to_string())],
            compact,
        );
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(16), Constraint::Min(4)])
            .spacing(Spacing::Overlap(1))
            .split(body[1]);
        render_panel(
            frame,
            right[0],
            "Location",
            location_lines(dashboard, scene),
            compact,
        );
        render_log(frame, right[1], log, compact);
    }

    if let Some(prompt_lines) = prompt {
        render_prompt_panel(frame, root[1], prompt_lines, compact);
    } else {
        render_footer(frame, root[1], dashboard, compact, notice);
    }
}

fn render_status_panel(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    dashboard: &Dashboard,
    compact: bool,
) {
    let head_title = format!("The Ashen Chronicle v{}", env!("CARGO_PKG_VERSION"));
    let head_title: &str = head_title.as_str();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(head_title)
        .style(border_style(compact))
        .merge_borders(MergeStrategy::Exact);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = Vec::new();
    if !dashboard.world_name.is_empty() {
        lines.push(format!("World: {}", dashboard.world_name));
    }
    if !dashboard.time_display.is_empty() {
        lines.push(dashboard.time_display.clone());
    }
    if let Some(line) = &dashboard.condition_line {
        lines.push(line.clone());
    }
    if let Some(line) = &dashboard.danger_line {
        lines.push(line.clone());
    }

    let gauge_rows = 1 + usize::from(dashboard.enemy_name.is_some());
    let text_height = inner.height.saturating_sub(gauge_rows as u16);
    let text_area = Rect {
        height: text_height,
        ..inner
    };
    if text_height > 0 {
        let paragraph = Paragraph::new(lines.join("\n")).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, text_area);
    }

    let mut gauge_area = Rect {
        x: inner.x,
        y: inner.y + text_height,
        width: inner.width,
        height: 1,
    };
    render_health_gauge(
        frame,
        gauge_area,
        "HP",
        dashboard.hp,
        dashboard.max_hp,
        Color::Red,
        Color::DarkGray,
    );

    if let (Some(enemy_name), Some(enemy_hp), Some(enemy_max_hp)) = (
        dashboard.enemy_name.as_deref(),
        dashboard.enemy_hp,
        dashboard.enemy_max_hp,
    ) {
        gauge_area.y += 1;
        let title = format!("{} HP", enemy_name);
        render_health_gauge(
            frame,
            gauge_area,
            &title,
            enemy_hp,
            enemy_max_hp,
            Color::Red,
            Color::DarkGray,
        );
    }
}

fn render_health_gauge(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    label: &str,
    current: i32,
    maximum: i32,
    fill: Color,
    empty: Color,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let maximum = maximum.max(1);
    let current = current.clamp(0, maximum);
    let ratio = current as f64 / maximum as f64;
    let gauge = LineGauge::default()
        .ratio(ratio)
        .label(format!("{}: ", label))
        .filled_symbol("█")
        .unfilled_symbol("░")
        .filled_style(Style::default().fg(fill))
        .unfilled_style(Style::default().fg(empty));
    frame.render_widget(gauge, area);
}

fn render_panel(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    title: &str,
    lines: Vec<String>,
    compact: bool,
) {
    let content = if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    };
    let paragraph = Paragraph::new(content.join("\n"))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .style(border_style(compact))
                .merge_borders(MergeStrategy::Exact),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn render_log(frame: &mut ratatui::Frame<'_>, area: Rect, log: &[String], compact: bool) {
    let visible_lines = area.height.saturating_sub(2) as usize;
    let content = if log.is_empty() {
        vec!["...".to_string()]
    } else {
        tail_lines(log, visible_lines.max(1))
    };
    let paragraph = Paragraph::new(content.join("\n"))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Journal")
                .style(border_style(compact))
                .merge_borders(MergeStrategy::Exact),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn render_footer(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    dashboard: &Dashboard,
    compact: bool,
    notice: Option<&str>,
) {
    let paragraph = if let Some(notice) = notice {
        Paragraph::new(notice.to_string())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Actions")
                    .style(border_style(compact))
                    .merge_borders(MergeStrategy::Exact),
            )
            .wrap(Wrap { trim: true })
    } else {
        let hint = dashboard
            .action_hint
            .clone()
            .unwrap_or_else(|| "Use arrows, Enter, and Esc.".to_string());
        Paragraph::new(hint)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Controls")
                    .style(border_style(compact))
                    .merge_borders(MergeStrategy::Exact),
            )
            .wrap(Wrap { trim: true })
    };
    frame.render_widget(paragraph, area);
}

fn render_prompt_panel(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    prompt_lines: &[String],
    compact: bool,
) {
    let paragraph = Paragraph::new(prompt_lines.join("\n"))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Actions")
                .style(border_style(compact))
                .merge_borders(MergeStrategy::Exact),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn location_lines(dashboard: &Dashboard, scene: &[String]) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(line) = &dashboard.location_name {
        lines.push(line.clone());
    }
    if !scene.is_empty() {
        lines.extend(scene.iter().cloned());
    }
    if let Some(line) = &dashboard.location_description {
        lines.push(line.clone());
    }
    if let Some(line) = &dashboard.threat_line {
        lines.push(line.clone());
    }
    lines
}

fn tail_lines(lines: &[String], max_lines: usize) -> Vec<String> {
    if lines.len() <= max_lines {
        return lines.to_vec();
    }
    lines[lines.len() - max_lines..].to_vec()
}

fn border_style(compact: bool) -> Style {
    if compact {
        Style::default().fg(Color::Gray)
    } else {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    }
}
