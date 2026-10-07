/** Mirrors `receipts::ReceiptState` (snake_case on the wire). */
export type IReceiptState =
  'printing' | 'done' | 'idle_timeout' | 'too_large' | 'connection_error';

/** Mirrors `receipts::Cut`. */
export type ICut = 'full' | 'partial';

/** Mirrors `receipts::ReceiptSummary`. Change both together. */
export interface IReceiptSummary {
  id: number;
  /** `ip:port` of the client. */
  peer: string;
  /** Unix ms. */
  started_at: number;
  ended_at: number | null;
  state: IReceiptState;
  cut: ICut | null;
  /** The cash drawer was opened. */
  drawer: boolean;
  beeps: number;
  /** Raw bytes received. */
  size: number;
  paper: IPaper;
  /** Printable width in dots. */
  width: number;
  /** Paper length so far, in dots. */
  height: number;
}

/** Mirrors `printer::Font`. */
export type IFont = 'a' | 'b';

/** Mirrors `printer::Segment`: character `i` sits at `x + i × advance` (dots). */
export interface ISegment {
  x: number;
  text: string;
  font: IFont;
  /** Size multipliers, 1–8. */
  width: number;
  height: number;
  advance: number;
  bold: boolean;
  underline: 0 | 1 | 2;
  reverse: boolean;
}

/** Mirrors `printer::Placed`: a 1-bit bitmap, rows MSB-first, 1 = black, base64. */
export interface IPlaced {
  x: number;
  width: number;
  height: number;
  data: string;
}

/** Mirrors `printer::Block` (tagged by `type`). */
export type IBlock =
  | {
      type: 'line';
      height: number;
      ascent: number;
      segments: ISegment[];
      images?: IPlaced[];
    }
  | ({ type: 'image' } & IPlaced)
  | { type: 'feed'; height: number };

/** Mirrors `printer::Paper`. */
export type IPaper = 'mm80' | 'mm58';

/** Mirrors `receipts::ReceiptView`: a receipt with what to draw. */
export interface IReceiptView extends IReceiptSummary {
  blocks: IBlock[];
}

/** Mirrors `listener::BindError`. */
export type IBindError = 'port_in_use' | 'permission_denied' | 'other';

/** Mirrors `listener::ListenerStatus` (tagged by `state`). */
export type IListenerStatus =
  | { state: 'starting' }
  | { state: 'listening'; port: number }
  | { state: 'failed'; port: number; error: IBindError };

/** Mirrors `settings::Bind`. */
export type IBind = 'lan' | 'local';

/** Mirrors `locale::Language`: `system` follows the OS. */
export type ILanguage = 'system' | 'en' | 'es' | 'pt-BR';

/** Mirrors `settings::Settings`. */
export interface ISettings {
  port: number;
  bind: IBind;
  paper: IPaper;
  /** `ESC t` table used until the POS selects one. */
  code_page: number;
  sound: boolean;
  language: ILanguage;
  /** Paper zoom in percent: 75, 100, 125, 150 or 200. */
  zoom: number;
}

/** A refused command (`ui::UiError`): an i18n key. */
export interface IUiError {
  key: string;
}

/** Mirrors `inspect::CommandRow`: one command of a receipt's raw bytes. */
export interface ICommandRow {
  offset: number;
  length: number;
  /** Hex of the first bytes, `…` when cut. */
  bytes: string;
  /** `ESC a`, `GS V`, `LF`; empty for text. */
  mnemonic: string;
  /** An i18n key under `inspect.kinds`. */
  kind: string;
  /** Parameters in the ESC/POS reference's notation, or the text. */
  detail: string;
}

/** Mirrors `inspect::Inspection`. */
export interface IInspection {
  rows: ICommandRow[];
  truncated: boolean;
}
