import { useEffect, useState } from 'react';

import { exportReceipt } from '@/shared/api/emulator';
import {
  type TranslationScope,
  useTranslation,
} from '@/shared/hooks/use-translation';
import { INTERACTIVE } from '@/shared/styles/patterns';
import { uiErrorKey } from '@/shared/utils/ui-error';

/** Saves the receipt's raw bytes (for a bug report or a replay with `nc`). */
export function ExportButton({ id }: { id: number }) {
  const { t } = useTranslation();
  const [result, setResult] = useState<TranslationScope | null>(null);

  useEffect(() => {
    if (!result) return;
    const timer = setTimeout(() => setResult(null), 2500);
    return () => clearTimeout(timer);
  }, [result]);

  return (
    <span className="flex items-center gap-2">
      <button
        type="button"
        className={`text-xs text-accent underline-offset-2 hover:underline ${INTERACTIVE}`}
        onClick={() => {
          exportReceipt(id)
            .then(() => setResult('receipts.exported'))
            .catch((reason: unknown) => setResult(uiErrorKey(reason)));
        }}
      >
        {t('receipts.export')}
      </button>
      {result && (
        <span role="status" className="text-xs text-muted">
          {t(result)}
        </span>
      )}
    </span>
  );
}
