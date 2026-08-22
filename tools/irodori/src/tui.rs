use std::collections::BTreeMap;
use std::env;
use std::io::{self, IsTerminal, Write};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    self, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

use crate::catalog::{Engine, SeedMode};
use crate::manager::SampleManager;
use crate::runtime::{Status, embedded_status};

const REFRESH_INTERVAL: Duration = Duration::from_secs(5);
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone, Debug, Eq, PartialEq)]
enum ConfirmationAction {
    Reset,
    Delete,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Confirmation {
    prompt: String,
    action: ConfirmationAction,
}

struct TuiState {
    selected: usize,
    statuses: BTreeMap<String, Status>,
    message: String,
    busy: bool,
    confirmation: Option<Confirmation>,
    overlay: Option<String>,
    runtime_name: String,
    colors: bool,
}

pub fn run_tui(manager: &mut SampleManager) -> Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        bail!("the TUI needs an interactive terminal; use 'task list' for a non-interactive view");
    }

    let catalog = manager.catalog().to_vec();
    let mut state = TuiState {
        selected: 0,
        statuses: fallback_statuses(&catalog),
        message: "Loading container status...".to_owned(),
        busy: false,
        confirmation: None,
        overlay: None,
        runtime_name: "detecting".to_owned(),
        colors: env::var_os("NO_COLOR").is_none(),
    };
    let _terminal = TerminalGuard::enter()?;
    let mut stdout = io::stdout();
    draw(&mut stdout, &catalog, &state)?;
    refresh(manager, &catalog, &mut state);
    draw(&mut stdout, &catalog, &state)?;
    let mut refreshed_at = Instant::now();

    loop {
        if refreshed_at.elapsed() >= REFRESH_INTERVAL && !state.busy {
            refresh(manager, &catalog, &mut state);
            draw(&mut stdout, &catalog, &state)?;
            refreshed_at = Instant::now();
        }
        if !event::poll(Duration::from_millis(200))? {
            continue;
        }
        match event::read()? {
            Event::Resize(_, _) => draw(&mut stdout, &catalog, &state)?,
            Event::Key(key) if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
                if handle_key(manager, &catalog, &mut state, key, &mut stdout)? {
                    break;
                }
                refreshed_at = Instant::now();
            }
            _ => {}
        }
    }
    Ok(())
}

fn handle_key(
    manager: &mut SampleManager,
    catalog: &[Engine],
    state: &mut TuiState,
    key: KeyEvent,
    stdout: &mut impl Write,
) -> Result<bool> {
    if is_ctrl_c(key) || (key.code == KeyCode::Char('q') && state.confirmation.is_none()) {
        return Ok(true);
    }
    if state.overlay.is_some() {
        state.overlay = None;
        draw(stdout, catalog, state)?;
        return Ok(false);
    }
    if let Some(confirmation) = state.confirmation.clone() {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                state.confirmation = None;
                let id = catalog[state.selected].id.clone();
                match confirmation.action {
                    ConfirmationAction::Reset => perform(
                        manager,
                        catalog,
                        state,
                        stdout,
                        &confirmation.prompt,
                        |manager| manager.reset(&id, "default", DEFAULT_TIMEOUT),
                    )?,
                    ConfirmationAction::Delete => perform(
                        manager,
                        catalog,
                        state,
                        stdout,
                        &confirmation.prompt,
                        |manager| manager.down(&id, "default", true),
                    )?,
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('q') | KeyCode::Esc => {
                state.confirmation = None;
                state.message = "Cancelled".to_owned();
                draw(stdout, catalog, state)?;
            }
            _ => {}
        }
        return Ok(false);
    }
    if state.busy {
        return Ok(false);
    }

    let id = catalog[state.selected].id.clone();
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            state.selected = (state.selected + catalog.len() - 1) % catalog.len();
            draw(stdout, catalog, state)?;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.selected = (state.selected + 1) % catalog.len();
            draw(stdout, catalog, state)?;
        }
        KeyCode::Home => {
            state.selected = 0;
            draw(stdout, catalog, state)?;
        }
        KeyCode::End => {
            state.selected = catalog.len() - 1;
            draw(stdout, catalog, state)?;
        }
        KeyCode::Enter => perform(
            manager,
            catalog,
            state,
            stdout,
            &format!("Starting {id}"),
            |manager| manager.start(&id, "default", DEFAULT_TIMEOUT),
        )?,
        KeyCode::Char('s') => perform(
            manager,
            catalog,
            state,
            stdout,
            &format!("Stopping {id}"),
            |manager| manager.stop(&id, "default"),
        )?,
        KeyCode::Char('e') => perform(
            manager,
            catalog,
            state,
            stdout,
            &format!("Seeding {id}"),
            |manager| manager.seed(&id),
        )?,
        KeyCode::Char('r') => {
            state.confirmation = Some(Confirmation {
                prompt: format!("Reset {id}"),
                action: ConfirmationAction::Reset,
            });
            draw(stdout, catalog, state)?;
        }
        KeyCode::Char('d') => {
            state.confirmation = Some(Confirmation {
                prompt: format!("Delete {id} and its data"),
                action: ConfirmationAction::Delete,
            });
            draw(stdout, catalog, state)?;
        }
        KeyCode::Char('l') => {
            state.busy = true;
            state.message = format!("Loading {id} logs...");
            draw(stdout, catalog, state)?;
            match manager.logs(&id, "default", 100) {
                Ok(logs) => {
                    state.overlay = Some(if logs.is_empty() {
                        "No logs".to_owned()
                    } else {
                        logs
                    });
                    state.message = format!("{id} logs loaded");
                }
                Err(error) => state.message = format!("Error: {error}"),
            }
            state.busy = false;
            draw(stdout, catalog, state)?;
        }
        KeyCode::Char('R') => {
            refresh(manager, catalog, state);
            draw(stdout, catalog, state)?;
        }
        KeyCode::Char('?') => {
            state.overlay = Some(HELP.to_owned());
            draw(stdout, catalog, state)?;
        }
        _ => {}
    }
    Ok(false)
}

fn perform(
    manager: &mut SampleManager,
    catalog: &[Engine],
    state: &mut TuiState,
    stdout: &mut impl Write,
    label: &str,
    operation: impl FnOnce(&mut SampleManager) -> Result<String>,
) -> Result<()> {
    state.busy = true;
    state.overlay = None;
    state.message = format!("{label}...");
    draw(stdout, catalog, state)?;
    state.message = match operation(manager) {
        Ok(message) if !message.is_empty() => message,
        Ok(_) => format!("{label}: done"),
        Err(error) => format!("Error: {error}"),
    };
    state.busy = false;
    refresh(manager, catalog, state);
    draw(stdout, catalog, state)
}

fn refresh(manager: &mut SampleManager, catalog: &[Engine], state: &mut TuiState) {
    if state.busy {
        return;
    }
    match manager.runtime_name().and_then(|name| {
        state.runtime_name = name;
        manager.statuses()
    }) {
        Ok(statuses) => {
            state.statuses = statuses;
            if state.message == "Loading container status..." {
                state.message = "Ready".to_owned();
            }
        }
        Err(error) => {
            state.runtime_name = "unavailable".to_owned();
            state.message = error.to_string();
            state.statuses = fallback_statuses(catalog);
        }
    }
}

fn draw(stdout: &mut impl Write, catalog: &[Engine], state: &TuiState) -> Result<()> {
    let (width, height) = terminal::size().unwrap_or((100, 30));
    let screen = render_tui(catalog, state, usize::from(width), usize::from(height));
    stdout.write_all(screen.as_bytes())?;
    stdout.flush()?;
    Ok(())
}

fn render_tui(catalog: &[Engine], state: &TuiState, width: usize, height: usize) -> String {
    let paint = Painter::new(state.colors);
    if let Some(overlay) = &state.overlay {
        return render_overlay(overlay, width, height, &paint);
    }
    let selected_engine = &catalog[state.selected];
    let available_rows = height.saturating_sub(11).max(4);
    let start = viewport_start(state.selected, catalog.len(), available_rows);
    let visible = catalog.iter().skip(start).take(available_rows);
    let mut lines = vec![
        format!(
            "{}  {}",
            paint.bold("irodori-samples"),
            paint.dim(&format!("runtime: {}", state.runtime_name))
        ),
        paint.dim(
            "↑↓/jk move  Enter start-ready  s stop  e seed  r reset  d delete  l logs  R refresh  ? help  q quit",
        ),
        String::new(),
    ];
    for (offset, engine) in visible.enumerate() {
        let index = start + offset;
        let status = state
            .statuses
            .get(&engine.id)
            .cloned()
            .unwrap_or_else(|| Status {
                state: "unknown".to_owned(),
                detail: String::new(),
                ports: String::new(),
            });
        let marker = if index == state.selected {
            paint.cyan(">")
        } else {
            " ".to_owned()
        };
        let id = pad(&engine.id, 15);
        let family = pad(&engine.family, 16);
        let state_label = format_state(&status.state, &paint);
        let connection_width = width.saturating_sub(51);
        let connection = truncate(&engine.connection, connection_width);
        let row = format!(
            "{marker} {id} {} {state_label} {connection}",
            paint.dim(&family)
        )
        .trim_end()
        .to_owned();
        lines.push(if index == state.selected {
            paint.bold(&row)
        } else {
            row
        });
    }
    lines.push(String::new());
    lines.push(format!(
        "{}  seed: {}",
        paint.bold(&selected_engine.id),
        if selected_engine.seed == SeedMode::Manual {
            "managed".to_owned()
        } else {
            selected_engine.seed.to_string()
        }
    ));
    lines.push(paint.dim(&truncate(
        if selected_engine.connection.is_empty() {
            "No connection string documented"
        } else {
            &selected_engine.connection
        },
        width,
    )));
    lines.push(String::new());
    lines.push(if state.busy {
        paint.yellow(&format!("◐ {}", state.message))
    } else {
        truncate(&state.message, width)
    });
    if let Some(confirmation) = &state.confirmation {
        lines.push(paint.red(&format!("Confirm: {}? [y/N]", confirmation.prompt)));
    }
    format!(
        "\u{1b}[2J\u{1b}[H{}",
        lines
            .into_iter()
            .take(height)
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn render_overlay(content: &str, width: usize, height: usize, paint: &Painter) -> String {
    let body = content.lines().collect::<Vec<_>>();
    let start = body.len().saturating_sub(height.saturating_sub(4).max(1));
    let visible = body[start..]
        .iter()
        .map(|line| truncate(line, width))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "\u{1b}[2J\u{1b}[H{}  {}\n\n{}",
        paint.bold("irodori-samples"),
        paint.dim("press any key to return"),
        visible
    )
}

fn fallback_statuses(catalog: &[Engine]) -> BTreeMap<String, Status> {
    catalog
        .iter()
        .map(|engine| {
            let status = if engine.embedded {
                embedded_status(engine)
            } else {
                Status {
                    state: "absent".to_owned(),
                    detail: "not created".to_owned(),
                    ports: String::new(),
                }
            };
            (engine.id.clone(), status)
        })
        .collect()
}

fn is_ctrl_c(key: KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
}

fn viewport_start(selected: usize, total: usize, rows: usize) -> usize {
    if total <= rows {
        0
    } else {
        selected
            .saturating_sub(rows / 2)
            .min(total.saturating_sub(rows))
    }
}

fn format_state(state: &str, paint: &Painter) -> String {
    match state {
        "healthy" => paint.green("● healthy "),
        "ready" => paint.green("● ready   "),
        "running" => paint.yellow("● running "),
        "starting" => paint.yellow("◐ starting"),
        "unhealthy" => paint.red("× unhealthy"),
        "stopped" => paint.red("○ stopped "),
        "absent" => paint.dim("○ absent  "),
        _ => paint.dim("? unknown "),
    }
}

struct Painter {
    enabled: bool,
}

impl Painter {
    fn new(enabled: bool) -> Self {
        Self { enabled }
    }
    fn color(&self, code: u8, value: &str) -> String {
        if self.enabled {
            format!("\u{1b}[{code}m{value}\u{1b}[0m")
        } else {
            value.to_owned()
        }
    }
    fn bold(&self, value: &str) -> String {
        self.color(1, value)
    }
    fn dim(&self, value: &str) -> String {
        self.color(2, value)
    }
    fn red(&self, value: &str) -> String {
        self.color(31, value)
    }
    fn green(&self, value: &str) -> String {
        self.color(32, value)
    }
    fn yellow(&self, value: &str) -> String {
        self.color(33, value)
    }
    fn cyan(&self, value: &str) -> String {
        self.color(36, value)
    }
}

fn pad(value: &str, length: usize) -> String {
    let count = value.chars().count();
    if count >= length {
        value.to_owned()
    } else {
        format!("{value}{}", " ".repeat(length - count))
    }
}

fn truncate(value: &str, length: usize) -> String {
    let characters = value.chars().collect::<Vec<_>>();
    if characters.len() <= length {
        value.to_owned()
    } else if length == 0 {
        String::new()
    } else if length == 1 {
        "…".to_owned()
    } else {
        format!("{}…", characters[..length - 1].iter().collect::<String>())
    }
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode().context("could not enable terminal raw mode")?;
        if let Err(error) = execute!(io::stdout(), EnterAlternateScreen, Hide) {
            let _ = disable_raw_mode();
            return Err(error).context("could not enter the alternate screen");
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), Show, LeaveAlternateScreen);
    }
}

const HELP: &str = "Keyboard\n\n  Enter   start the selected engine, wait until ready, and seed it when supported\n  s       stop the engine without deleting its data\n  e       apply or recreate its sample seed\n  r       reset: delete its data, start it, and seed it\n  d       delete its containers and data\n  l       show the last 100 log lines\n  R       refresh status immediately\n  q       quit\n\nReset and delete always ask for confirmation.";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::SeedMode;

    fn engine(id: &str, family: &str, seed: SeedMode, connection: &str) -> Engine {
        Engine {
            id: id.into(),
            family: family.into(),
            seed,
            embedded: false,
            directory: id.into(),
            compose_path: Some(format!("{id}/compose.yaml").into()),
            project: Some(format!("irodori-{id}")),
            has_healthcheck: true,
            variants: BTreeMap::new(),
            connection: connection.into(),
            data_path: None,
        }
    }

    #[test]
    fn rendering_keeps_selection_and_connection_visible() {
        let catalog = vec![
            engine(
                "postgres",
                "Relational",
                SeedMode::Init,
                "postgres://localhost/samples",
            ),
            engine(
                "redis",
                "Key-value",
                SeedMode::Manual,
                "redis://localhost/0",
            ),
        ];
        let state = TuiState {
            selected: 1,
            statuses: BTreeMap::from([
                (
                    "postgres".into(),
                    Status {
                        state: "healthy".into(),
                        detail: String::new(),
                        ports: String::new(),
                    },
                ),
                (
                    "redis".into(),
                    Status {
                        state: "absent".into(),
                        detail: String::new(),
                        ports: String::new(),
                    },
                ),
            ]),
            message: "Ready".into(),
            busy: false,
            confirmation: None,
            overlay: None,
            runtime_name: "docker".into(),
            colors: false,
        };
        let screen = render_tui(&catalog, &state, 100, 24);
        assert!(screen.contains("> redis"));
        assert!(screen.contains("redis://localhost/0"));
        assert!(screen.contains("○ absent"));
    }

    #[test]
    fn viewport_tracks_the_selected_row() {
        assert_eq!(viewport_start(0, 25, 10), 0);
        assert_eq!(viewport_start(20, 25, 10), 15);
    }
}
