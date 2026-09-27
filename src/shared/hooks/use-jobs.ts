import { useEffect, useState } from 'react';

import { getJobs, onJobs } from '@/shared/api/emulator';
import type { IJobSummary } from '@/shared/interfaces/emulator';

/**
 * The received jobs, oldest first; `null` until the first answer. Subscribes **before**
 * reading, so a job that starts in between is not lost; once an event has arrived, the
 * (possibly older) read is ignored.
 */
export function useJobs(): IJobSummary[] | null {
  const [jobs, setJobs] = useState<IJobSummary[] | null>(null);

  useEffect(() => {
    let active = true;
    let gotEvent = false;
    let unlisten: (() => void) | undefined;

    void (async () => {
      try {
        const stop = await onJobs((next) => {
          gotEvent = true;
          if (active) setJobs(next);
        });
        if (!active) return stop();
        unlisten = stop;
        const initial = await getJobs();
        if (active && !gotEvent) setJobs(initial);
      } catch {
        // Not running inside Tauri (`yarn dev` in a browser).
        if (active && !gotEvent) setJobs([]);
      }
    })();

    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  return jobs;
}
