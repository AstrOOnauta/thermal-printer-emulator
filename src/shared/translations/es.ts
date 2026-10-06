import type { TranslationKeys } from './en';

const es: TranslationKeys = {
  nav: {
    settings: 'Configuración',
  },
  settings: {
    title: 'Configuración',
    close: 'Cerrar configuración',
    language: {
      label: 'Idioma',
      system: 'Igual que el sistema',
    },
    port: {
      label: 'Puerto',
      hint: 'El puerto TCP al que el POS envía los trabajos. Las impresoras de red usan el 9100.',
      apply: 'Aplicar',
    },
    bind: {
      label: 'Quién puede imprimir',
      lan: 'Cualquier dispositivo de la red',
      local: 'Solo esta computadora',
    },
    paper: {
      label: 'Ancho del papel',
      mm80: '80 mm (48 columnas)',
      mm58: '58 mm (32 columnas)',
      hint: 'Se aplica a los recibos nuevos.',
    },
    codePage: {
      label: 'Página de códigos predeterminada',
      hint: 'Se usa hasta que el POS elige otra con ESC t.',
    },
    codePages: {
      cp437: 'CP437 · EE. UU., Europa estándar',
      cp850: 'CP850 · Latín 1 multilingüe',
      cp860: 'CP860 · Portugués',
      cp863: 'CP863 · Francés canadiense',
      cp865: 'CP865 · Nórdico',
      wpc1252: 'WPC1252 · Windows Latín 1',
      cp866: 'CP866 · Cirílico',
      cp852: 'CP852 · Latín 2',
      cp858: 'CP858 · Latín 1 con €',
    },
    sound: {
      label: 'Sonido',
      printing: 'Sonido de impresión y pitido',
    },
    errors: {
      port: 'Usa un puerto del 1 al 65535.',
      codePage: 'Esta página de códigos no es compatible.',
      save: 'No se pudo guardar la configuración.',
    },
  },
  errors: {
    unexpected: 'Algo salió mal. Inténtalo de nuevo.',
  },
  connection: {
    lan: 'Configura tu POS para',
    local: 'Solo esta computadora puede imprimir, en',
    offline: 'No se encontró ninguna red. Esta computadora puede imprimir en',
    copy: 'Copiar',
    copied: 'Copiado',
  },
  testReceipt: {
    print: 'Imprimir recibo de prueba',
    errors: {
      notListening: 'El emulador todavía no está escuchando.',
      send: 'No se pudo enviar el recibo de prueba.',
    },
  },
  listener: {
    starting: 'Iniciando…',
    failed: {
      port_in_use: 'El puerto %{port} está en uso',
      permission_denied: 'El puerto %{port} está bloqueado',
      other: 'No se puede abrir el puerto %{port}',
    },
    hint: {
      port_in_use:
        'Otra aplicación usa este puerto. Ciérrala y el emulador se inicia solo.',
      permission_denied:
        'El sistema no permite este puerto. El emulador sigue intentándolo.',
      other: 'El emulador sigue intentándolo.',
    },
  },
  receipts: {
    title: 'Recibos',
    newOne: 'Nuevo recibo',
    newMany: '%{count} recibos nuevos',
    empty: 'Esperando recibos',
    emptyHint: 'Envía trabajos ESC/POS a %{address}.',
    from: 'desde %{host}',
    drawer: 'Cajón abierto',
    beeps: 'Pitido ×%{count}',
    unavailable: 'Ya no está en memoria',
    clear: 'Limpiar recibos',
    clearTitle: '¿Limpiar todos los recibos?',
    clearBody: 'Se quitan de la lista y de la memoria. No se puede deshacer.',
    clearConfirm: 'Limpiar',
    cancel: 'Cancelar',
    export: 'Guardar .bin',
    exported: 'Guardado en Descargas',
    errors: {
      gone: 'Este recibo ya no está en memoria.',
      export: 'No se pudo guardar el archivo.',
    },
    state: {
      printing: 'Imprimiendo…',
      done: 'Impreso',
      idle_timeout: 'Tiempo agotado',
      too_large: 'Demasiado grande',
      connection_error: 'Conexión perdida',
    },
  },
};

export default es;
