import { ReceiptPaper } from '@/screens/receipts/receipt-paper';
import { useNearViewport } from '@/shared/hooks/use-near-viewport';
import { getLocale, useTranslation } from '@/shared/hooks/use-translation';
import type {
  IReceiptState,
  IReceiptSummary,
} from '@/shared/interfaces/emulator';
import { cn } from '@/shared/styles/cn';
import { formatBytes, peerHost } from '@/shared/utils/format';

/** Paper margin around the printable area, in CSS pixels (= dots). */
const MARGIN = 16;
/** Space kept for a receipt still printing: its blocks are drawn once it ends. */
const PRINTING_HEIGHT = 48;

const STATE_DOT: Record<Exclude<IReceiptState, 'done'>, string> = {
  printing: 'bg-accent',
  idle_timeout: 'bg-warning',
  too_large: 'bg-error',
  connection_error: 'bg-error',
};

interface IReceiptCardProps {
  receipt: IReceiptSummary;
  /** The scrolling element, for drawing only the receipts near the visible area. */
  root: Element | null;
}

export function ReceiptCard({ receipt, root }: IReceiptCardProps) {
  const { t } = useTranslation();
  const { ref, near } = useNearViewport<HTMLDivElement>(root);
  const locale = getLocale();
  const startedAt = new Date(receipt.started_at);
  const printing = receipt.state === 'printing';

  return (
    <li className="flex flex-col gap-2">
      <header className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-muted">
        <time
          dateTime={startedAt.toISOString()}
          className="text-ink tabular-nums"
        >
          {startedAt.toLocaleTimeString(locale)}
        </time>
        <span>{t('receipts.from', { host: peerHost(receipt.peer) })}</span>
        {!printing && (
          <span className="tabular-nums">
            {formatBytes(receipt.size, locale)}
          </span>
        )}
        {receipt.state !== 'done' && (
          <span className="flex items-center gap-1.5 text-ink">
            <span
              aria-hidden
              className={cn('size-2 rounded-full', STATE_DOT[receipt.state])}
            />
            {t(`receipts.state.${receipt.state}`)}
          </span>
        )}
        {receipt.drawer && <Badge>{t('receipts.drawer')}</Badge>}
        {receipt.beeps > 0 && (
          <Badge>{t('receipts.beeps', { count: receipt.beeps })}</Badge>
        )}
      </header>
      {/* Paper and torn edge touch: no gap between them. */}
      <div>
        <div
          ref={ref}
          className="bg-paper shadow-sm"
          style={{
            width: receipt.width + 2 * MARGIN,
            padding: `${MARGIN}px ${MARGIN}px ${receipt.cut ? 4 : MARGIN}px`,
            minHeight: printing ? PRINTING_HEIGHT : undefined,
          }}
        >
          {near && !printing ? (
            <ReceiptPaper receipt={receipt} />
          ) : (
            <div style={{ height: printing ? 0 : receipt.height }} />
          )}
        </div>
        {receipt.cut && (
          <div
            aria-hidden
            className="paper-cut"
            style={{ width: receipt.width + 2 * MARGIN }}
          />
        )}
      </div>
    </li>
  );
}

function Badge({ children }: { children: string }) {
  return (
    <span className="rounded-full border border-border px-2 py-0.5 text-xs text-ink">
      {children}
    </span>
  );
}
