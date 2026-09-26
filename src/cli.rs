use clap::{Parser, Subcommand};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::PathBuf;

const REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/ziuus/vanta-integrations/main/registry.json";

#[derive(Parser)]
#[command(
    name = "vanta",
    version = env!("CARGO_PKG_VERSION"),
    about = "Aesthetic Rust TUI system dashboard",
    disable_version_flag = true
)]
pub struct Cli {
    /// Print version
    #[arg(short = 'v', short_alias = 'V', long = "version", action = clap::ArgAction::Version)]
    pub version: Option<bool>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Browse community extensions categorized by workspace (interactive with -i)
    Browse {
        /// Optional category or keyword filter
        filter: Option<String>,
        /// Open interactive terminal menu browser and installer
        #[arg(short = 'i', long = "interactive")]
        interactive: bool,
    },
    /// Interactive menu installer to browse, install, and manage extensions
    Menu,
    /// Open the interactive configuration menu
    Config,
    /// Open the initial setup wizard
    Setup,
    /// Search for available extensions in the community registry
    Search { query: Option<String> },
    /// Install an extension from the registry (launches menu if no ID given)
    Install {
        /// Optional extension ID to install. If omitted, launches menu installer
        id: Option<String>,
    },
    /// Enable an installed extension
    Enable { id: String },
    /// Disable an installed extension
    Disable { id: String },
    /// List installed extensions
    List,
    /// Update installed extensions from the registry, or update Vanta itself with --self
    Update {
        /// Optional extension ID to update, or 'self' to update Vanta itself
        id: Option<String>,
        /// Update Vanta host binary itself
        #[arg(long = "self")]
        self_update: bool,
    },
    /// Remove an installed extension
    Remove { id: String },
    /// Symlink a local WASM extension for development without publishing
    Link { path: PathBuf },
}

#[derive(Deserialize, Debug)]
struct Registry {
    api_version: String,
    extensions: Vec<RegistryExtension>,
}

use crate::cli_menu::RegistryExtension;

fn get_extensions_dir() -> PathBuf {
    let mut dir = directories::ProjectDirs::from("", "", "vanta")
        .map(|p| p.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    dir.push("extensions");
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub enum RunMode {
    Exit,
    Normal,
    Config,
    Setup,
}

pub fn handle_cli(cli: Cli) -> RunMode {
    if let Some(cmd) = cli.command {
        match cmd {
            Commands::Browse {
                filter,
                interactive,
            } => {
                if interactive {
                    let _ = crate::cli_menu::run_menu_installer();
                } else {
                    browse(filter);
                }
            }
            Commands::Config => return RunMode::Config,
            Commands::Setup => return RunMode::Setup,
            Commands::Menu => {
                let _ = crate::cli_menu::run_menu_installer();
            }
            Commands::Search { query } => search(query),
            Commands::Install { id } => match id {
                Some(single_id) => install(single_id),
                None => {
                    let _ = crate::cli_menu::run_menu_installer();
                }
            },
            Commands::Enable { id } => enable(id),
            Commands::Disable { id } => disable(id),
            Commands::List => list(),
            Commands::Update { id, self_update } => update(id, self_update),
            Commands::Remove { id } => remove(id),
            Commands::Link { path } => link(path),
        }
        return RunMode::Exit;
    }
    RunMode::Normal
}

fn fetch_registry() -> Result<Registry, Box<dyn std::error::Error>> {
    let res = ureq::get(REGISTRY_URL).call()?;
    let registry: Registry = serde_json::from_reader(res.into_body().into_reader())?;
    Ok(registry)
}

fn get_category(id: &str) -> &'static str {
    if id.starts_with("cryptopulse") || id.starts_with("crypto") {
        "CryptoPulse & Market Visualizers"
    } else if id.starts_with("filespace") {
        "FileSpace Terminal File Manager"
    } else if id.starts_with("mediadeck") {
        "MediaDeck Audio Workstation"
    } else if id == "security" {
        "Security & Vulnerability Monitor"
    } else {
        "Deep Observability & Sentry"
    }
}

fn browse(filter: Option<String>) {
    println!("Fetching Vanta extensions registry...");
    let registry = match fetch_registry() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to fetch registry: {}", e);
            return;
        }
    };

    let ext_dir = get_extensions_dir();
    let filter_lower = filter.as_ref().map(|f| f.to_lowercase());

    let (pages, components): (Vec<_>, Vec<_>) = registry
        .extensions
        .into_iter()
        .partition(|e| e.ext_type == "page");

    let show_pages = match &filter_lower {
        Some(f) if f == "page" || f == "pages" => true,
        Some(f) if f == "component" || f == "components" || f == "widget" || f == "widgets" => {
            false
        }
        _ => true,
    };

    let show_components = !matches!(&filter_lower, Some(f) if f == "page" || f == "pages");

    println!("\n╔════════════════════════════════════════════════════════════════════╗");
    println!("║                   VANTA EXTENSION DIRECTORY                        ║");
    println!("╚════════════════════════════════════════════════════════════════════╝\n");

    let mut total_shown = 0;

    // 1. PAGE EXTENSIONS
    if show_pages && !pages.is_empty() {
        let matching_pages: Vec<_> = pages
            .iter()
            .filter(|e| {
                if let Some(ref f) = filter_lower {
                    if f == "page" || f == "pages" {
                        true
                    } else {
                        e.id.to_lowercase().contains(f)
                            || e.name.to_lowercase().contains(f)
                            || e.description.to_lowercase().contains(f)
                    }
                } else {
                    true
                }
            })
            .collect();

        if !matching_pages.is_empty() {
            println!("📑 PAGE WORKSPACES (Full multi-panel workspaces)\n");
            for page in matching_pages {
                let all_installed = !page.components.is_empty()
                    && page
                        .components
                        .iter()
                        .all(|c| ext_dir.join(format!("{}.wasm", c)).exists());
                let status_badge = if all_installed {
                    "\x1b[32m[installed]\x1b[0m"
                } else {
                    "\x1b[90m[available]\x1b[0m"
                };

                println!("  {:<24} {:<24} {}", page.id, page.name, status_badge);
                println!("    {}", page.description);
                println!("    Bundled components: {}", page.components.join(", "));
                if !all_installed {
                    println!(
                        "    \x1b[36m→ Install workspace: vanta install {}\x1b[0m",
                        page.id
                    );
                }
                println!();
                total_shown += 1;
            }
        }
    }

    // 2. COMPONENT EXTENSIONS
    if show_components && !components.is_empty() {
        println!("🧩 COMPONENT EXTENSIONS (Modular building blocks)\n");
        let categories = [
            "CryptoPulse & Market Visualizers",
            "FileSpace Terminal File Manager",
            "MediaDeck Audio Workstation",
            "Deep Observability & Sentry",
            "Security & Vulnerability Monitor",
        ];

        for cat in categories {
            let cat_extensions: Vec<_> = components
                .iter()
                .filter(|e| get_category(&e.id) == cat)
                .filter(|e| {
                    if let Some(ref f) = filter_lower {
                        if f == "component" || f == "components" || f == "widget" || f == "widgets"
                        {
                            true
                        } else {
                            cat.to_lowercase().contains(f)
                                || e.id.to_lowercase().contains(f)
                                || e.name.to_lowercase().contains(f)
                                || e.description.to_lowercase().contains(f)
                        }
                    } else {
                        true
                    }
                })
                .collect();

            if cat_extensions.is_empty() {
                continue;
            }

            println!("━━━ {} ━━━", cat);

            for ext in cat_extensions {
                let is_installed = ext_dir.join(format!("{}.wasm", ext.id)).exists();
                let status_badge = if is_installed {
                    "\x1b[32m[installed]\x1b[0m"
                } else {
                    "\x1b[90m[available]\x1b[0m"
                };

                println!("  {:<24} {:<24} {}", ext.id, ext.name, status_badge);
                println!("    {}", ext.description);
                if !is_installed {
                    println!("    \x1b[36m→ Install: vanta install {}\x1b[0m", ext.id);
                }
                println!();
                total_shown += 1;
            }
        }
    }

    if total_shown == 0 {
        println!("No extensions found matching your filter.");
    } else {
        println!(
            "Shown {} item(s). Use `vanta install <id>` to install any extension or workspace.",
            total_shown
        );
        println!("Tip: Run `vanta browse pages` or `vanta browse components` to view separately.");
    }
}

fn search(query: Option<String>) {
    println!("Fetching Vanta extensions registry...");
    let registry = match fetch_registry() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to fetch registry: {}", e);
            return;
        }
    };

    println!("\nVanta Extensions (API v{})\n", registry.api_version);

    let mut found = 0;
    for ext in registry.extensions {
        let matches = match &query {
            Some(q) => ext.id.contains(q) || ext.name.contains(q) || ext.description.contains(q),
            None => true,
        };

        if matches {
            let type_tag = if ext.ext_type == "page" {
                "[PAGE]"
            } else {
                "[COMPONENT]"
            };
            println!("  {:<12} {:<24} {}", ext.id, ext.name, type_tag);
            println!("               {}", ext.description);
            println!(
                "               v{} (API v{}) by {}\n",
                ext.version, ext.api_version, ext.author
            );
            found += 1;
        }
    }

    if found == 0 {
        println!("No extensions found matching your query.");
    }
}

fn install(id: String) {
    println!("Fetching registry...");
    let registry = match fetch_registry() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to fetch registry: {}", e);
            return;
        }
    };

    let ext = match registry.extensions.iter().find(|e| e.id == id) {
        Some(e) => e.clone(),
        None => {
            eprintln!("Extension '{}' not found in the registry.", id);
            return;
        }
    };

    if ext.ext_type == "page" {
        println!(
            "Installing page workspace '{}' (v{})...",
            ext.name, ext.version
        );
        println!(
            "Bundled component extensions: {}\n",
            ext.components.join(", ")
        );

        match crate::cli_menu::install_page_ext(&ext, &registry.extensions) {
            Ok(msg) => {
                println!("✓ {}", msg);
                println!("✓ Configured '{}' in ~/.config/vanta/config.toml", ext.name);
                println!("\nLaunch Vanta and press Tab to navigate to the new workspace!");
            }
            Err(e) => eprintln!("✗ Failed to install page workspace: {}", e),
        }
        return;
    }

    println!("Installing component '{}' v{}...", ext.name, ext.version);
    println!("  Permissions requested:");
    println!("    UI             ✓");
    println!("    Configuration  ✓");
    println!("    Network        ✗");
    println!("    Filesystem     ✗");
    println!("    Processes      ✗\n");

    match download_and_install(&ext) {
        Ok(path) => {
            println!("✓ Installed to {}", path.display());
            println!(
                "\nYou can enable it by adding '{}' to the 'enabled' array in your config.toml",
                ext.id
            );
        }
        Err(e) => eprintln!("✗ {}", e),
    }
}

fn download_and_install(ext: &RegistryExtension) -> Result<PathBuf, String> {
    if !ext.api_version.starts_with("0.9") && !ext.api_version.starts_with("0.10") {
        return Err(format!(
            "'{}' requires Vanta UI API {}, but your Vanta supports API 0.10",
            ext.id, ext.api_version
        ));
    }

    // Download
    println!("Downloading {}...", ext.wasm_url);
    let res = ureq::get(&ext.wasm_url)
        .call()
        .map_err(|e| format!("Failed to download extension artifact: {}", e))?;

    let mut buf = Vec::new();
    res.into_body()
        .into_reader()
        .read_to_end(&mut buf)
        .map_err(|e| format!("Failed to read artifact: {}", e))?;
    println!("✓ Downloaded {}.wasm", ext.id);

    // Verify
    let mut hasher = Sha256::new();
    hasher.update(&buf);
    let hash: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();

    if !ext.sha256.is_empty() && hash != ext.sha256 {
        return Err(format!(
            "Security Error: Artifact hash mismatch!\n  Expected: {}\n  Got:      {}",
            ext.sha256, hash
        ));
    }
    println!("✓ Verified SHA-256 checksum");

    // Install atomically
    let ext_dir = get_extensions_dir();
    let wasm_path = ext_dir.join(format!("{}.wasm", ext.id));
    let tmp_path = ext_dir.join(format!("{}.wasm.tmp", ext.id));

    fs::write(&tmp_path, buf).map_err(|e| format!("Failed to write extension to disk: {}", e))?;

    if let Err(e) = fs::rename(&tmp_path, &wasm_path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(format!("Failed to atomically install extension: {}", e));
    }

    Ok(wasm_path)
}

fn update_self() {
    let current_version = env!("CARGO_PKG_VERSION");
    println!("Updating Vanta host (current: v{})...", current_version);

    let status = std::process::Command::new("npm")
        .args(["install", "-g", "@ziuus/vanta@latest"])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("✓ Vanta successfully updated via npm!");
        }
        Ok(s) => {
            eprintln!("npm update exited with status {}. To update manually:", s);
            println!("  npm:   npm install -g @ziuus/vanta");
            println!("  cargo: cargo install --git https://github.com/ziuus/vanta");
        }
        Err(_) => {
            println!("npm command not found. To update manually:");
            println!("  cargo install --git https://github.com/ziuus/vanta");
        }
    }
}

fn update(id: Option<String>, self_update: bool) {
    if self_update || id.as_deref() == Some("self") || id.as_deref() == Some("vanta") {
        update_self();
        return;
    }

    let update_everything = id.is_none();

    println!("Fetching registry...");
    let registry = match fetch_registry() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to fetch registry: {}", e);
            return;
        }
    };

    let ext_dir = get_extensions_dir();
    let to_update: Vec<String> = match id {
        Some(single_id) => {
            let path = ext_dir.join(format!("{}.wasm", single_id));
            if !path.exists() {
                eprintln!("Extension '{}' is not installed.", single_id);
                return;
            }
            vec![single_id]
        }
        None => {
            let mut installed = Vec::new();
            if let Ok(entries) = fs::read_dir(&ext_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            installed.push(stem.to_string());
                        }
                    }
                }
            }
            if installed.is_empty() {
                println!("No installed extensions found to update.");
                if update_everything {
                    println!("\n");
                    update_self();
                }
                return;
            }
            installed
        }
    };

    println!("Checking updates for {} extension(s)...\n", to_update.len());
    let mut updated = 0;
    for ext_id in to_update {
        if let Some(ext) = registry.extensions.iter().find(|e| e.id == ext_id) {
            println!("Updating '{}' (v{})...", ext.name, ext.version);
            match download_and_install(ext) {
                Ok(_) => {
                    println!("✓ Successfully updated '{}' to v{}\n", ext.id, ext.version);
                    updated += 1;
                }
                Err(e) => eprintln!("✗ Failed to update '{}': {}\n", ext.id, e),
            }
        } else {
            eprintln!("⚠ Extension '{}' not found in registry (skipped)\n", ext_id);
        }
    }

    println!("Completed: {} extension(s) updated.", updated);
    if update_everything {
        println!("\n");
        update_self();
    }
}

fn list() {
    let ext_dir = get_extensions_dir();
    println!("Installed extensions in {}:\n", ext_dir.display());

    let mut found = 0;
    if let Ok(entries) = fs::read_dir(ext_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                let name = path.file_stem().unwrap_or_default().to_string_lossy();
                println!("  - {}", name);
                found += 1;
            }
        }
    }

    if found == 0 {
        println!("  No extensions installed.");
    }
}

fn remove(id: String) {
    let ext_dir = get_extensions_dir();
    let registry = fetch_registry().ok();

    // 1. Check if `id` corresponds to a page extension in the registry
    let matched_page = registry.as_ref().and_then(|r| {
        r.extensions.iter().find(|e| {
            e.ext_type == "page"
                && (e.id.eq_ignore_ascii_case(&id) || e.name.eq_ignore_ascii_case(&id))
        })
    });

    if let Some(page) = matched_page {
        let mut removed_count = 0;
        for comp in &page.components {
            let comp_file = ext_dir.join(format!("{}.wasm", comp));
            if comp_file.exists() && fs::remove_file(&comp_file).is_ok() {
                removed_count += 1;
            }
        }
        let _ = vanta::config::remove_page_from_config(&page.name, &page.components);
        println!(
            "✓ Removed page workspace '{}' (deleted {} component files, pruned config.toml)",
            page.name, removed_count
        );
        return;
    }

    // 2. Check if `id` corresponds to a page defined in config.toml directly
    let config = vanta::config::Config::load();
    if let Some(cfg_page) = config
        .pages
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(&id))
    {
        let comps: Vec<String> = cfg_page.layout.iter().flat_map(|col| col.clone()).collect();
        let mut removed_count = 0;
        for comp in &comps {
            let comp_file = ext_dir.join(format!("{}.wasm", comp));
            if comp_file.exists() && fs::remove_file(&comp_file).is_ok() {
                removed_count += 1;
            }
        }
        let _ = vanta::config::remove_page_from_config(&cfg_page.name, &comps);
        println!(
            "✓ Removed page workspace '{}' from config.toml (deleted {} component files)",
            cfg_page.name, removed_count
        );
        return;
    }

    // 3. Otherwise treat as single extension component
    let wasm_path = ext_dir.join(format!("{}.wasm", id));
    let existed_on_disk = wasm_path.exists();
    if existed_on_disk {
        if let Err(e) = fs::remove_file(&wasm_path) {
            eprintln!("Failed to remove extension: {}", e);
        } else {
            println!("✓ Removed extension '{}'", id);
        }
    }

    let _ = vanta::config::remove_component_from_config(&id);
    if existed_on_disk {
        println!("✓ Uninstalled extension '{}' and pruned config.toml", id);
    } else {
        println!("✓ Pruned extension '{}' from config.toml", id);
    }
}

fn get_config_path() -> PathBuf {
    directories::ProjectDirs::from("", "", "vanta")
        .map(|p| p.config_dir().join("config.toml"))
        .unwrap_or_else(|| PathBuf::from("config.toml"))
}

fn enable(id: String) {
    let path = get_config_path();
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            // Create a default config if it doesn't exist
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let default_config = "[extensions]\nenabled = []\n";
            let _ = fs::write(&path, default_config);
            default_config.to_string()
        }
    };

    if content.contains(&format!("\"{}\"", id)) || content.contains(&format!("'{}'", id)) {
        println!("Extension '{}' is already enabled.", id);
        return;
    }

    if let Some(idx) = content.find("enabled = [") {
        let mut new_content = String::new();
        let (before, after) = content.split_at(idx + "enabled = [".len());
        new_content.push_str(before);
        new_content.push_str(&format!("\"{}\", ", id));
        new_content.push_str(after);

        fs::write(&path, new_content).unwrap();
        println!("✓ Enabled '{}' in config.toml", id);
    } else {
        // Append [extensions] if missing, or just append enabled array
        let mut new_content = content.clone();
        if !new_content.contains("[extensions]") {
            new_content.push_str("\n[extensions]\n");
        }
        if !new_content.contains("enabled = [") {
            new_content.push_str(&format!("enabled = [\"{}\"]\n", id));
        }
        fs::write(&path, new_content).unwrap();
        println!("✓ Enabled '{}' in config.toml", id);
    }
}

fn disable(id: String) {
    match vanta::config::remove_component_from_config(&id) {
        Ok(_) => println!("✓ Disabled '{}' in config.toml", id),
        Err(e) => eprintln!("Failed to disable extension: {}", e),
    }
}

fn link(path: PathBuf) {
    if !path.exists() {
        eprintln!("File does not exist: {}", path.display());
        return;
    }
    if path.extension().and_then(|s| s.to_str()) != Some("wasm")
        && path.extension().and_then(|s| s.to_str()) != Some("toml")
    {
        eprintln!("File must be a .wasm or .toml file.");
        return;
    }

    let stem = path.file_stem().unwrap().to_string_lossy().to_string();
    let ext_dir = if path.extension().and_then(|s| s.to_str()) == Some("toml") {
        let mut dir = directories::ProjectDirs::from("", "", "vanta")
            .map(|p| p.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        dir.push("themes");
        fs::create_dir_all(&dir).unwrap();
        dir
    } else {
        get_extensions_dir()
    };

    let ext_str = path.extension().unwrap().to_string_lossy();
    let target = ext_dir.join(format!("{}.{}", stem, ext_str));

    let abs_path = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
    let _ = fs::remove_file(&target);

    #[cfg(unix)]
    let res = std::os::unix::fs::symlink(&abs_path, &target);
    #[cfg(not(unix))]
    let res = fs::copy(&abs_path, &target).map(|_| ());

    match res {
        Ok(_) => {
            println!("✓ Linked '{}' to {}", stem, target.display());
            if ext_str == "wasm" {
                enable(stem.clone());
                println!(
                    "✓ Enabled '{}' in config.toml. You can now test it in Vanta!",
                    stem
                );
            } else {
                println!("✓ Theme '{}' linked. It is now available in Vanta!", stem);
            }
        }
        Err(e) => eprintln!("✗ Failed to link: {}", e),
    }
}
