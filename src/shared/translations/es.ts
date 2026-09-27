import type { TranslationKeys } from './en';

const es: TranslationKeys = {
  app: {
    name: 'Thermal Printer Emulator',
    tagline: 'Impresora térmica virtual para ESC/POS',
  },
  jobs: {
    title: 'Trabajos recibidos',
    empty: 'Esperando trabajos de impresión',
    emptyHint: 'Envía trabajos ESC/POS al puerto %{port} de esta computadora.',
    from: 'desde %{host}',
    state: {
      receiving: 'Recibiendo…',
      done: 'Recibido',
      idle_timeout: 'Tiempo agotado',
      too_large: 'Demasiado grande',
      connection_error: 'Conexión perdida',
    },
  },
};

export default es;
