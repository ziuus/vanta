mod cli;
mod theme;

use std::io::{self, IsTerminal, Write};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use vanta::app::App;
use vanta::config::Config;

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, crossterm::cursor::Show);
    let _ = io::stdout().flush();
}

fn main() -> io::Result<()> {
    vanta::logger::init();
    log::info!(target: "core", "Vanta started (v{})", env!("CARGO_PKG_VERSION"));
    use clap::Parser;
    let cli_args = cli::Cli::parse();
    if cli::handle_cli(cli_args) {
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
    app.ext_manager.register(
        Box::new(vanta::extension::template::TemplateExtension),
        app.config.extensions.as_ref(),
    );

    // Load WASM extensions
    if let Some(mut ext_dir) =
        directories::ProjectDirs::from("", "", "vanta").map(|p| p.config_dir().to_path_buf())
    {
        ext_dir.push("extensions");
        if let Ok(entries) = std::fs::read_dir(&ext_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                    if let Ok(ext) = vanta::extension::wasm::WasmExtension::new(path) {
                        app.ext_manager
                            .register(Box::new(ext), app.config.extensions.as_ref());
                    }
                }
            }
        }
    }

    let res = run(&mut terminal, &mut app);

    vanta::widgets::music_viz::shutdown();
    app.ext_manager.shutdown_all();
    restore_terminal();
    res
}

fn run<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    let mut frame_time = Duration::from_secs_f64(1.0 / app.config.ui.fps as f64);
    let mut last_frame = Instant::now();
    // VANTA_PROFILE=1: log frame-time percentiles every 60 frames.
    let profiling = std::env::var_os("VANTA_PROFILE").is_some();
    let mut frame_us: Vec<u128> = Vec::with_capacity(60);

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
            let t0 = Instant::now();
            terminal.draw(|f| app.render(f))?;
            if profiling {
                frame_us.push(t0.elapsed().as_micros());
                if frame_us.len() == 60 {
                    use std::io::Write;
                    frame_us.sort_unstable();
                    if let Ok(mut f) = std::fs::OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open("/tmp/vanta-profile.log")
                    {
                        let _ = writeln!(
                            f,
                            "frame p50={:.1}ms p95={:.1}ms max={:.1}ms",
                            frame_us[30] as f64 / 1000.0,
                            frame_us[57] as f64 / 1000.0,
                            frame_us[59] as f64 / 1000.0
                        );
                    }
                    frame_us.clear();
                }
            }
            last_frame = Instant::now();
            frame_time = Duration::from_secs_f64(1.0 / app.config.ui.fps as f64);
        }
    }
    Ok(())
}
