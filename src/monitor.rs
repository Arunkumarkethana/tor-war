use crate::config::NipeConfig;
use crate::engine::NipeEngine;
use crate::status::ConnectionStatus;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::time::Duration;
use tokio::time::Instant;

pub struct Monitor {
    config: NipeConfig,
}

impl Monitor {
    pub fn new() -> Self {
        Self {
            config: NipeConfig::load().unwrap_or_default(),
        }
    }

    pub async fn run(&self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = self.run_app(&mut terminal).await;

        // Restore terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        if let Err(err) = res {
            println!("{:?}", err)
        }

        Ok(())
    }

    async fn run_app<B: ratatui::backend::Backend>(
        &self,
        terminal: &mut Terminal<B>,
    ) -> Result<()> {
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(250);

        let mut status_msg = "Checking...".to_string();
        let mut ip_info = "Unknown".to_string();
        let mut is_secure = false;

        // Initial check
        if let Ok(status) = ConnectionStatus::check().await {
            is_secure = status.is_tor;
            ip_info = status.current_ip;
            status_msg = if is_secure {
                "SECURE".to_string()
            } else {
                "UNSECURE".to_string()
            };
        }

        loop {
            terminal.draw(|f| {
                let size = f.size();
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints(
                        [
                            Constraint::Length(3), // Title
                            Constraint::Min(5),    // Main Content
                            Constraint::Length(3), // Footer
                        ]
                        .as_ref(),
                    )
                    .split(size);

                // Title
                let title = Paragraph::new("Nipe Security Monitor")
                    .style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .block(Block::default().borders(Borders::ALL));
                f.render_widget(title, chunks[0]);

                // Main Status
                let status_color = if is_secure { Color::Green } else { Color::Red };
                let status_text = vec![
                    Line::from(vec![
                        Span::raw("Status: "),
                        Span::styled(
                            status_msg.clone(),
                            Style::default()
                                .fg(status_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::raw("Current IP: "),
                        Span::styled(ip_info.clone(), Style::default().fg(Color::Yellow)),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::raw("Exit Country: "),
                        Span::styled(
                            self.config
                                .tor
                                .country
                                .clone()
                                .unwrap_or("Random".to_string()),
                            Style::default().fg(Color::Blue),
                        ),
                    ]),
                ];

                let main_block = Paragraph::new(status_text)
                    .block(
                        Block::default()
                            .title("Connection Info")
                            .borders(Borders::ALL),
                    )
                    .style(Style::default().fg(Color::White));
                f.render_widget(main_block, chunks[1]);

                // Footer
                let footer = Paragraph::new("Press 'q' to Quit | 'r' to Rotate Identity")
                    .style(Style::default().fg(Color::Gray))
                    .block(Block::default().borders(Borders::ALL));
                f.render_widget(footer, chunks[2]);
            })?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('r') => {
                            status_msg = "Rotating...".to_string();
                            // Non-blocking rotation attempt (spawn a task or just do it blocking for now)
                            // Ideally we shouldn't block the UI thread too long
                            if let Ok(engine) = NipeEngine::new(self.config.clone()) {
                                let _ = engine.rotate().await;
                                // Re-check status
                                if let Ok(status) = ConnectionStatus::check().await {
                                    is_secure = status.is_tor;
                                    ip_info = status.current_ip;
                                    status_msg = if is_secure {
                                        "SECURE".to_string()
                                    } else {
                                        "UNSECURE".to_string()
                                    };
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                // Periodic refresh could go here if needed, but we rely on manual refresh/events for now or slow poll
                last_tick = Instant::now();
            }
        }
    }
}
