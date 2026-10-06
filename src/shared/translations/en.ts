/**
 * Source of truth for i18n keys: every other locale is typed `: TranslationKeys`, so a
 * missing or extra key fails `tsc`. Native menu labels live in Rust (`locale.rs`).
 */
const en = {
  nav: {
    settings: 'Settings',
  },
  settings: {
    title: 'Settings',
    close: 'Close settings',
    language: {
      label: 'Language',
      system: 'Same as the system',
    },
    port: {
      label: 'Port',
      hint: 'The TCP port the POS sends jobs to. Network printers use 9100.',
      apply: 'Apply',
    },
    bind: {
      label: 'Who can print',
      lan: 'Any device on the network',
      local: 'Only this computer',
    },
    paper: {
      label: 'Paper width',
      mm80: '80 mm (48 columns)',
      mm58: '58 mm (32 columns)',
      hint: 'Applies to new receipts.',
    },
    codePage: {
      label: 'Default code page',
      hint: 'Used until the POS selects one with ESC t.',
    },
    codePages: {
      cp437: 'CP437 · USA, standard Europe',
      cp850: 'CP850 · Multilingual Latin 1',
      cp860: 'CP860 · Portuguese',
      cp863: 'CP863 · Canadian French',
      cp865: 'CP865 · Nordic',
      wpc1252: 'WPC1252 · Windows Latin 1',
      cp866: 'CP866 · Cyrillic',
      cp852: 'CP852 · Latin 2',
      cp858: 'CP858 · Latin 1 with €',
    },
    sound: {
      label: 'Sound',
      beep: "Play the printer's beep",
    },
    errors: {
      port: 'Use a port from 1 to 65535.',
      codePage: 'This code page is not supported.',
      save: "Couldn't save the settings.",
    },
  },
  errors: {
    unexpected: 'Something went wrong. Try again.',
  },
  connection: {
    lan: 'Point your POS at',
    local: 'Only this computer can print, at',
    offline: 'No network found. This computer can print at',
    copy: 'Copy',
    copied: 'Copied',
  },
  testReceipt: {
    print: 'Print test receipt',
    errors: {
      notListening: 'The emulator is not listening yet.',
      send: "Couldn't send the test receipt.",
    },
  },
  listener: {
    starting: 'Starting…',
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
    newOne: 'New receipt',
    newMany: '%{count} new receipts',
    empty: 'Waiting for receipts',
    emptyHint: 'Send ESC/POS jobs to %{address}.',
    from: 'from %{host}',
    drawer: 'Drawer opened',
    beeps: 'Beep ×%{count}',
    unavailable: 'No longer in memory',
    clear: 'Clear receipts',
    clearTitle: 'Clear all receipts?',
    clearBody:
      "They are removed from the list and from memory. This can't be undone.",
    clearConfirm: 'Clear',
    cancel: 'Cancel',
    export: 'Save .bin',
    exported: 'Saved to Downloads',
    errors: {
      gone: 'This receipt is no longer in memory.',
      export: "Couldn't save the file.",
    },
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
