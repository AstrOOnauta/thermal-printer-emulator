/** Mirrors `jobs::JobState` (snake_case on the wire). */
export type IJobState =
  'receiving' | 'done' | 'idle_timeout' | 'too_large' | 'connection_error';

/** Mirrors `jobs::JobSummary`. Change both together. */
export interface IJobSummary {
  id: number;
  /** `ip:port` of the client. */
  peer: string;
  /** Unix ms. */
  started_at: number;
  ended_at: number | null;
  state: IJobState;
  size: number;
}

/** Mirrors `listener::BindError`. */
export type IBindError = 'port_in_use' | 'permission_denied' | 'other';

/** Mirrors `listener::ListenerStatus` (tagged by `state`). */
export type IListenerStatus =
  | { state: 'starting' }
  | { state: 'listening'; port: number }
  | { state: 'failed'; port: number; error: IBindError };
