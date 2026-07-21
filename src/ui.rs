use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

use crate::config::{Config, ConfigManager};
use crate::core::{Snapshot, SnapshotManager};

enum Focus {
    Configs,
    Snapshots,
}

enum InputMode {
    Normal,
    CreateSnapshot,
}

pub struct TuiApp {
    config_manager: ConfigManager,
    configs: Vec<Config>,
    config_state: ListState,
    snapshots: Vec<Snapshot>,
    snapshot_state: ListState,
    focus: Focus,
    input_mode: InputMode,
    input: Input,
    message: Option<String>,
    should_quit: bool,
}

impl TuiApp {
    pub fn new() -> Result<Self> {
        let config_manager = ConfigManager::new()?;
        let configs = config_manager.list_configs()?;

        let mut config_state = ListState::default();
        if !configs.is_empty() {
            config_state.select(Some(0));
        }

        Ok(Self {
            config_manager,
            configs,
            config_state,
            snapshots: Vec::new(),
            snapshot_state: ListState::default(),
            focus: Focus::Configs,
            input_mode: InputMode::Normal,
            input: Input::default(),
            message: None,
            should_quit: false,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Load snapshots for first config
        if let Some(selected) = self.config_state.selected() {
            if selected < self.configs.len() {
                self.load_snapshots_for_config(selected)?;
            }
        }

        // Main loop
        let result = self.run_loop(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    fn run_loop<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match self.input_mode {
                        InputMode::Normal => self.handle_normal_input(key.code)?,
                        InputMode::CreateSnapshot => self.handle_create_input(key)?,
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    fn handle_normal_input(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('c') => {
                self.input_mode = InputMode::CreateSnapshot;
                self.input = Input::default();
            }
            KeyCode::Char('d') => self.delete_selected_snapshot()?,
            KeyCode::Char('r') => self.rollback_selected_snapshot()?,
            KeyCode::Up => self.navigate_up(),
            KeyCode::Down => self.navigate_down(),
            KeyCode::Tab => self.switch_focus(),
            KeyCode::Enter => {
                if matches!(self.focus, Focus::Configs) {
                    if let Some(selected) = self.config_state.selected() {
                        self.load_snapshots_for_config(selected)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_create_input(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Enter => {
                let description = self.input.value().to_string();
                self.create_snapshot(&description)?;
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            _ => {
                self.input.handle_event(&Event::Key(key));
            }
        }
        Ok(())
    }

    fn navigate_up(&mut self) {
        match self.focus {
            Focus::Configs => {
                let i = match self.config_state.selected() {
                    Some(i) => {
                        if i > 0 {
                            i - 1
                        } else {
                            self.configs.len().saturating_sub(1)
                        }
                    }
                    None => 0,
                };
                self.config_state.select(Some(i));
            }
            Focus::Snapshots => {
                let i = match self.snapshot_state.selected() {
                    Some(i) => {
                        if i > 0 {
                            i - 1
                        } else {
                            self.snapshots.len().saturating_sub(1)
                        }
                    }
                    None => 0,
                };
                self.snapshot_state.select(Some(i));
            }
        }
    }

    fn navigate_down(&mut self) {
        match self.focus {
            Focus::Configs => {
                let i = match self.config_state.selected() {
                    Some(i) => {
                        if i >= self.configs.len().saturating_sub(1) {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.config_state.select(Some(i));
            }
            Focus::Snapshots => {
                let i = match self.snapshot_state.selected() {
                    Some(i) => {
                        if i >= self.snapshots.len().saturating_sub(1) {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.snapshot_state.select(Some(i));
            }
        }
    }

    fn switch_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Configs => {
                if !self.snapshots.is_empty() && self.snapshot_state.selected().is_none() {
                    self.snapshot_state.select(Some(0));
                }
                Focus::Snapshots
            }
            Focus::Snapshots => Focus::Configs,
        };
    }

    fn load_snapshots_for_config(&mut self, index: usize) -> Result<()> {
        if index < self.configs.len() {
            let config = self.configs[index].clone();
            let mut mgr = SnapshotManager::new(config)?;
            self.snapshots = mgr.list_snapshots()?;
            self.snapshot_state = ListState::default();
            if !self.snapshots.is_empty() {
                self.snapshot_state.select(Some(0));
            }
        }
        Ok(())
    }

    fn create_snapshot(&mut self, description: &str) -> Result<()> {
        if let Some(selected) = self.config_state.selected() {
            if selected < self.configs.len() {
                let config = self.configs[selected].clone();
                let mut mgr = SnapshotManager::new(config)?;
                mgr.create_snapshot(description)?;
                self.load_snapshots_for_config(selected)?;
                self.message = Some(format!("✓ Snapshot created: {}", description));
            }
        }
        Ok(())
    }

    fn delete_selected_snapshot(&mut self) -> Result<()> {
        if let Some(snap_idx) = self.snapshot_state.selected() {
            if snap_idx < self.snapshots.len() {
                let snapshot_id = self.snapshots[snap_idx].id;
                if let Some(config_idx) = self.config_state.selected() {
                    if config_idx < self.configs.len() {
                        let config = self.configs[config_idx].clone();
                        let mut mgr = SnapshotManager::new(config)?;
                        mgr.delete_snapshot(snapshot_id)?;
                        self.load_snapshots_for_config(config_idx)?;
                        self.message = Some(format!("✓ Snapshot #{} deleted", snapshot_id));
                    }
                }
            }
        }
        Ok(())
    }

    fn rollback_selected_snapshot(&mut self) -> Result<()> {
        if let Some(snap_idx) = self.snapshot_state.selected() {
            if snap_idx < self.snapshots.len() {
                let snapshot_id = self.snapshots[snap_idx].id;
                if let Some(config_idx) = self.config_state.selected() {
                    if config_idx < self.configs.len() {
                        let config = self.configs[config_idx].clone();
                        let mut mgr = SnapshotManager::new(config)?;
                        mgr.rollback_snapshot(snapshot_id)?;
                        self.message = Some(format!(
                            "✓ Rolled back to snapshot #{}. Please reboot.",
                            snapshot_id
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Header
        self.render_header(f, chunks[0]);

        // Main content
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(chunks[1]);

        self.render_configs(f, main_chunks[0]);
        self.render_snapshots(f, main_chunks[1]);

        // Footer
        self.render_footer(f, chunks[2]);

        // Input popup
        if matches!(self.input_mode, InputMode::CreateSnapshot) {
            self.render_input_popup(f, f.area());
        }
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let header = Paragraph::new(" ❄️  ICY - Snapshot Manager")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(header, area);
    }

    fn render_configs(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .configs
            .iter()
            .map(|c| {
                let content = format!("📁 {}", c.name);
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Configs")
                    .borders(Borders::ALL)
                    .border_style(if matches!(self.focus, Focus::Configs) {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default()
                    }),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, area, &mut self.config_state);
    }

    fn render_snapshots(&mut self, f: &mut Frame, area: Rect) {
        let title = if let Some(selected) = self.config_state.selected() {
            if selected < self.configs.len() {
                format!("Snapshots ({})", self.configs[selected].name)
            } else {
                "Snapshots".to_string()
            }
        } else {
            "Snapshots".to_string()
        };

        let items: Vec<ListItem> = self
            .snapshots
            .iter()
            .map(|s| {
                let date = s.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
                let content = format!("#{:<3} {} - {}", s.id, date, s.description);
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(if matches!(self.focus, Focus::Snapshots) {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default()
                    }),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, area, &mut self.snapshot_state);
    }

    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let help_text = match self.input_mode {
            InputMode::Normal => {
                "c: Create  r: Rollback  d: Delete  q: Quit  ↑/↓: Navigate  Tab: Switch Panel"
            }
            InputMode::CreateSnapshot => "Enter: Confirm  Esc: Cancel",
        };

        let mut footer_text = vec![Line::from(help_text)];

        if let Some(ref msg) = self.message {
            footer_text.insert(0, Line::from(Span::styled(msg, Style::default().fg(Color::Green))));
        }

        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, area);
    }

    fn render_input_popup(&mut self, f: &mut Frame, area: Rect) {
        let popup_area = centered_rect(60, 20, area);

        let input_widget = Paragraph::new(self.input.value())
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Create Snapshot - Enter Description"),
            );

        f.render_widget(ratatui::widgets::Clear, popup_area);
        f.render_widget(input_widget, popup_area);

        // Show cursor
        f.set_cursor_position((
            popup_area.x + self.input.visual_cursor() as u16 + 1,
            popup_area.y + 1,
        ));
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
