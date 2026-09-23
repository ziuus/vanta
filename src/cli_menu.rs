use std::fs;
use std::io::{self, stdout, Read};
use std::path::PathBuf;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Terminal,
};
use sha2::{Digest, Sha256};

const REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/ziuus/vanta-integrations/main/registry.json";

#[derive(serde::Deserialize, Debug, Clone)]
pub struct RegistryExtension {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub api_version: String,
    pub author: String,
    pub wasm_url: String,
    pub sha256: String,
}

#[derive(serde::Deserialize, Debug)]
struct Registry {
    api_version: String,
    extensions: Vec<RegistryExtension>,
}

fn get_category(id: &str) -> &'static str {
    if id.starts_with("cryptopulse") || id.starts_with("crypto") {
        "Crypto"
    } else if id.starts_with("filespace") {
        "FileSpace"
    } else if id.starts_with("mediadeck") {
        "MediaDeck"
    } else if id == "security" {
        "Security"
    } else {
        "Observability"
    }
}

fn get_extensions_dir() -> PathBuf {
    let mut dir = directories::ProjectDirs::from("", "", "vanta")
        .map(|p| p.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    dir.push("extensions");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn download_and_install_ext(ext: &RegistryExtension) -> Result<PathBuf, String> {
    if !ext.api_version.starts_with("0.9") && !ext.api_version.starts_with("0.10") {
        return Err(format!(
            "'{}' requires Vanta UI API {}, but your Vanta supports API 0.10",
            ext.id, ext.api_version
        ));
    }

    let res = ureq::get(&ext.wasm_url)
        .call()
        .map_err(|e| format!("Network download failed: {}", e))?;

    let mut buf = Vec::new();
    res.into_body()
        .into_reader()
        .read_to_end(&mut buf)
        .map_err(|e| format!("Read artifact failed: {}", e))?;

    if !ext.sha256.is_empty() {
        let mut hasher = Sha256::new();
        hasher.update(&buf);
        let hash: String = hasher
            .finalize()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();

        if hash != ext.sha256 {
            return Err(format!(
                "Checksum mismatch (expected {}, got {})",
                &ext.sha256[..8],
                &hash[..8]
            ));
        }
    }

    let ext_dir = get_extensions_dir();
    let wasm_path = ext_dir.join(format!("{}.wasm", ext.id));
    let tmp_path = ext_dir.join(format!("{}.wasm.tmp", ext.id));

    fs::write(&tmp_path, buf).map_err(|e| format!("Disk write failed: {}", e))?;
    fs::rename(&tmp_path, &wasm_path).map_err(|e| format!("Atomic replace failed: {}", e))?;

    Ok(wasm_path)
}

pub fn run_menu_installer() -> io::Result<()> {
    println!("Fetching Vanta extensions registry...");
    let res = match ureq::get(REGISTRY_URL).call() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to reach extensions registry: {}", e);
            return Ok(());
        }
    };

    let registry: Registry = match serde_json::from_reader(res.into_body().into_reader()) {
        Ok(reg) => reg,
        Err(e) => {
            eprintln!("Failed to parse registry: {}", e);
            return Ok(());
        }
    };

    enable_raw_mode()?;
    let mut stdout_handle = stdout();
    execute!(stdout_handle, EnterAlternateScreen, crossterm::cursor::Hide)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout_handle))?;

    let categories = [
        "All",
        "Observability",
        "Crypto",
        "FileSpace",
        "MediaDeck",
        "Security",
    ];

    let mut active_cat_idx = 0;
    let mut list_state = ListState::default();
    list_state.select(Some(0));

    let mut status_msg = String::from("Use ↑/↓ to navigate, Enter to install, u to remove, Tab for category, q to quit.");
    let mut status_color = Color::DarkGray;

    loop {
        let current_category = categories[active_cat_idx];
        let filtered_extensions: Vec<&RegistryExtension> = registry
            .extensions
            .iter()
            .filter(|e| {
                if current_category == "All" {
                    true
                } else {
                    get_category(&e.id) == current_category
                }
            })
            .collect();

        if let Some(selected) = list_state.selected() {
            if filtered_extensions.is_empty() {
                list_state.select(None);
            } else if selected >= filtered_extensions.len() {
                list_state.select(Some(filtered_extensions.len() - 1));
            }
        } else if !filtered_extensions.is_empty() {
            list_state.select(Some(0));
        }

        let ext_dir = get_extensions_dir();

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header & Category Tabs
                    Constraint::Min(10),   // Main list & details
                    Constraint::Length(3), // Status & hotkeys footer
                ])
                .split(f.area());

            // 1. Header & Tabs
            let tab_titles: Vec<Line> = categories
                .iter()
                .map(|t| {
                    let count = if *t == "All" {
                        registry.extensions.len()
                    } else {
                        registry.extensions.iter().filter(|e| get_category(&e.id) == *t).count()
                    };
                    Line::from(format!(" {} ({}) ", t, count))
                })
                .collect();

            let tabs = Tabs::new(tab_titles)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Cyan))
                        .title(" 🧩 VANTA EXTENSION INSTALLER "),
                )
                .select(active_cat_idx)
                .style(Style::default().fg(Color::DarkGray))
                .highlight_style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                );
            f.render_widget(tabs, chunks[0]);

            // 2. Main Area (Split Left: List, Right: Details)
            let body_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
                .split(chunks[1]);

            // Left: Extension items list
            let items: Vec<ListItem> = filtered_extensions
                .iter()
                .map(|ext| {
                    let is_installed = ext_dir.join(format!("{}.wasm", ext.id)).exists();
                    let (badge, badge_color) = if is_installed {
                        ("✓", Color::Green)
                    } else {
                        (" ", Color::DarkGray)
                    };

                    let content = Line::from(vec![
                        Span::styled(format!("[{}] ", badge), Style::default().fg(badge_color).add_modifier(Modifier::BOLD)),
                        Span::styled(format!("{:<20} ", ext.id), Style::default().fg(Color::White)),
                        Span::styled(&ext.name, Style::default().fg(Color::DarkGray)),
                    ]);
                    ListItem::new(content)
                })
                .collect();

            let list_widget = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::White))
                        .title(format!(" Available Extensions ({}) ", filtered_extensions.len())),
                )
                .highlight_style(
                    Style::default()
                        .bg(Color::Rgb(40, 44, 52))
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");
            f.render_stateful_widget(list_widget, body_chunks[0], &mut list_state);

            // Right: Detail Card
            let selected_ext = list_state
                .selected()
                .and_then(|idx| filtered_extensions.get(idx).copied());

            let details_widget = if let Some(ext) = selected_ext {
                let is_installed = ext_dir.join(format!("{}.wasm", ext.id)).exists();
                let status_line = if is_installed {
                    Line::from(vec![
                        Span::styled("Status: ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                        Span::styled("● INSTALLED", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                        Span::styled(format!(" (~/.config/vanta/extensions/{}.wasm)", ext.id), Style::default().fg(Color::DarkGray)),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled("Status: ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                        Span::styled("○ NOT INSTALLED", Style::default().fg(Color::Yellow)),
                    ])
                };

                let action_line = if is_installed {
                    Line::from(vec![
                        Span::styled("Action: ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                        Span::styled("Press [Enter] to re-install, [u] to uninstall", Style::default().fg(Color::Cyan)),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled("Action: ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                        Span::styled("Press [Enter] or [i] to install this extension", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    ])
                };

                let lines = vec![
                    Line::from(vec![
                        Span::styled(format!("{} ", ext.name), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        Span::styled(format!("(v{})", ext.version), Style::default().fg(Color::DarkGray)),
                    ]),
                    Line::from(vec![
                        Span::styled("ID:       ", Style::default().fg(Color::DarkGray)),
                        Span::styled(&ext.id, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(vec![
                        Span::styled("Category: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(get_category(&ext.id), Style::default().fg(Color::Yellow)),
                    ]),
                    Line::from(vec![
                        Span::styled("Author:   ", Style::default().fg(Color::DarkGray)),
                        Span::styled(&ext.author, Style::default().fg(Color::White)),
                    ]),
                    Line::from(vec![
                        Span::styled("API Spec: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(format!("v{}", ext.api_version), Style::default().fg(Color::White)),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Description:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(Span::styled(&ext.description, Style::default().fg(Color::LightCyan))),
                    Line::from(""),
                    status_line,
                    action_line,
                ];

                Paragraph::new(lines)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(Color::White))
                            .title(" Extension Details "),
                    )
                    .wrap(Wrap { trim: true })
            } else {
                Paragraph::new("No extension selected.")
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .title(" Extension Details "),
                    )
            };
            f.render_widget(details_widget, body_chunks[1]);

            // 3. Footer
            let footer_lines = vec![
                Line::from(vec![
                    Span::styled(" [q] Quit ", Style::default().fg(Color::Black).bg(Color::White)),
                    Span::raw(" "),
                    Span::styled(" [↑/↓/j/k] Select ", Style::default().fg(Color::White).bg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::styled(" [Tab] Category ", Style::default().fg(Color::White).bg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::styled(" [Enter/i] Install ", Style::default().fg(Color::Black).bg(Color::Green)),
                    Span::raw(" "),
                    Span::styled(" [u] Remove ", Style::default().fg(Color::Black).bg(Color::Red)),
                ]),
                Line::from(Span::styled(&status_msg, Style::default().fg(status_color))),
            ];

            let footer = Paragraph::new(footer_lines).alignment(Alignment::Left);
            f.render_widget(footer, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(150))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Up | KeyCode::Char('k') => {
                            let current = list_state.selected().unwrap_or(0);
                            if current > 0 {
                                list_state.select(Some(current - 1));
                            } else if !filtered_extensions.is_empty() {
                                list_state.select(Some(filtered_extensions.len() - 1));
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let current = list_state.selected().unwrap_or(0);
                            if !filtered_extensions.is_empty() {
                                if current + 1 < filtered_extensions.len() {
                                    list_state.select(Some(current + 1));
                                } else {
                                    list_state.select(Some(0));
                                }
                            }
                        }
                        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                            active_cat_idx = (active_cat_idx + 1) % categories.len();
                            list_state.select(Some(0));
                            status_msg = format!("Filtered by category: {}", categories[active_cat_idx]);
                            status_color = Color::Cyan;
                        }
                        KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                            if active_cat_idx == 0 {
                                active_cat_idx = categories.len() - 1;
                            } else {
                                active_cat_idx -= 1;
                            }
                            list_state.select(Some(0));
                            status_msg = format!("Filtered by category: {}", categories[active_cat_idx]);
                            status_color = Color::Cyan;
                        }
                        KeyCode::Enter | KeyCode::Char('i') | KeyCode::Char(' ') => {
                            if let Some(selected_idx) = list_state.selected() {
                                if let Some(ext) = filtered_extensions.get(selected_idx) {
                                    status_msg = format!("Downloading & installing '{}'...", ext.id);
                                    status_color = Color::Yellow;

                                    match download_and_install_ext(ext) {
                                        Ok(path) => {
                                            status_msg = format!("✓ Installed '{}' to {}", ext.id, path.display());
                                            status_color = Color::Green;
                                        }
                                        Err(e) => {
                                            status_msg = format!("✗ Failed to install '{}': {}", ext.id, e);
                                            status_color = Color::Red;
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('u') | KeyCode::Delete | KeyCode::Backspace => {
                            if let Some(selected_idx) = list_state.selected() {
                                if let Some(ext) = filtered_extensions.get(selected_idx) {
                                    let ext_file = ext_dir.join(format!("{}.wasm", ext.id));
                                    if ext_file.exists() {
                                        match fs::remove_file(&ext_file) {
                                            Ok(_) => {
                                                status_msg = format!("✓ Uninstalled '{}' ({})", ext.id, ext_file.display());
                                                status_color = Color::Yellow;
                                            }
                                            Err(e) => {
                                                status_msg = format!("✗ Failed to remove '{}': {}", ext.id, e);
                                                status_color = Color::Red;
                                            }
                                        }
                                    } else {
                                        status_msg = format!("'{}' is not installed.", ext.id);
                                        status_color = Color::DarkGray;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, crossterm::cursor::Show)?;
    let _ = terminal.show_cursor();

    Ok(())
}
