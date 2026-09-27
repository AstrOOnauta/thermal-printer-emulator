import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type {
  IJobSummary,
  IListenerStatus,
} from '@/shared/interfaces/emulator';

/** Received jobs, oldest first. */
export function getJobs(): Promise<IJobSummary[]> {
  return invoke<IJobSummary[]>('get_jobs');
}

/** The whole list again, whenever a job starts or ends. */
export function onJobs(
  handler: (jobs: IJobSummary[]) => void,
): Promise<UnlistenFn> {
  return listen<IJobSummary[]>('jobs', (event) => handler(event.payload));
}

/** Whether the emulator is listening, and on which port. */
export function getListenerStatus(): Promise<IListenerStatus> {
  return invoke<IListenerStatus>('get_listener_status');
}

/** Fires when the listener starts, fails to bind or recovers. */
export function onListenerStatus(
  handler: (status: IListenerStatus) => void,
): Promise<UnlistenFn> {
  return listen<IListenerStatus>('listener_status', (event) =>
    handler(event.payload),
  );
}
