import { useEffect, useRef, useState } from 'react';

import { setUnseen } from '@/shared/api/app';
import type { IReceiptSummary } from '@/shared/interfaces/emulator';
import { pendingSounds } from '@/shared/utils/sounds';

/**
 * Counts receipts that finish while the window is not in front and shows the count on the
 * Dock icon / menu bar / tray (`set_unseen`); it clears when the window gets focus.
 */
export function useUnseenBadge(receipts: IReceiptSummary[] | null) {
  const heardRef = useRef<Set<number> | null>(null);
  const [count, setCount] = useState(0);

  useEffect(() => {
    if (receipts === null) return;
    // Same "finished since last time" rule as the sounds.
    const { printed, seen } = pendingSounds(heardRef.current, receipts);
    heardRef.current = seen;
    if (printed.length > 0 && !document.hasFocus()) {
      setCount((previous) => previous + printed.length);
    }
  }, [receipts]);

  useEffect(() => {
    const clear = () => setCount(0);
    window.addEventListener('focus', clear);
    return () => window.removeEventListener('focus', clear);
  }, []);

  useEffect(() => {
    setUnseen(count).catch(() => {
      // Not running inside Tauri.
    });
  }, [count]);
}
