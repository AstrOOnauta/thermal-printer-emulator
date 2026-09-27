import { useEffect, useState } from 'react';

/**
 * Rust-owned state mirrored in the webview: `null` until the first answer. Subscribes
 * **before** reading, so a change in between is not lost; once an event has arrived, the
 * (possibly older) read is ignored. Outside Tauri (`yarn dev` in a browser) both calls
 * fail and `fallback` is used.
 *
 * All three arguments must be stable: functions from `shared/api/` and a module-level
 * constant, or the effect resubscribes on every render.
 */
export function useSynced<T>(
  subscribe: (handler: (value: T) => void) => Promise<() => void>,
  read: () => Promise<T>,
  fallback: T,
): T | null {
  const [value, setValue] = useState<T | null>(null);

  useEffect(() => {
    let active = true;
    let gotEvent = false;
    let unlisten: (() => void) | undefined;

    void (async () => {
      try {
        const stop = await subscribe((next) => {
          gotEvent = true;
          if (active) setValue(next);
        });
        if (!active) return stop();
        unlisten = stop;
        const initial = await read();
        if (active && !gotEvent) setValue(initial);
      } catch {
        if (active && !gotEvent) setValue(fallback);
      }
    })();

    return () => {
      active = false;
      unlisten?.();
    };
  }, [subscribe, read, fallback]);

  return value;
}
