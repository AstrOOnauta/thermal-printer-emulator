import { useEffect, useRef, useState } from 'react';

import { ClearButton } from '@/screens/receipts/clear-button';
import { ReceiptCard } from '@/screens/receipts/receipt-card';
import { TestReceiptButton } from '@/screens/receipts/test-receipt-button';
import { getReceipts, onReceipts } from '@/shared/api/emulator';
import { useSynced } from '@/shared/hooks/use-synced';
import { useTranslation } from '@/shared/hooks/use-translation';
import type { IReceiptSummary } from '@/shared/interfaces/emulator';
import { pendingBeeps, playBeeps } from '@/shared/utils/beep';

const NO_RECEIPTS: IReceiptSummary[] = [];

interface IReceiptsScreenProps {
  /** Where to print, for the empty-state hint; `null` while the port is not open. */
  address: string | null;
  /** Play the printer's beep (setting). */
  sound: boolean;
}

/** The printed receipts on paper, newest first. */
export function ReceiptsScreen({ address, sound }: IReceiptsScreenProps) {
  const { t } = useTranslation();
  const receipts = useSynced(onReceipts, getReceipts, NO_RECEIPTS);
  // State, not a ref: the cards' observers need the element once it exists.
  const [scroller, setScroller] = useState<HTMLElement | null>(null);
  const heard = useRef<Set<number> | null>(null);

  // Ring for receipts that finished with `ESC B` since the last list.
  useEffect(() => {
    if (receipts === null) return;
    const { beeps, seen } = pendingBeeps(heard.current, receipts);
    heard.current = seen;
    if (sound) playBeeps(beeps);
  }, [receipts, sound]);

  if (receipts === null) return null;

  if (receipts.length === 0) {
    return (
      <div
        role="status"
        className="flex flex-1 flex-col items-center justify-center gap-1 px-6 text-center"
      >
        <p className="font-medium text-ink">{t('receipts.empty')}</p>
        {/* While the port is not open, the banner above says why: no hint here. */}
        {address && (
          <>
            <p className="text-sm text-muted">
              {t('receipts.emptyHint', { address })}
            </p>
            <div className="mt-4">
              <TestReceiptButton />
            </div>
          </>
        )}
      </div>
    );
  }

  return (
    <section
      ref={setScroller}
      aria-label={t('receipts.title')}
      className="flex-1 overflow-y-auto px-6 py-6"
    >
      <div className="mb-6 flex items-start justify-end gap-2">
        <TestReceiptButton />
        <ClearButton />
      </div>
      <ol className="flex flex-col items-center gap-8">
        {[...receipts].reverse().map((receipt) => (
          <ReceiptCard key={receipt.id} receipt={receipt} root={scroller} />
        ))}
      </ol>
    </section>
  );
}
