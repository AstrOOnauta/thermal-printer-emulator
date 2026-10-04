import { useState } from 'react';

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

  return (
    <div className="flex flex-col items-center gap-2">
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
      {error && (
        <p role="alert" className="text-sm text-error">
          {t(error)}
        </p>
      )}
    </div>
  );
}
