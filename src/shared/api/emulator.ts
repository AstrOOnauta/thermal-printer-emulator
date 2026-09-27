import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type { IJobSummary } from '@/shared/interfaces/emulator';

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
