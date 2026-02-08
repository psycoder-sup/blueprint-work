use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const SETTINGS_DIR: &str = ".blueprint";
const SETTINGS_FILE: &str = "setting.json";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub project_id: Option<String>,
}

impl Settings {
    /// Reads `.blueprint/setting.json` from `std::env::current_dir()`.
    /// Returns `Settings` with `None` project_id if file is missing or malformed.
    pub fn load() -> Self {
        Self::load_from(std::env::current_dir().ok())
    }

    fn load_from(cwd: Option<PathBuf>) -> Self {
        let Some(cwd) = cwd else {
            return Self::default();
        };
        let path = cwd.join(SETTINGS_DIR).join(SETTINGS_FILE);
        Self::read_file(&path).unwrap_or_default()
    }

    fn read_file(path: &Path) -> Option<Self> {
        let data = fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    /// Returns the path to the settings file relative to a given directory.
    pub fn path_in(dir: &Path) -> PathBuf {
        dir.join(SETTINGS_DIR).join(SETTINGS_FILE)
    }

    /// Check whether `.blueprint/setting.json` exists in the given directory.
    pub fn exists_in(dir: &Path) -> bool {
        Self::path_in(dir).exists()
    }

    /// Check whether `.blueprint/` directory exists in the given directory.
    pub fn blueprint_dir_exists_in(dir: &Path) -> bool {
        dir.join(SETTINGS_DIR).is_dir()
    }

    /// Write settings to a specific directory (used by TUI).
    pub fn save_to(dir: &Path, project_id: &str) -> std::io::Result<()> {
        let settings_dir = dir.join(SETTINGS_DIR);
        fs::create_dir_all(&settings_dir)?;

        let settings = Settings {
            project_id: Some(project_id.to_string()),
        };
        let json = serde_json::to_string_pretty(&settings)
            .map_err(std::io::Error::other)?;
        fs::write(settings_dir.join(SETTINGS_FILE), json.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_missing_file_returns_default() {
        let dir = TempDir::new().unwrap();
        let settings = Settings::load_from(Some(dir.path().to_path_buf()));
        assert!(settings.project_id.is_none());
    }

    #[test]
    fn test_load_valid_file() {
        let dir = TempDir::new().unwrap();
        let bp_dir = dir.path().join(".blueprint");
        fs::create_dir_all(&bp_dir).unwrap();
        fs::write(
            bp_dir.join("setting.json"),
            r#"{"project_id": "abc123"}"#,
        )
        .unwrap();

        let settings = Settings::load_from(Some(dir.path().to_path_buf()));
        assert_eq!(settings.project_id.as_deref(), Some("abc123"));
    }

    #[test]
    fn test_load_malformed_file_returns_default() {
        let dir = TempDir::new().unwrap();
        let bp_dir = dir.path().join(".blueprint");
        fs::create_dir_all(&bp_dir).unwrap();
        fs::write(bp_dir.join("setting.json"), "not json").unwrap();

        let settings = Settings::load_from(Some(dir.path().to_path_buf()));
        assert!(settings.project_id.is_none());
    }

    #[test]
    fn test_save_to_creates_file() {
        let dir = TempDir::new().unwrap();
        Settings::save_to(dir.path(), "proj_001").unwrap();

        let settings = Settings::load_from(Some(dir.path().to_path_buf()));
        assert_eq!(settings.project_id.as_deref(), Some("proj_001"));
    }

    #[test]
    fn test_exists_in() {
        let dir = TempDir::new().unwrap();
        assert!(!Settings::exists_in(dir.path()));

        Settings::save_to(dir.path(), "proj_001").unwrap();
        assert!(Settings::exists_in(dir.path()));
    }

    #[test]
    fn test_blueprint_dir_exists_in() {
        let dir = TempDir::new().unwrap();
        assert!(!Settings::blueprint_dir_exists_in(dir.path()));

        fs::create_dir_all(dir.path().join(".blueprint")).unwrap();
        assert!(Settings::blueprint_dir_exists_in(dir.path()));
    }
}
