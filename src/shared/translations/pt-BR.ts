import type { TranslationKeys } from './en';

const ptBR: TranslationKeys = {
  app: {
    name: 'Thermal Printer Emulator',
    tagline: 'Impressora térmica virtual para ESC/POS',
  },
  nav: {
    settings: 'Configurações',
    receipts: 'Cupons',
  },
  settings: {
    title: 'Configurações',
    port: {
      label: 'Porta',
      hint: 'A porta TCP para onde o PDV envia as impressões. Impressoras de rede usam a 9100.',
      apply: 'Aplicar',
    },
    bind: {
      label: 'Quem pode imprimir',
      lan: 'Qualquer dispositivo da rede',
      local: 'Só este computador',
    },
    paper: {
      label: 'Largura do papel',
      mm80: '80 mm (48 colunas)',
      mm58: '58 mm (32 colunas)',
      hint: 'Vale para os próximos cupons.',
    },
    codePage: {
      label: 'Página de código padrão',
      hint: 'Usada até o PDV escolher outra com ESC t.',
    },
    codePages: {
      cp437: 'CP437 · EUA, Europa padrão',
      cp850: 'CP850 · Latin 1 multilíngue',
      cp860: 'CP860 · Português',
      cp863: 'CP863 · Francês canadense',
      cp865: 'CP865 · Nórdico',
      wpc1252: 'WPC1252 · Windows Latin 1',
      cp866: 'CP866 · Cirílico',
      cp852: 'CP852 · Latin 2',
      cp858: 'CP858 · Latin 1 com €',
    },
    sound: {
      label: 'Som',
      beep: 'Tocar o bipe da impressora',
    },
    errors: {
      port: 'Use uma porta de 1 a 65535.',
      codePage: 'Esta página de código não é suportada.',
      save: 'Não foi possível salvar as configurações.',
    },
  },
  errors: {
    unexpected: 'Algo deu errado. Tente de novo.',
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
    drawer: 'Gaveta aberta',
    beeps: 'Bipe ×%{count}',
    unavailable: 'Não está mais na memória',
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
