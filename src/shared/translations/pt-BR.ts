import type { TranslationKeys } from './en';

const ptBR: TranslationKeys = {
  app: {
    name: 'Thermal Printer Emulator',
    tagline: 'Impressora térmica virtual para ESC/POS',
  },
  jobs: {
    title: 'Impressões recebidas',
    empty: 'Aguardando impressões',
    emptyHint:
      'Envie impressões ESC/POS para a porta %{port} deste computador.',
    from: 'de %{host}',
    state: {
      receiving: 'Recebendo…',
      done: 'Recebida',
      idle_timeout: 'Tempo esgotado',
      too_large: 'Grande demais',
      connection_error: 'Conexão perdida',
    },
  },
};

export default ptBR;
