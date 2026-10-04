import { useEffect, useState } from 'react';

import { clearReceipts } from '@/shared/api/emulator';
import { useTranslation } from '@/shared/hooks/use-translation';
import { BUTTON } from '@/shared/styles/patterns';

/** How long the "Clear all?" confirmation waits for the second click. */
const CONFIRM_MS = 3000;

/** Two clicks: the history cannot come back once cleared. */
export function ClearButton() {
  const { t } = useTranslation();
  const [armed, setArmed] = useState(false);

  useEffect(() => {
    if (!armed) return;
    const timer = setTimeout(() => setArmed(false), CONFIRM_MS);
    return () => clearTimeout(timer);
  }, [armed]);

  return (
    <button
      type="button"
      className={`${BUTTON} ${armed ? 'border-error text-error' : ''}`}
      onClick={() => {
        if (!armed) {
          setArmed(true);
          return;
        }
        setArmed(false);
        void clearReceipts();
      }}
    >
      {armed ? t('receipts.clearConfirm') : t('receipts.clear')}
    </button>
  );
}
