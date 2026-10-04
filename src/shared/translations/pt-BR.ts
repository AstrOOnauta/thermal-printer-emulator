import type { TranslationKeys } from './en';

const ptBR: TranslationKeys = {
  app: {
    name: 'Thermal Printer Emulator',
    tagline: 'Impressora térmica virtual para ESC/POS',
  },
  listener: {
    starting: 'Iniciando…',
    listening: 'Escutando na porta %{port}',
    failed: {
      port_in_use: 'A porta %{port} está em uso',
      permission_denied: 'A porta %{port} está bloqueada',
      other: 'Não foi possível abrir a porta %{port}',
    },
    hint: {
      port_in_use:
        'Outro app está usando esta porta. Feche-o e o emulador inicia sozinho.',
      permission_denied:
        'O sistema não permite esta porta. O emulador continua tentando.',
      other: 'O emulador continua tentando.',
    },
  },
  receipts: {
    title: 'Cupons',
    empty: 'Aguardando cupons',
    emptyHint:
      'Envie impressões ESC/POS para a porta %{port} deste computador.',
    from: 'de %{host}',
    state: {
      printing: 'Imprimindo…',
      done: 'Impresso',
      idle_timeout: 'Tempo esgotado',
      too_large: 'Grande demais',
      connection_error: 'Conexão perdida',
    },
  },
};

export default ptBR;
