import { useEffect, useState } from 'react';

import { printTestReceipt } from '@/shared/api/emulator';
import {
  type TranslationScope,
  useTranslation,
} from '@/shared/hooks/use-translation';
import { BUTTON } from '@/shared/styles/patterns';
import { uiErrorKey } from '@/shared/utils/ui-error';

/** Sends the test receipt; the receipt itself shows up in the list like any other. */
export function TestReceiptButton() {
  const { t } = useTranslation();
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<TranslationScope | null>(null);

  useEffect(() => {
    if (!error) return;
    const timer = setTimeout(() => setError(null), 5000);
    return () => clearTimeout(timer);
  }, [error]);

  return (
    <span className="flex items-center gap-2">
      {error && (
        <span role="alert" className="text-xs text-error">
          {t(error)}
        </span>
      )}
      <button
        type="button"
        className={BUTTON}
        disabled={sending}
        onClick={() => {
          setSending(true);
          setError(null);
          printTestReceipt()
            .catch((reason: unknown) => setError(uiErrorKey(reason)))
            .finally(() => setSending(false));
        }}
      >
        {t('testReceipt.print')}
      </button>
    </span>
  );
}
