import { ReceiptRow } from '@/screens/receipts/receipt-row';
import { getReceipts, onReceipts } from '@/shared/api/emulator';
import { useSynced } from '@/shared/hooks/use-synced';
import { useTranslation } from '@/shared/hooks/use-translation';
import type { IReceiptSummary } from '@/shared/interfaces/emulator';

const NO_RECEIPTS: IReceiptSummary[] = [];

interface IReceiptsScreenProps {
  /** For the empty-state hint. */
  port: number;
}

// ponytail: a plain list of receipts until the renderer draws them on paper.
export function ReceiptsScreen({ port }: IReceiptsScreenProps) {
  const { t } = useTranslation();
  const receipts = useSynced(onReceipts, getReceipts, NO_RECEIPTS);

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
      aria-labelledby="receipts-title"
      className="flex-1 overflow-y-auto px-6 py-4"
    >
      <h2 id="receipts-title" className="mb-3 text-sm font-semibold text-muted">
        {t('receipts.title')}
      </h2>
      <ol className="divide-y divide-border rounded-lg border border-border">
        {[...receipts].reverse().map((receipt) => (
          <ReceiptRow key={receipt.id} receipt={receipt} />
        ))}
      </ol>
    </section>
  );
}
