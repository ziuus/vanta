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

fn default_ext_type() -> String {
    "component".to_string()
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct RegistryExtension {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub api_version: String,
    pub author: String,
    #[serde(default)]
    pub wasm_url: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(default = "default_ext_type", alias = "type", alias = "kind")]
    pub ext_type: String,
    #[serde(default)]
    pub components: Vec<String>,
    #[serde(default)]
    pub layout: Vec<Vec<String>>,
}

#[derive(serde::Deserialize, Debug)]
#[allow(dead_code)]
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

fn get_themes_dir() -> PathBuf {
    let base = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("vanta")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config").join("vanta")
    } else {
        PathBuf::from(".")
    };
    let dir = base.join("themes");
    let _ = fs::create_dir_all(&dir);
    dir
}
fn get_extensions_dir() -> PathBuf {
    let mut dir = directories::ProjectDirs::from("", "", "vanta")
        .map(|p| p.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    dir.push("extensions");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn get_config_path() -> PathBuf {
    directories::ProjectDirs::from("", "", "vanta")
        .map(|p| p.config_dir().join("config.toml"))
        .unwrap_or_else(|| PathBuf::from("config.toml"))
}

pub fn add_page_to_config(
    name: &str,
    layout: &[Vec<String>],
    components: &[String],
) -> Result<(), String> {
    let path = get_config_path();
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let default_config = "[extensions]\nenabled = []\n";
            let _ = fs::write(&path, default_config);
            default_config.to_string()
        }
    };

    let mut new_content = content;

    // 1. Enable components in [extensions.enabled]
    for comp in components {
        if !new_content.contains(&format!("\"{}\"", comp))
            && !new_content.contains(&format!("'{}'", comp))
        {
            if let Some(idx) = new_content.find("enabled = [") {
                let insert_at = idx + "enabled = [".len();
                let (before, after) = new_content.split_at(insert_at);
                new_content = format!("{}\"{}\", {}", before, comp, after);
            } else {
                if !new_content.contains("[extensions]") {
                    new_content.push_str("\n[extensions]\n");
                }
                new_content.push_str(&format!("enabled = [\"{}\"]\n", comp));
            }
        }
    }

    // 2. Add [[pages]] if not already present
    if !new_content.contains(&format!("name = \"{}\"", name)) {
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        let layout_str = serde_json::to_string(layout).unwrap_or_else(|_| "[]".to_string());
        new_content.push_str(&format!(
            "\n[[pages]]\nname = \"{}\"\nlayout = {}\n",
            name, layout_str
        ));
    }

    fs::write(&path, new_content).map_err(|e| format!("Failed to write config.toml: {}", e))?;
    Ok(())
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

    let (_ext_dir, ext_path, tmp_path) = if ext.ext_type == "theme" || ext.ext_type == "scene" {
        let d = get_themes_dir();
        let p = d.join(format!("{}.toml", ext.id));
        let t = d.join(format!("{}.toml.tmp", ext.id));
        (d, p, t)
    } else {
        let d = get_extensions_dir();
        let p = d.join(format!("{}.wasm", ext.id));
        let t = d.join(format!("{}.wasm.tmp", ext.id));
        (d, p, t)
    };

    fs::write(&tmp_path, buf).map_err(|e| format!("Disk write failed: {}", e))?;
    fs::rename(&tmp_path, &ext_path).map_err(|e| format!("Atomic replace failed: {}", e))?;

    Ok(ext_path)
}

pub fn install_page_ext(
    ext: &RegistryExtension,
    all_extensions: &[RegistryExtension],
) -> Result<String, String> {
    let mut installed_count = 0;
    for comp_id in &ext.components {
        if let Some(comp_ext) = all_extensions.iter().find(|e| e.id == *comp_id) {
            download_and_install_ext(comp_ext)?;
            installed_count += 1;
        } else {
            return Err(format!("Component '{}' missing from registry", comp_id));
        }
    }

    add_page_to_config(&ext.name, &ext.layout, &ext.components)?;
    Ok(format!(
        "Installed page '{}' with {} components",
        ext.name, installed_count
    ))
}

pub fn run_menu_installer() -> io::Result<()> {
    vanta::config::clean_orphaned_extensions();
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
        "📑 Pages",
        "🧩 Components",
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

    let mut status_msg = String::from(
        "Use ↑/↓ to navigate, Enter to install, u to remove, Tab for category, q to quit.",
    );
    let mut status_color = Color::DarkGray;
    let mut search_query = String::new();
    let mut search_mode = false;

    loop {
        let current_category = categories[active_cat_idx];
        let filtered_extensions: Vec<&RegistryExtension> = registry
            .extensions
            .iter()
            .filter(|e| match current_category {
                "📑 Pages" => e.ext_type == "page",
                "🧩 Components" => e.ext_type == "component",
                "All" => true,
                _ => get_category(&e.id) == current_category,
            })
            .filter(|e| {
                if search_query.is_empty() {
                    true
                } else {
                    let q = search_query.to_lowercase();
                    e.name.to_lowercase().contains(&q)
                        || e.description.to_lowercase().contains(&q)
                        || e.id.to_lowercase().contains(&q)
                        || e.author.to_lowercase().contains(&q)
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
                    Constraint::Length(3), // Search bar
                    Constraint::Min(10),   // Main list & details
                    Constraint::Length(3), // Status & hotkeys footer
                ])
                .split(f.area());

            // 1. Header & Tabs
            let tab_titles: Vec<Line> = categories
                .iter()
                .map(|t| {
                    let count = match *t {
                        "📑 Pages" => registry
                            .extensions
                            .iter()
                            .filter(|e| e.ext_type == "page")
                            .count(),
                        "🧩 Components" => registry
                            .extensions
                            .iter()
                            .filter(|e| e.ext_type == "component")
                            .count(),
                        "All" => registry.extensions.len(),
                        _ => registry
                            .extensions
                            .iter()
                            .filter(|e| get_category(&e.id) == *t)
                            .count(),
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
                        .title(" 🧩 VANTA EXTENSION & WORKSPACE INSTALLER "),
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
            let search_border = if search_mode {
                Color::Cyan
            } else {
                Color::DarkGray
            };
            let search_title = if search_mode {
                " 🔍 Search (Esc to exit) "
            } else {
                " 🔍 Search (/ to focus) "
            };
            let search_p = Paragraph::new(format!("{}█", search_query))
                .style(Style::default().fg(Color::White))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(search_border))
                        .title(search_title),
                );
            f.render_widget(search_p, chunks[1]);

            let body_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
                .split(chunks[2]);

            // Left: Extension items list
            let items: Vec<ListItem> = filtered_extensions
                .iter()
                .map(|ext| {
                    let is_page = ext.ext_type == "page";
                    let is_installed = if is_page {
                        !ext.components.is_empty()
                            && ext
                                .components
                                .iter()
                                .all(|c| ext_dir.join(format!("{}.wasm", c)).exists())
                    } else {
                        ext_dir.join(format!("{}.wasm", ext.id)).exists()
                    };

                    let (badge, badge_color) = if is_installed {
                        ("✓", Color::Green)
                    } else {
                        (" ", Color::DarkGray)
                    };

                    let type_prefix = if is_page { "📑 " } else { "🧩 " };

                    let content = Line::from(vec![
                        Span::styled(
                            format!("[{}] ", badge),
                            Style::default()
                                .fg(badge_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            type_prefix,
                            Style::default().fg(if is_page { Color::Yellow } else { Color::Cyan }),
                        ),
                        Span::styled(
                            format!("{:<20} ", ext.id),
                            Style::default().fg(Color::White),
                        ),
                        Span::styled(&ext.name, Style::default().fg(Color::DarkGray)),
                    ]);
                    ListItem::new(content)
                })
                .collect();

            let list_title = format!(" {} ({}) ", current_category, filtered_extensions.len());
            let list_widget = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::White))
                        .title(list_title),
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
                let is_page = ext.ext_type == "page";
                let is_installed = if is_page {
                    !ext.components.is_empty()
                        && ext
                            .components
                            .iter()
                            .all(|c| ext_dir.join(format!("{}.wasm", c)).exists())
                } else {
                    ext_dir.join(format!("{}.wasm", ext.id)).exists()
                };

                let status_line = if is_page {
                    let installed_comps = ext
                        .components
                        .iter()
                        .filter(|c| ext_dir.join(format!("{}.wasm", c)).exists())
                        .count();

                    if is_installed {
                        Line::from(vec![
                            Span::styled(
                                "Status: ",
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                format!(
                                    "● FULLY INSTALLED ({}/{} components)",
                                    installed_comps,
                                    ext.components.len()
                                ),
                                Style::default()
                                    .fg(Color::Green)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ])
                    } else if installed_comps > 0 {
                        Line::from(vec![
                            Span::styled(
                                "Status: ",
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                format!(
                                    "◐ PARTIALLY INSTALLED ({}/{} components)",
                                    installed_comps,
                                    ext.components.len()
                                ),
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ])
                    } else {
                        Line::from(vec![
                            Span::styled(
                                "Status: ",
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled("○ NOT INSTALLED", Style::default().fg(Color::DarkGray)),
                        ])
                    }
                } else if is_installed {
                    Line::from(vec![
                        Span::styled(
                            "Status: ",
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            "● INSTALLED",
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!(" (~/.config/vanta/extensions/{}.wasm)", ext.id),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(
                            "Status: ",
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled("○ NOT INSTALLED", Style::default().fg(Color::Yellow)),
                    ])
                };

                let action_line = if is_page {
                    Line::from(vec![
                        Span::styled(
                            "Action: ",
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            "Press [Enter] to install full workspace page & configure config.toml",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ])
                } else if is_installed {
                    Line::from(vec![
                        Span::styled(
                            "Action: ",
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            "Press [Enter] to re-install, [u] to uninstall",
                            Style::default().fg(Color::Cyan),
                        ),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(
                            "Action: ",
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            "Press [Enter] or [i] to install this component",
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ])
                };

                let mut lines = vec![
                    Line::from(vec![
                        Span::styled(
                            if is_page { "📑 " } else { "🧩 " },
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::styled(
                            format!("{} ", ext.name),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("(v{})", ext.version),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("Type:     ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            if is_page {
                                "Page Workspace (Full Multi-Panel Layout)"
                            } else {
                                "Component Widget (Modular)"
                            },
                            Style::default()
                                .fg(if is_page { Color::Yellow } else { Color::Green })
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("ID:       ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            &ext.id,
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("Category: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(get_category(&ext.id), Style::default().fg(Color::Yellow)),
                    ]),
                    Line::from(vec![
                        Span::styled("Author:   ", Style::default().fg(Color::DarkGray)),
                        Span::styled(&ext.author, Style::default().fg(Color::White)),
                    ]),
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "Description:",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(Span::styled(
                        &ext.description,
                        Style::default().fg(Color::LightCyan),
                    )),
                    Line::from(""),
                ];

                if is_page && !ext.components.is_empty() {
                    lines.push(Line::from(vec![Span::styled(
                        "Bundled Component Extensions:",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )]));
                    for comp in &ext.components {
                        let comp_inst = ext_dir.join(format!("{}.wasm", comp)).exists();
                        let (c_badge, c_color) = if comp_inst {
                            ("✓", Color::Green)
                        } else {
                            ("○", Color::DarkGray)
                        };
                        lines.push(Line::from(vec![
                            Span::styled(format!("  [{}] ", c_badge), Style::default().fg(c_color)),
                            Span::styled(comp, Style::default().fg(Color::White)),
                        ]));
                    }
                    lines.push(Line::from(""));
                }

                lines.push(status_line);
                lines.push(action_line);

                Paragraph::new(lines)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(Color::White))
                            .title(if is_page {
                                " Page Workspace Details "
                            } else {
                                " Component Details "
                            }),
                    )
                    .wrap(Wrap { trim: true })
            } else {
                Paragraph::new("No item selected.").block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title(" Details "),
                )
            };
            f.render_widget(details_widget, body_chunks[1]);

            // 3. Footer
            let footer_lines = vec![
                Line::from(vec![
                    Span::styled(
                        " [q] Quit ",
                        Style::default().fg(Color::Black).bg(Color::White),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        " [↑/↓/j/k] Select ",
                        Style::default().fg(Color::White).bg(Color::DarkGray),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        " [Tab] Switch Category ",
                        Style::default().fg(Color::White).bg(Color::DarkGray),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        " [Enter/i] Install ",
                        Style::default().fg(Color::Black).bg(Color::Green),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        " [u] Remove ",
                        Style::default().fg(Color::Black).bg(Color::Red),
                    ),
                ]),
                Line::from(Span::styled(&status_msg, Style::default().fg(status_color))),
            ];

            let footer = Paragraph::new(footer_lines).alignment(Alignment::Left);
            f.render_widget(footer, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(150))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if search_mode {
                        match key.code {
                            KeyCode::Esc => {
                                search_mode = false;
                                status_msg = String::from("Exited search.");
                                status_color = Color::DarkGray;
                            }
                            KeyCode::Backspace => {
                                search_query.pop();
                                list_state.select(Some(0));
                            }
                            KeyCode::Char(c) => {
                                search_query.push(c);
                                list_state.select(Some(0));
                            }
                            KeyCode::Enter => {
                                search_mode = false;
                                status_msg = String::from("Press Enter again to install.");
                                status_color = Color::Cyan;
                            }
                            KeyCode::Up => {
                                let current = list_state.selected().unwrap_or(0);
                                if current > 0 {
                                    list_state.select(Some(current - 1));
                                }
                            }
                            KeyCode::Down => {
                                let current = list_state.selected().unwrap_or(0);
                                if !filtered_extensions.is_empty()
                                    && current + 1 < filtered_extensions.len()
                                {
                                    list_state.select(Some(current + 1));
                                }
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('/') | KeyCode::Char('s') => {
                                search_mode = true;
                                status_msg = String::from("Type to search...");
                                status_color = Color::Cyan;
                            }
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
                                status_msg = format!("Filtered by: {}", categories[active_cat_idx]);
                                status_color = Color::Cyan;
                            }
                            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                                if active_cat_idx == 0 {
                                    active_cat_idx = categories.len() - 1;
                                } else {
                                    active_cat_idx -= 1;
                                }
                                list_state.select(Some(0));
                                status_msg = format!("Filtered by: {}", categories[active_cat_idx]);
                                status_color = Color::Cyan;
                            }
                            KeyCode::Enter | KeyCode::Char('i') | KeyCode::Char(' ') => {
                                if let Some(selected_idx) = list_state.selected() {
                                    if let Some(ext) = filtered_extensions.get(selected_idx) {
                                        if ext.ext_type == "page" {
                                            match install_page_ext(ext, &registry.extensions) {
                                                Ok(msg) => {
                                                    status_msg = format!("✓ {}", msg);
                                                    status_color = Color::Green;
                                                }
                                                Err(e) => {
                                                    status_msg =
                                                        format!("✗ Failed to install page: {}", e);
                                                    status_color = Color::Red;
                                                }
                                            }
                                        } else {
                                            match download_and_install_ext(ext) {
                                                Ok(path) => {
                                                    status_msg = format!(
                                                        "✓ Installed '{}' to {}",
                                                        ext.id,
                                                        path.display()
                                                    );
                                                    status_color = Color::Green;
                                                }
                                                Err(e) => {
                                                    status_msg = format!(
                                                        "✗ Failed to install '{}': {}",
                                                        ext.id, e
                                                    );
                                                    status_color = Color::Red;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('u') | KeyCode::Delete | KeyCode::Backspace => {
                                if let Some(selected_idx) = list_state.selected() {
                                    if let Some(ext) = filtered_extensions.get(selected_idx) {
                                        if ext.ext_type == "page" {
                                            let mut removed_count = 0;
                                            for comp in &ext.components {
                                                let comp_file =
                                                    ext_dir.join(format!("{}.wasm", comp));
                                                if comp_file.exists()
                                                    && fs::remove_file(&comp_file).is_ok()
                                                {
                                                    removed_count += 1;
                                                }
                                            }
                                            let _ = vanta::config::remove_page_from_config(
                                                &ext.name,
                                                &ext.components,
                                            );
                                            status_msg = format!(
                                            "✓ Uninstalled page '{}' (removed {} components & cleaned config)",
                                            ext.name, removed_count
                                        );
                                            status_color = Color::Yellow;
                                        } else {
                                            let ext_file = ext_dir.join(format!("{}.wasm", ext.id));
                                            let existed = ext_file.exists();
                                            if existed {
                                                match fs::remove_file(&ext_file) {
                                                    Ok(_) => {
                                                        let _ =
                                                        vanta::config::remove_component_from_config(
                                                            &ext.id,
                                                        );
                                                        status_msg = format!(
                                                            "✓ Uninstalled '{}' ({})",
                                                            ext.id,
                                                            ext_file.display()
                                                        );
                                                        status_color = Color::Yellow;
                                                    }
                                                    Err(e) => {
                                                        status_msg = format!(
                                                            "✗ Failed to remove '{}': {}",
                                                            ext.id, e
                                                        );
                                                        status_color = Color::Red;
                                                    }
                                                }
                                            } else {
                                                let _ = vanta::config::remove_component_from_config(
                                                    &ext.id,
                                                );
                                                status_msg = format!(
                                                    "✓ Cleaned '{}' from config.toml",
                                                    ext.id
                                                );
                                                status_color = Color::Yellow;
                                            }
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
    }

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;
    let _ = terminal.show_cursor();

    Ok(())
}
