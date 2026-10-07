//! User settings, a JSON file in the app's config folder. Rust owns it: the webview reads
//! and changes it only through commands, and every value is validated here.

use std::io::{self, Write};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::escpos::codepage::CodePage;
use crate::escpos::model::Paper;
use crate::locale::Language;

pub const FILE_NAME: &str = "settings.json";

/// Paper zoom steps, in percent.
pub const ZOOM_STEPS: [u16; 5] = [75, 100, 125, 150, 200];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Bind {
    /// Every interface: POS terminals on other machines can print (decision 2).
    Lan,
    /// 127.0.0.1 only: this computer.
    Local,
}

/// No `#[serde(default)]`: a change from the webview must carry every field (a missing one
/// would silently fall back, `bind` to `lan` included). The file fills gaps in `load`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub port: u16,
    pub bind: Bind,
    pub paper: Paper,
    /// `ESC t` table used until the POS selects one (decision 4).
    pub code_page: u8,
    /// Play the printing sound for each receipt and the printer's beep (`ESC B`).
    pub sound: bool,
    pub language: Language,
    /// Paper zoom in percent, one of `ZOOM_STEPS`.
    pub zoom: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            port: 9100,
            bind: Bind::Lan,
            paper: Paper::Mm80,
            code_page: 0,
            sound: true,
            language: Language::System,
            zoom: 100,
        }
    }
}

/// Why a settings change was refused: an i18n key under `settings.errors`.
#[derive(Debug, PartialEq, Eq)]
pub enum Invalid {
    Port,
    CodePage,
    Zoom,
}

impl Invalid {
    pub fn key(&self) -> &'static str {
        match self {
            Self::Port => "settings.errors.port",
            Self::CodePage => "settings.errors.codePage",
            Self::Zoom => "settings.errors.zoom",
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<(), Invalid> {
        if self.port == 0 {
            return Err(Invalid::Port);
        }
        if CodePage::from_table(self.code_page).is_none() {
            return Err(Invalid::CodePage);
        }
        if !ZOOM_STEPS.contains(&self.zoom) {
            return Err(Invalid::Zoom);
        }
        Ok(())
    }

    pub fn addr(&self) -> SocketAddr {
        let ip = match self.bind {
            Bind::Lan => Ipv4Addr::UNSPECIFIED,
            Bind::Local => Ipv4Addr::LOCALHOST,
        };
        SocketAddr::V4(SocketAddrV4::new(ip, self.port))
    }

    pub fn code_page(&self) -> CodePage {
        CodePage::from_table(self.code_page).unwrap_or(CodePage::DEFAULT)
    }

    /// The saved settings. Field by field: a missing or invalid field takes its default and
    /// the others are kept, so one bad value (a hand edit, a newer version's option) never
    /// resets the rest, `bind: local` included. Never fatal.
    pub fn load(path: &Path) -> Self {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => Some(text),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Self::default(),
            // Not UTF-8: broken, like bad JSON below.
            Err(error) if error.kind() == io::ErrorKind::InvalidData => None,
            Err(error) => {
                log::warn!("settings_unreadable path={} error={error}", path.display());
                return Self::default();
            }
        };
        let parsed = text.and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
        let saved = match parsed {
            Some(serde_json::Value::Object(saved)) => saved,
            _ => {
                // Kept for a look: the next save replaces the file.
                let _ = std::fs::copy(path, path.with_extension("json.bad"));
                log::warn!("settings_broken path={}, using defaults", path.display());
                return Self::default();
            }
        };
        let mut merged = serde_json::to_value(Self::default()).expect("settings serialize");
        for (key, value) in saved {
            let Some(slot) = merged.get_mut(&key) else {
                continue; // Unknown: an option from a newer version.
            };
            let previous = std::mem::replace(slot, value);
            let valid = serde_json::from_value::<Self>(merged.clone())
                .is_ok_and(|settings| settings.validate().is_ok());
            if !valid {
                log::warn!("settings_field_invalid key={key}, using its default");
                merged[&key] = previous;
            }
        }
        serde_json::from_value(merged).unwrap_or_default()
    }

    /// Writes a temporary file, then renames it over the old one, so a crash mid-write
    /// never leaves half a file.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let temporary = path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(json.as_bytes())?;
        // On disk before the rename, or a power loss could leave an empty file behind it.
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("tpe-settings-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join(FILE_NAME)
    }

    #[test]
    fn missing_file_gives_defaults() {
        let settings = Settings::load(&scratch("missing"));
        assert_eq!(settings, Settings::default());
        assert_eq!(settings.addr().to_string(), "0.0.0.0:9100");
    }

    #[test]
    fn saves_and_loads() {
        let path = scratch("round-trip");
        let settings = Settings {
            port: 9101,
            bind: Bind::Local,
            paper: Paper::Mm58,
            code_page: 2,
            sound: false,
            language: Language::PtBr,
            zoom: 150,
        };
        settings.save(&path).expect("saves");
        assert_eq!(Settings::load(&path), settings);
        assert_eq!(settings.addr().to_string(), "127.0.0.1:9101");
        assert_eq!(settings.code_page(), CodePage::Cp850);
        assert!(
            !path.with_extension("json.tmp").exists(),
            "temporary file renamed"
        );
    }

    #[test]
    fn broken_or_invalid_files_give_defaults() {
        let path = scratch("broken");
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("creates dir");
        std::fs::write(&path, "{ not json").expect("writes");
        assert_eq!(Settings::load(&path), Settings::default());
        assert!(path.with_extension("json.bad").exists(), "kept for a look");
        std::fs::write(&path, r#"{ "port": 0 }"#).expect("writes");
        assert_eq!(Settings::load(&path), Settings::default());
    }

    #[test]
    fn a_bad_field_keeps_the_others() {
        let path = scratch("bad-field");
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("creates dir");
        std::fs::write(
            &path,
            r#"{ "port": 0, "bind": "local", "paper": "mm58", "zoom": 110,
                 "language": "fr", "future_option": true }"#,
        )
        .expect("writes");
        let settings = Settings::load(&path);
        assert_eq!(
            (settings.bind, settings.paper),
            (Bind::Local, Paper::Mm58),
            "valid fields kept: never back to the LAN by accident"
        );
        assert_eq!(
            (settings.port, settings.zoom, settings.language),
            (9100, 100, Language::System),
            "invalid ones take their default"
        );
    }

    #[test]
    fn a_change_must_carry_every_field() {
        let partial = serde_json::from_str::<Settings>(r#"{ "port": 9100, "bind": "local" }"#);
        assert!(partial.is_err());
    }

    #[test]
    fn missing_fields_take_their_default() {
        let path = scratch("partial");
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("creates dir");
        std::fs::write(&path, r#"{ "port": 9200 }"#).expect("writes");
        let settings = Settings::load(&path);
        assert_eq!(settings.port, 9200);
        assert_eq!(settings.paper, Paper::Mm80);
    }

    #[test]
    fn validation() {
        let valid = Settings::default();
        assert_eq!(valid.validate(), Ok(()));
        assert_eq!(
            Settings {
                port: 0,
                ..valid.clone()
            }
            .validate(),
            Err(Invalid::Port)
        );
        assert_eq!(
            Settings {
                zoom: 110,
                ..valid.clone()
            }
            .validate(),
            Err(Invalid::Zoom),
            "zoom moves in steps"
        );
        assert_eq!(
            Settings {
                code_page: 1,
                ..valid
            }
            .validate(),
            Err(Invalid::CodePage),
            "Katakana is not supported"
        );
    }
}
