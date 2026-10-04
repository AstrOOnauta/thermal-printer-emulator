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
}

/** Mirrors `listener::BindError`. */
export type IBindError = 'port_in_use' | 'permission_denied' | 'other';

/** Mirrors `listener::ListenerStatus` (tagged by `state`). */
export type IListenerStatus =
  | { state: 'starting' }
  | { state: 'listening'; port: number }
  | { state: 'failed'; port: number; error: IBindError };
