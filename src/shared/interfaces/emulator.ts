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
