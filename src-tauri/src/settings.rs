//! User settings, a JSON file in the app's config folder. Rust owns it: the webview reads
//! and changes it only through commands, and every value is validated here.

use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::escpos::codepage::CodePage;
use crate::escpos::printer::Paper;
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub port: u16,
    pub bind: Bind,
    pub paper: Paper,
    /// `ESC t` table used until the POS selects one (decision 4).
    pub code_page: u8,
    /// Play the printer's beep (`ESC B`).
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

    /// The saved settings, or the defaults when the file is missing or unreadable (a
    /// broken file is logged and replaced on the next save, never fatal).
    pub fn load(path: &Path) -> Self {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Self::default(),
            Err(error) => {
                log::warn!("settings_unreadable path={} error={error}", path.display());
                return Self::default();
            }
        };
        match serde_json::from_str::<Self>(&text) {
            Ok(settings) if settings.validate().is_ok() => settings,
            Ok(_) | Err(_) => {
                log::warn!("settings_invalid path={}, using defaults", path.display());
                Self::default()
            }
        }
    }

    /// Writes a temporary file, then renames it over the old one, so a crash mid-write
    /// never leaves half a file.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let temporary = path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        std::fs::write(&temporary, json)?;
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
        std::fs::write(&path, r#"{ "port": 0 }"#).expect("writes");
        assert_eq!(Settings::load(&path), Settings::default());
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
