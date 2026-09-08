mod app;
mod config;
mod mode;
mod monitors;
mod screens;
mod theme;
mod widgets;

use std::io::{self, IsTerminal, Write};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::App;
use crate::config::Config;

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, crossterm::cursor::Show);
    let _ = io::stdout().flush();
}

fn print_help() {
    println!(
        "vanta {} — aesthetic terminal system dashboard\n\n\
         usage: vanta [--version] [--help]\n\n\
         config: {}\n\
         keys:   1/2/3 pages · ? help · T theme · v visualizer · q quit",
        env!("CARGO_PKG_VERSION"),
        config::config_path()
    );
}

fn main() -> io::Result<()> {
    let mut args = std::env::args().skip(1);
    if let Some(a) = args.next() {
        match a.as_str() {
            "-V" | "--version" => println!("vanta {}", env!("CARGO_PKG_VERSION")),
            _ => print_help(),
        }
        return Ok(());
    }

    if !io::stdout().is_terminal() {
        eprintln!("error: vanta needs a terminal (stdout is not a tty)");
        std::process::exit(1);
    }

    let config = Config::load();

    // Restore the terminal on panic so a bug never leaves the shell in raw mode.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    terminal.clear()?;

    let mut app = App::new(config);
    let res = run(&mut terminal, &mut app);

    widgets::music_viz::shutdown();
    restore_terminal();
    res
}

fn run<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    let mut frame_time = Duration::from_secs_f64(1.0 / app.config.ui.fps as f64);
    let mut last_frame = Instant::now();

    terminal.draw(|f| app.render(f))?;
    while app.running {
        // Coalesce all pending input, then draw once.
        let timeout = frame_time.saturating_sub(last_frame.elapsed());
        if event::poll(timeout)? {
            loop {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => app.handle_key(key),
                    Event::Resize(_, _) => {}
                    _ => {}
                }
                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }
        if last_frame.elapsed() >= frame_time || !app.running {
            terminal.draw(|f| app.render(f))?;
            last_frame = Instant::now();
            frame_time = Duration::from_secs_f64(1.0 / app.config.ui.fps as f64);
        }
    }
    Ok(())
}
