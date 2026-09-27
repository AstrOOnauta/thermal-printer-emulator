/**
 * Source of truth for i18n keys: every other locale is typed `: TranslationKeys`, so a
 * missing or extra key fails `tsc`. Native menu labels live in Rust (`locale.rs`).
 */
const en = {
  app: {
    name: 'Thermal Printer Emulator',
    tagline: 'Virtual thermal printer for ESC/POS',
  },
  jobs: {
    title: 'Received jobs',
    empty: 'Waiting for print jobs',
    emptyHint: 'Send ESC/POS jobs to port %{port} of this computer.',
    from: 'from %{host}',
    state: {
      receiving: 'Receiving…',
      done: 'Received',
      idle_timeout: 'Timed out',
      too_large: 'Too large',
      connection_error: 'Connection lost',
    },
  },
};

export type TranslationKeys = typeof en;

export default en;
