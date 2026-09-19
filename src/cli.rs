use clap::{Parser, Subcommand};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::PathBuf;

const REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/ziuus/vanta-integrations/main/registry.json";

#[derive(Parser)]
#[command(name = "vanta", version = env!("CARGO_PKG_VERSION"), about = "Aesthetic Rust TUI system dashboard")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Search for available extensions in the community registry
    Search { query: Option<String> },
    /// Install an extension from the registry
    Install { id: String },
    /// Enable an installed extension
    Enable { id: String },
    /// Disable an installed extension
    Disable { id: String },
    /// List installed extensions
    List,
    /// Update installed extensions from the registry (specify an ID or update all)
    Update { id: Option<String> },
    /// Remove an installed extension
    Remove { id: String },
}

#[derive(Deserialize, Debug)]
struct Registry {
    api_version: String,
    extensions: Vec<RegistryExtension>,
}

#[derive(Deserialize, Debug)]
struct RegistryExtension {
    id: String,
    name: String,
    description: String,
    version: String,
    api_version: String,
    author: String,
    wasm_url: String,
    sha256: String,
}

fn get_extensions_dir() -> PathBuf {
    let mut dir = directories::ProjectDirs::from("", "", "vanta")
        .map(|p| p.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    dir.push("extensions");
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn handle_cli(cli: Cli) -> bool {
    if let Some(cmd) = cli.command {
        match cmd {
            Commands::Search { query } => search(query),
            Commands::Install { id } => install(id),
            Commands::Enable { id } => enable(id),
            Commands::Disable { id } => disable(id),
            Commands::List => list(),
            Commands::Update { id } => update(id),
            Commands::Remove { id } => remove(id),
        }
        return true;
    }
    false
}

fn fetch_registry() -> Result<Registry, Box<dyn std::error::Error>> {
    let res = ureq::get(REGISTRY_URL).call()?;
    let registry: Registry = serde_json::from_reader(res.into_body().into_reader())?;
    Ok(registry)
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
            println!("  {:<12} {}", ext.id, ext.name);
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

    let ext = match registry.extensions.into_iter().find(|e| e.id == id) {
        Some(e) => e,
        None => {
            eprintln!("Extension '{}' not found in the registry.", id);
            return;
        }
    };

    println!("Installing '{}' v{}...", ext.name, ext.version);
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
    if !ext.api_version.starts_with("0.9") {
        return Err(format!(
            "'{}' requires Vanta UI API {}, but your Vanta supports API 0.9",
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

    if hash != ext.sha256 {
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

fn update(id: Option<String>) {
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
    let wasm_path = ext_dir.join(format!("{}.wasm", id));

    if wasm_path.exists() {
        if let Err(e) = fs::remove_file(&wasm_path) {
            eprintln!("Failed to remove extension: {}", e);
        } else {
            println!("✓ Removed extension '{}'", id);
        }
    } else {
        eprintln!("Extension '{}' is not installed.", id);
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
    let path = get_config_path();
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return eprintln!("Could not read config.toml"),
    };

    let target1 = format!("\"{}\", ", id);
    let target2 = format!("\"{}\"", id);

    if content.contains(&target1) {
        let new_content = content.replace(&target1, "");
        fs::write(&path, new_content).unwrap();
        println!("✓ Disabled '{}' in config.toml", id);
    } else if content.contains(&target2) {
        let new_content = content.replace(&target2, "");
        fs::write(&path, new_content).unwrap();
        println!("✓ Disabled '{}' in config.toml", id);
    } else {
        println!("Extension '{}' was not enabled.", id);
    }
}
