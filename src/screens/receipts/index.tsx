import { useState } from 'react';

import { ReceiptCard } from '@/screens/receipts/receipt-card';
import { getReceipts, onReceipts } from '@/shared/api/emulator';
import { useSynced } from '@/shared/hooks/use-synced';
import { useTranslation } from '@/shared/hooks/use-translation';
import type { IReceiptSummary } from '@/shared/interfaces/emulator';

const NO_RECEIPTS: IReceiptSummary[] = [];

interface IReceiptsScreenProps {
  /** For the empty-state hint. */
  port: number;
}

/** The printed receipts on paper, newest first. */
export function ReceiptsScreen({ port }: IReceiptsScreenProps) {
  const { t } = useTranslation();
  const receipts = useSynced(onReceipts, getReceipts, NO_RECEIPTS);
  // State, not a ref: the cards' observers need the element once it exists.
  const [scroller, setScroller] = useState<HTMLElement | null>(null);

  if (receipts === null) return null;

  if (receipts.length === 0) {
    return (
      <div
        role="status"
        className="flex flex-1 flex-col items-center justify-center gap-1 px-6 text-center"
      >
        <p className="font-medium text-ink">{t('receipts.empty')}</p>
        <p className="text-sm text-muted">
          {t('receipts.emptyHint', { port })}
        </p>
      </div>
    );
  }

  return (
    <section
      ref={setScroller}
      aria-label={t('receipts.title')}
      className="flex-1 overflow-y-auto px-6 py-6"
    >
      <ol className="flex flex-col items-center gap-8">
        {[...receipts].reverse().map((receipt) => (
          <ReceiptCard key={receipt.id} receipt={receipt} root={scroller} />
        ))}
      </ol>
    </section>
  );
}
