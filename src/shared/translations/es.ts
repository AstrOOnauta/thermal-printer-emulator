import type { TranslationKeys } from './en';

const es: TranslationKeys = {
  app: {
    name: 'Thermal Printer Emulator',
    tagline: 'Impresora térmica virtual para ESC/POS',
  },
  listener: {
    starting: 'Iniciando…',
    listening: 'Escuchando en el puerto %{port}',
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
    empty: 'Esperando recibos',
    emptyHint: 'Envía trabajos ESC/POS al puerto %{port} de esta computadora.',
    from: 'desde %{host}',
    drawer: 'Cajón abierto',
    beeps: 'Pitido ×%{count}',
    unavailable: 'Ya no está en memoria',
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
