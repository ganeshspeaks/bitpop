use dirs;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
#[allow(dead_code)]
pub struct AppEntry {
    pub name: String,
    pub exec: String,
    pub icon: String,
    pub desktop_file: PathBuf,
}

pub fn load_desktop_apps() -> Vec<AppEntry> {
    let mut apps = Vec::new();
    let app_dirs = [
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        dirs::data_dir()
            .map(|d| d.join("applications"))
            .unwrap_or_default(),
    ];

    for dir in &app_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                    if let Some(app) = parse_desktop_file(&path) {
                        if !apps.iter().any(|a: &AppEntry| a.name == app.name) {
                            apps.push(app);
                        }
                    }
                }
            }
        }
    }

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

fn parse_desktop_file(path: &Path) -> Option<AppEntry> {
    let content = fs::read_to_string(path).ok()?;

    if !content.contains("[Desktop Entry]") {
        return None;
    }

    if content.contains("NoDisplay=true") {
        return None;
    }

    if content.contains("Type=Application") {
        let name = extract_field(&content, "Name").unwrap_or_default();
        let exec = extract_field(&content, "Exec").unwrap_or_default();
        let icon = extract_field(&content, "Icon")
            .unwrap_or_else(|| "application-x-executable".to_string());

        if !name.is_empty() && !exec.is_empty() {
            return Some(AppEntry {
                name,
                exec,
                icon,
                desktop_file: path.to_path_buf(),
            });
        }
    }

    None
}

fn extract_field(content: &str, field: &str) -> Option<String> {
    for line in content.lines() {
        if line.starts_with(&format!("{}=", field)) {
            let value = line.splitn(2, '=').nth(1)?;
            let cleaned = value
                .replace("%f", "")
                .replace("%F", "")
                .replace("%u", "")
                .replace("%U", "")
                .replace("%i", "")
                .replace("%c", "")
                .replace("%k", "")
                .trim()
                .to_string();
            return Some(cleaned);
        }
    }
    None
}

pub fn launch_app(app: &AppEntry) {
    let _ = std::process::Command::new("gtk-launch")
        .arg(&app.desktop_file.file_name().unwrap_or_default())
        .spawn();
}
