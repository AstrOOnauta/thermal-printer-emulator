import type { TranslationKeys } from './en';

const ptBR: TranslationKeys = {
  nav: {
    settings: 'Configurações',
  },
  settings: {
    title: 'Configurações',
    close: 'Fechar configurações',
    language: {
      label: 'Idioma',
      system: 'Igual ao sistema',
    },
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
    zoom: {
      label: 'Zoom do papel',
      hint: '%{in} para aproximar, %{out} para afastar, %{reset} para 100%.',
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
      printing: 'Som de impressão e bipe',
    },
    errors: {
      port: 'Use uma porta de 1 a 65535.',
      codePage: 'Esta página de código não é suportada.',
      zoom: 'Escolha um dos níveis de zoom da lista.',
      save: 'Não foi possível salvar as configurações.',
    },
  },
  errors: {
    unexpected: 'Algo deu errado. Tente de novo.',
  },
  connection: {
    lan: 'Configure seu PDV para',
    local: 'Só este computador pode imprimir, em',
    offline: 'Nenhuma rede encontrada. Este computador pode imprimir em',
    copy: 'Copiar',
    copied: 'Copiado',
  },
  testReceipt: {
    print: 'Imprimir cupom de teste',
    errors: {
      notListening: 'O emulador ainda não está escutando.',
      send: 'Não foi possível enviar o cupom de teste.',
    },
  },
  listener: {
    starting: 'Iniciando…',
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
    newOne: 'Novo cupom',
    newMany: '%{count} cupons novos',
    empty: 'Aguardando cupons',
    emptyHint: 'Envie impressões ESC/POS para %{address}.',
    from: 'de %{host}',
    drawer: 'Gaveta aberta',
    beeps: 'Bipe ×%{count}',
    unavailable: 'Não está mais na memória',
    clear: 'Limpar cupons',
    clearTitle: 'Limpar todos os cupons?',
    clearBody: 'Eles saem da lista e da memória. Não dá para desfazer.',
    clearConfirm: 'Limpar',
    cancel: 'Cancelar',
    export: 'Salvar .bin',
    exported: 'Salvo em Downloads',
    errors: {
      gone: 'Este cupom não está mais na memória.',
      export: 'Não foi possível salvar o arquivo.',
    },
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
