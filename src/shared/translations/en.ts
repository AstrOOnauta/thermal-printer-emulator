/**
 * Source of truth for i18n keys: every other locale is typed `: TranslationKeys`, so a
 * missing or extra key fails `tsc`. Native menu labels live in Rust (`locale.rs`).
 */
const en = {
  app: {
    name: 'Thermal Printer Emulator',
    tagline: 'Virtual thermal printer for ESC/POS',
  },
  listener: {
    starting: 'Starting…',
    listening: 'Listening on port %{port}',
    failed: {
      port_in_use: 'Port %{port} is in use',
      permission_denied: 'Port %{port} is blocked',
      other: "Can't open port %{port}",
    },
    hint: {
      port_in_use:
        'Another app is using this port. Close it and the emulator starts on its own.',
      permission_denied:
        "The system doesn't allow this port. The emulator keeps trying.",
      other: 'The emulator keeps trying.',
    },
  },
  receipts: {
    title: 'Receipts',
    empty: 'Waiting for receipts',
    emptyHint: 'Send ESC/POS jobs to port %{port} of this computer.',
    from: 'from %{host}',
    drawer: 'Drawer opened',
    beeps: 'Beep ×%{count}',
    unavailable: 'No longer in memory',
    state: {
      printing: 'Printing…',
      done: 'Printed',
      idle_timeout: 'Timed out',
      too_large: 'Too large',
      connection_error: 'Connection lost',
    },
  },
};

export type TranslationKeys = typeof en;

export default en;
