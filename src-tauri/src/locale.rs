//! UI language: the user's choice (`Language`, a setting), or the OS locale for `System`.
//! Rust is the single source: the native tray/menu read it here and the webview asks for it
//! (`app_locale`), so both always agree. WKWebView's `navigator.language` does not reliably
//! follow the system language.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// The language setting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    /// Follow the OS.
    #[default]
    #[serde(rename = "system")]
    System,
    #[serde(rename = "en")]
    En,
    #[serde(rename = "es")]
    Es,
    #[serde(rename = "pt-BR")]
    PtBr,
}

/// The chosen `Language` as 0 (system) to 3. A process-wide value read by every menu.
static PREFERENCE: AtomicU8 = AtomicU8::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locale {
    En,
    Es,
    PtBr,
}

impl Locale {
    /// Maps a BCP 47 / POSIX tag (`pt-BR`, `es_MX.UTF-8`, `C`) to a supported locale.
    /// Unsupported languages fall back to English.
    pub fn from_tag(tag: &str) -> Self {
        let language = tag
            .split(['-', '_', '.', '@'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        match language.as_str() {
            "pt" => Self::PtBr,
            "es" => Self::Es,
            _ => Self::En,
        }
    }

    /// The OS language, read once per process.
    pub fn system() -> Self {
        static SYSTEM: OnceLock<Locale> = OnceLock::new();
        *SYSTEM.get_or_init(|| {
            sys_locale::get_locale()
                .map(|tag| Self::from_tag(&tag))
                .unwrap_or(Self::En)
        })
    }

    /// The language the UI uses now: the setting, or the OS language for `System`.
    pub fn current() -> Self {
        match PREFERENCE.load(Ordering::Relaxed) {
            1 => Self::En,
            2 => Self::Es,
            3 => Self::PtBr,
            _ => Self::system(),
        }
    }

    /// Applies the language setting. Native menus must be rebuilt afterwards.
    pub fn prefer(language: Language) {
        let value = match language {
            Language::System => 0,
            Language::En => 1,
            Language::Es => 2,
            Language::PtBr => 3,
        };
        PREFERENCE.store(value, Ordering::Relaxed);
    }

    /// The tag the webview's translation files are keyed by.
    pub fn tag(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Es => "es",
            Self::PtBr => "pt-BR",
        }
    }

    /// Labels of the native menus, which the webview cannot translate.
    pub fn strings(self) -> &'static Strings {
        match self {
            Self::En => &EN,
            Self::Es => &ES,
            Self::PtBr => &PT_BR,
        }
    }
}

pub struct Strings {
    pub open: &'static str,
    pub autostart: &'static str,
    pub logs: &'static str,
    pub quit: &'static str,
    /// The macOS app menu. Windows and Linux have none, so its labels exist only on macOS.
    #[cfg(target_os = "macos")]
    pub menu: MenuStrings,
    /// Listener status line. `{port}` is replaced by the port number.
    pub starting: &'static str,
    pub listening: &'static str,
    pub port_in_use: &'static str,
    pub port_denied: &'static str,
    pub port_failed: &'static str,
    /// Tray item and the test receipt's own text.
    pub print_test: &'static str,
    /// Receipts that arrived while the window was not in front. `{count}` is replaced.
    pub unseen: &'static str,
    /// Tray item for a found update. `{version}` is replaced. `update_install` when the app
    /// can update itself, `update_download` for a `.deb` or `.rpm` (opens the release page).
    pub update_install: &'static str,
    pub update_download: &'static str,
    pub test_title: &'static str,
    pub test_working: &'static str,
    pub test_port: &'static str,
    pub test_paper: &'static str,
    pub test_code_page: &'static str,
    pub test_bold: &'static str,
    pub test_underline: &'static str,
    pub test_reverse: &'static str,
    pub test_font_b: &'static str,
}

/// The macOS app menu's labels.
#[cfg(target_os = "macos")]
pub struct MenuStrings {
    pub close_window: &'static str,
    pub close: &'static str,
    /// The About panel's item and its credits line (`{author}` is replaced).
    pub about: &'static str,
    pub made_by: &'static str,
    pub edit: &'static str,
    pub undo: &'static str,
    pub redo: &'static str,
    pub cut: &'static str,
    pub copy: &'static str,
    pub paste: &'static str,
    pub select_all: &'static str,
}

const EN: Strings = Strings {
    open: "Open",
    autostart: "Launch at login",
    logs: "Show logs",
    quit: "Quit",
    #[cfg(target_os = "macos")]
    menu: MenuStrings {
        close_window: "Close window",
        close: "Close",
        about: "About Thermal Printer Emulator",
        made_by: "Made by {author}",
        edit: "Edit",
        undo: "Undo",
        redo: "Redo",
        cut: "Cut",
        copy: "Copy",
        paste: "Paste",
        select_all: "Select all",
    },
    starting: "Starting…",
    listening: "Listening on port {port}",
    port_in_use: "Port {port} is in use",
    port_denied: "Port {port} is blocked",
    port_failed: "Can't open port {port}",
    print_test: "Print test receipt",
    unseen: "{count} new",
    update_install: "Restart to update to {version}",
    update_download: "Download version {version}",
    test_title: "TEST RECEIPT",
    test_working: "The emulator is working.",
    test_port: "Port:",
    test_paper: "Paper:",
    test_code_page: "Code page:",
    test_bold: "Bold",
    test_underline: "Underline",
    test_reverse: "Reverse",
    test_font_b: "Font B: smaller, more columns per line.",
};

const ES: Strings = Strings {
    open: "Abrir",
    autostart: "Iniciar con el sistema",
    logs: "Ver registros",
    quit: "Salir",
    #[cfg(target_os = "macos")]
    menu: MenuStrings {
        close_window: "Cerrar ventana",
        close: "Cerrar",
        about: "Acerca de Thermal Printer Emulator",
        made_by: "Hecho por {author}",
        edit: "Editar",
        undo: "Deshacer",
        redo: "Rehacer",
        cut: "Cortar",
        copy: "Copiar",
        paste: "Pegar",
        select_all: "Seleccionar todo",
    },
    starting: "Iniciando…",
    listening: "Escuchando en el puerto {port}",
    port_in_use: "El puerto {port} está en uso",
    port_denied: "El puerto {port} está bloqueado",
    port_failed: "No se puede abrir el puerto {port}",
    print_test: "Imprimir recibo de prueba",
    unseen: "{count} nuevos",
    update_install: "Reiniciar y actualizar a la {version}",
    update_download: "Descargar la versión {version}",
    test_title: "RECIBO DE PRUEBA",
    test_working: "El emulador funciona.",
    test_port: "Puerto:",
    test_paper: "Papel:",
    test_code_page: "Página de códigos:",
    test_bold: "Negrita",
    test_underline: "Subrayado",
    test_reverse: "Inverso",
    test_font_b: "Fuente B: más pequeña, más columnas por línea.",
};

const PT_BR: Strings = Strings {
    open: "Abrir",
    autostart: "Iniciar com o sistema",
    logs: "Ver registros",
    quit: "Sair",
    #[cfg(target_os = "macos")]
    menu: MenuStrings {
        close_window: "Fechar janela",
        close: "Fechar",
        about: "Sobre o Thermal Printer Emulator",
        made_by: "Feito por {author}",
        edit: "Editar",
        undo: "Desfazer",
        redo: "Refazer",
        cut: "Recortar",
        copy: "Copiar",
        paste: "Colar",
        select_all: "Selecionar tudo",
    },
    starting: "Iniciando…",
    listening: "Escutando na porta {port}",
    port_in_use: "A porta {port} está em uso",
    port_denied: "A porta {port} está bloqueada",
    port_failed: "Não foi possível abrir a porta {port}",
    print_test: "Imprimir cupom de teste",
    unseen: "{count} novos",
    update_install: "Reiniciar e atualizar para a {version}",
    update_download: "Baixar a versão {version}",
    test_title: "CUPOM DE TESTE",
    test_working: "O emulador está funcionando.",
    test_port: "Porta:",
    test_paper: "Papel:",
    test_code_page: "Página de código:",
    test_bold: "Negrito",
    test_underline: "Sublinhado",
    test_reverse: "Reverso",
    test_font_b: "Fonte B: menor, mais colunas por linha.",
};

#[cfg(test)]
mod tests {
    use super::{Language, Locale};

    #[test]
    fn resolves_os_tags() {
        for (tag, expected) in [
            ("pt-BR", Locale::PtBr),
            ("pt_PT.UTF-8", Locale::PtBr),
            ("es", Locale::Es),
            ("es-419", Locale::Es),
            ("ES_mx", Locale::Es),
            ("en-US", Locale::En),
            ("fr-FR", Locale::En),
            ("C", Locale::En),
            ("", Locale::En),
        ] {
            assert_eq!(Locale::from_tag(tag), expected, "tag {tag:?}");
        }
    }

    #[test]
    fn the_setting_overrides_the_system_language() {
        Locale::prefer(Language::Es);
        assert_eq!(Locale::current(), Locale::Es);
        Locale::prefer(Language::PtBr);
        assert_eq!(Locale::current().tag(), "pt-BR");
        Locale::prefer(Language::System);
        assert_eq!(Locale::current(), Locale::system());
    }

    #[test]
    fn language_uses_the_webview_tags_on_the_wire() {
        let json = serde_json::to_string(&[Language::System, Language::PtBr]).expect("serializes");
        assert_eq!(json, r#"["system","pt-BR"]"#);
    }
}
