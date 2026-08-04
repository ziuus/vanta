mod app;
mod config;
mod layout;
mod mode;
mod monitors;
mod screens;
mod widgets;

use std::io::{self, IsTerminal};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::app::App;
use crate::config::Config;

pub fn run() -> io::Result<()> {
    // Guard: crossterm raw mode needs a real TTY
    if !std::io::stdin().is_terminal() {
        eprintln!("error: vanta requires a terminal. Run it from your terminal emulator, not from a non-TTY context.");
        std::process::exit(1);
    }

    let config = Config::load();
    widgets::profile::set_image_path(config.ui.image_path.clone());

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(config);
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    let mut tick_rate = std::time::Duration::from_secs_f64(app.config.ui.refresh_rate);
    let render_rate = std::time::Duration::from_millis(8); // ~120 FPS for buttery smooth visualizer
    let mut last_tick = std::time::Instant::now();
    let mut last_render = std::time::Instant::now();

    while app.running {
        let now = std::time::Instant::now();
        if now.duration_since(last_render) >= render_rate {
            terminal.draw(|f| app.render(f))?;
            last_render = std::time::Instant::now();
        }

        let time_to_tick = tick_rate.saturating_sub(last_tick.elapsed());
        let time_to_render = render_rate.saturating_sub(last_render.elapsed());
        let timeout = time_to_tick.min(time_to_render);

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        // Global keys
                        KeyCode::Char('q') | KeyCode::Char('Q') => app.running = false,
                        KeyCode::Char('T') => app.toggle_theme(),
                        KeyCode::Char('v') | KeyCode::Char('V') => {
                            widgets::music_viz::cycle_style();
                        }
                        // Mode switches (must be BEFORE panel-focused handler to avoid capture)
                        KeyCode::Char('1') => {
                            app.set_mode(mode::DashboardMode::Overview);
                        }
                        KeyCode::Char('2') => {
                            app.set_mode(mode::DashboardMode::Monitor);
                        }
                        KeyCode::Char('3') => {
                            app.set_mode(mode::DashboardMode::Processes);
                        }
                        KeyCode::Char('4') => {
                            app.set_mode(mode::DashboardMode::Media);
                        }
                        KeyCode::Char('5') => {
                            app.set_mode(mode::DashboardMode::Aesthetic);
                        }
                        KeyCode::Char('6') => {
                            app.set_mode(mode::DashboardMode::Settings);
                        }
                        KeyCode::Esc => {
                            if app.panel_states.process_search_active {
                                app.panel_states.process_search_active = false;
                                app.panel_states.process_search.clear();
                            } else {
                                app.focused_panel = None;
                            }
                        }
                        KeyCode::Tab => app.cycle_focus(true),
                        KeyCode::BackTab => app.cycle_focus(false),

                        // When a panel is focused, dispatch key to panel handlers
                        _ if app.focused_panel.is_some() => {
                            app.handle_panel_nav(key.code);
                        }

                        // No focus — arrow keys auto-focus first panel
                        KeyCode::Up
                        | KeyCode::Down
                        | KeyCode::Left
                        | KeyCode::Right
                        | KeyCode::Home
                        | KeyCode::PageUp
                        | KeyCode::PageDown => {
                            app.focused_panel = app::PanelId::all(&app.config).first().copied();
                            app.handle_panel_nav(key.code);
                        }

                        // Enter on any focused panel
                        KeyCode::Enter if app.focused_panel == Some(app::PanelId::Calendar) => {
                            app.panel_states.calendar_month_offset = 0;
                        }

                        _ => {}
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            tick_rate = std::time::Duration::from_secs_f64(app.config.ui.refresh_rate);
            app.tick();
            last_tick = std::time::Instant::now();
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    run()
}
