import { useCallback, useEffect, useState } from 'react';

import { printTestReceipt } from '@/shared/api/emulator';
import type { TranslationScope } from '@/shared/hooks/use-translation';
import { uiErrorKey } from '@/shared/utils/ui-error';

export interface ITestReceipt {
  print: () => void;
  sending: boolean;
  /** Shown for 5 s after a failure. */
  error: TranslationScope | null;
}

/** Sending the test receipt, shared by its buttons and its shortcut. */
export function useTestReceipt(): ITestReceipt {
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<TranslationScope | null>(null);

  useEffect(() => {
    if (!error) return;
    const timer = setTimeout(() => setError(null), 5000);
    return () => clearTimeout(timer);
  }, [error]);

  const print = useCallback(() => {
    setSending(true);
    setError(null);
    printTestReceipt()
      .catch((reason: unknown) => setError(uiErrorKey(reason)))
      .finally(() => setSending(false));
  }, []);

  return { print, sending, error };
}
