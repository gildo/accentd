use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub popup: PopupConfig,
    #[serde(default)]
    pub locale: LocaleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "GeneralConfig::default_threshold")]
    pub threshold_ms: u64,
    #[serde(default = "GeneralConfig::default_enabled")]
    pub enabled: bool,
}

impl GeneralConfig {
    fn default_threshold() -> u64 { 300 }
    fn default_enabled() -> bool { true }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            threshold_ms: 300,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopupConfig {
    #[serde(default = "PopupConfig::default_font_size")]
    pub font_size: u32,
    #[serde(default = "PopupConfig::default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "PopupConfig::default_keep_open")]
    pub keep_open: bool,
}

impl PopupConfig {
    fn default_font_size() -> u32 { 24 }
    fn default_timeout() -> u64 { 5000 }
    fn default_keep_open() -> bool { true }
}

impl Default for PopupConfig {
    fn default() -> Self {
        Self {
            font_size: 24,
            timeout_ms: 5000,
            keep_open: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleConfig {
    #[serde(default = "LocaleConfig::default_active")]
    pub active: String,
    #[serde(flatten)]
    pub locales: HashMap<String, HashMap<String, Vec<String>>>,
}

impl LocaleConfig {
    fn default_active() -> String { "it".into() }
}

impl Default for LocaleConfig {
    fn default() -> Self {
        Self {
            active: "it".into(),
            locales: HashMap::new(),
        }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/etc"))
            .join("accentd")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            Self::load_from(&path)
        } else {
            Ok(Self::default())
        }
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("reading config from {}", path.display()))?;
        toml::from_str(&contents).with_context(|| "parsing config TOML")
    }

    pub fn load_locale_map(&self) -> Result<HashMap<String, Vec<String>>> {
        // Inline locales from config file
        if let Some(locale_map) = self.locale.locales.get(&self.locale.active) {
            if !locale_map.is_empty() {
                return Ok(locale_map.clone());
            }
        }

        // Runtime locale files
        for dir in &[
            Self::config_dir().join("locales"),
            PathBuf::from("/usr/share/accentd/locales"),
        ] {
            let path = dir.join(format!("{}.toml", self.locale.active));
            if path.exists() {
                let contents = std::fs::read_to_string(&path)
                    .with_context(|| format!("reading locale {}", path.display()))?;
                return toml::from_str(&contents).with_context(|| "parsing locale TOML");
            }
        }

        // Built-in
        let builtin = crate::charmap::builtin_locale(&self.locale.active);
        if !builtin.is_empty() {
            return Ok(builtin);
        }

        anyhow::bail!("locale '{}' not found", self.locale.active)
    }
}

pub fn socket_path() -> PathBuf {
    // ACCENTD_SOCK env var overrides for testing.
    // Default: /run/accentd/accentd.sock (created by RuntimeDirectory=accentd in systemd).
    if let Ok(path) = std::env::var("ACCENTD_SOCK") {
        return PathBuf::from(path);
    }
    PathBuf::from("/run/accentd/accentd.sock")
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- spec: defaults ---

    #[test]
    fn default_threshold_is_300ms() {
        let config = Config::default();
        assert_eq!(config.general.threshold_ms, 300);
    }

    #[test]
    fn default_enabled_is_true() {
        let config = Config::default();
        assert!(config.general.enabled);
    }

    #[test]
    fn default_locale_is_italian() {
        let config = Config::default();
        assert_eq!(config.locale.active, "it");
    }

    #[test]
    fn default_popup_font_size_is_24() {
        let config = Config::default();
        assert_eq!(config.popup.font_size, 24);
    }

    #[test]
    fn default_popup_timeout_is_5000() {
        let config = Config::default();
        assert_eq!(config.popup.timeout_ms, 5000);
    }

    #[test]
    fn default_popup_keep_open_is_true() {
        let config = Config::default();
        assert!(config.popup.keep_open);
    }

    // --- spec: TOML parsing ---

    #[test]
    fn parse_minimal_toml() {
        let toml = "";
        let config: Config = toml::from_str(toml).unwrap();
        // All defaults should apply
        assert_eq!(config.general.threshold_ms, 300);
        assert!(config.general.enabled);
        assert_eq!(config.locale.active, "it");
    }

    #[test]
    fn parse_custom_threshold() {
        let toml = r#"
[general]
threshold_ms = 500
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.general.threshold_ms, 500);
        // Other fields should still be defaults
        assert!(config.general.enabled);
    }

    #[test]
    fn parse_disabled() {
        let toml = r#"
[general]
enabled = false
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(!config.general.enabled);
    }

    #[test]
    fn parse_locale_change() {
        let toml = r#"
[locale]
active = "fr"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.locale.active, "fr");
    }

    // --- spec: load_locale_map falls back to built-in ---

    #[test]
    fn load_locale_map_falls_back_to_builtin_italian() {
        let config = Config::default();
        let map = config.load_locale_map().unwrap();
        assert!(map.contains_key("e"));
        assert_eq!(map["e"][0], "è");
    }

    #[test]
    fn load_locale_map_fails_for_unknown_locale() {
        let mut config = Config::default();
        config.locale.active = "zz".into();
        assert!(config.load_locale_map().is_err());
    }

    // --- spec: socket path ---

    #[test]
    fn socket_path_ends_with_accentd_sock() {
        let path = socket_path();
        assert_eq!(path.file_name().unwrap(), "accentd.sock");
    }
}
