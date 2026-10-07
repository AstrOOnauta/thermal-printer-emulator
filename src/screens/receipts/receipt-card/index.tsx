import { ExportButton } from '@/screens/receipts/export-button';
import type { CSSProperties } from 'react';

import { ReceiptPaper } from '@/screens/receipts/receipt-paper';
import { useNearViewport } from '@/shared/hooks/use-near-viewport';
import { getLocale, useTranslation } from '@/shared/hooks/use-translation';
import type {
  IReceiptState,
  IReceiptSummary,
} from '@/shared/interfaces/emulator';
import { cn } from '@/shared/styles/cn';
import { formatBytes, peerHost } from '@/shared/utils/format';
import { feedDuration } from '@/shared/utils/paper-feed';

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
  /** Finished while the window was open: its paper comes out with motion. */
  fresh: boolean;
  /** Paper zoom: CSS pixels per dot. */
  scale: number;
}

export function ReceiptCard({
  receipt,
  root,
  fresh,
  scale,
}: IReceiptCardProps) {
  const margin = MARGIN * scale;
  const paperWidth = receipt.width * scale + 2 * margin;
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
        {!printing && <ExportButton id={receipt.id} />}
      </header>
      {/* Paper and torn edge touch: no gap between them. A fresh receipt is revealed top
          to bottom while the list feeds it out (same duration as the scroll). */}
      <div
        className={fresh && !printing ? 'paper-print' : undefined}
        style={
          fresh && !printing
            ? ({
                '--feed-duration': `${feedDuration(receipt.height)}ms`,
              } as CSSProperties)
            : undefined
        }
      >
        <div
          ref={ref}
          className="bg-paper shadow-sm"
          style={{
            width: paperWidth,
            padding: `${margin}px ${margin}px ${receipt.cut ? 4 : margin}px`,
            minHeight: printing ? PRINTING_HEIGHT : undefined,
          }}
        >
          {near && !printing ? (
            <ReceiptPaper receipt={receipt} scale={scale} />
          ) : (
            <div style={{ height: printing ? 0 : receipt.height * scale }} />
          )}
        </div>
        {receipt.cut && (
          <div
            aria-hidden
            className="paper-cut"
            style={{ width: paperWidth }}
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
