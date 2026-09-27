/**
 * Source of truth for i18n keys: every other locale is typed `: TranslationKeys`, so a
 * missing or extra key fails `tsc`. Native menu labels live in Rust (`locale.rs`).
 */
const en = {
  app: {
    name: 'Thermal Printer Emulator',
    tagline: 'Virtual thermal printer for ESC/POS',
  },
};

export type TranslationKeys = typeof en;

export default en;
