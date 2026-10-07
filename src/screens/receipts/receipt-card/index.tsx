import { useEffect, useState, type CSSProperties } from 'react';

import { ChevronIcon, CopyIcon, DownloadIcon } from '@/components/ui/icons';
import { CommandsPanel } from '@/screens/receipts/commands-panel';
import { ReceiptAction } from '@/screens/receipts/receipt-action';
import { ReceiptPaper } from '@/screens/receipts/receipt-paper';
import { exportReceipt, getReceipt } from '@/shared/api/emulator';
import { useNearViewport } from '@/shared/hooks/use-near-viewport';
import {
  getLocale,
  type TranslationScope,
  useTranslation,
} from '@/shared/hooks/use-translation';
import type {
  IReceiptState,
  IReceiptSummary,
  IReceiptView,
} from '@/shared/interfaces/emulator';
import { cn } from '@/shared/styles/cn';
import { INTERACTIVE } from '@/shared/styles/patterns';
import { formatBytes, peerHost } from '@/shared/utils/format';
import { feedDuration } from '@/shared/utils/paper-feed';
import { MAX_DRAWN_HEIGHT } from '@/shared/utils/receipt-layout';
import { receiptText } from '@/shared/utils/receipt-text';
import { uiErrorKey } from '@/shared/utils/ui-error';

/** Paper margin around the printable area, in CSS pixels (= dots). */
const MARGIN = 16;
/** Rejects "Copy text" for a receipt no longer in memory. */
const GONE = new Error('gone');

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
  /** Today's `Date.toDateString()`: an older receipt shows its date too. */
  today: string;
}

export function ReceiptCard({
  receipt,
  root,
  fresh,
  scale,
  today,
}: IReceiptCardProps) {
  const [showCommands, setShowCommands] = useState(false);
  const [error, setError] = useState<TranslationScope | null>(null);
  const margin = MARGIN * scale;
  const paperWidth = receipt.width * scale + 2 * margin;
  const { t } = useTranslation();
  const { ref, near } = useNearViewport<HTMLDivElement>(root);
  const locale = getLocale();
  const startedAt = new Date(receipt.started_at);
  const printing = receipt.state === 'printing';
  // The print model, fetched the first time the card comes near: `undefined` until then,
  // `null` once Rust dropped it. Kept while the card lives, so scrolling back to it does
  // not fetch it again; its slices unmount when it is far.
  const [view, setView] = useState<IReceiptView | null | undefined>();
  const wanted = near && !printing;

  useEffect(() => {
    if (!wanted || view !== undefined) return;
    let active = true;
    getReceipt(receipt.id)
      .then((next) => active && setView(next))
      .catch(() => active && setView(null));
    return () => {
      active = false;
    };
  }, [wanted, view, receipt.id]);

  useEffect(() => {
    if (!error) return;
    const timer = setTimeout(() => setError(null), 5000);
    return () => clearTimeout(timer);
  }, [error]);

  const copyText = () =>
    getReceipt(receipt.id).then((view) => {
      if (!view) throw GONE;
      return navigator.clipboard.writeText(receiptText(view.blocks));
    });

  return (
    <li className="flex flex-col gap-2" data-receipt-id={receipt.id}>
      {/* As wide as the paper, so the actions end at its right edge. On narrow paper the
          facts wrap inside their own block; the actions stay on the first line. */}
      <header
        className="flex items-start justify-between gap-x-3"
        style={{ width: paperWidth }}
      >
        <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-0.5 py-1 text-sm text-muted">
          <time
            dateTime={startedAt.toISOString()}
            className="text-ink tabular-nums"
          >
            {startedAt.toLocaleTimeString(locale)}
          </time>
          <Dot />
          {startedAt.toDateString() !== today && (
            <>
              <span className="tabular-nums">
                {startedAt.toLocaleDateString(locale, { dateStyle: 'short' })}
              </span>
              <Dot />
            </>
          )}
          <span>{peerHost(receipt.peer)}</span>
          {!printing && (
            <>
              <Dot />
              <span className="tabular-nums">
                {formatBytes(receipt.size, locale)}
              </span>
            </>
          )}
          {receipt.state !== 'done' && (
            <span className="ml-1 flex items-center gap-1.5 text-ink">
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
        </div>
        {!printing && (
          <div className="flex shrink-0 items-center gap-0.5">
            <ReceiptAction
              label={t('receipts.copyText')}
              done={t('receipts.copied')}
              run={copyText}
              onError={(reason) =>
                setError(
                  reason === GONE
                    ? 'receipts.errors.gone'
                    : 'receipts.errors.copy',
                )
              }
            >
              <CopyIcon />
            </ReceiptAction>
            <ReceiptAction
              label={t('receipts.export')}
              done={t('receipts.exported')}
              run={() => exportReceipt(receipt.id)}
              onError={(reason) => setError(uiErrorKey(reason))}
            >
              <DownloadIcon />
            </ReceiptAction>
          </div>
        )}
      </header>
      {error && (
        <p role="alert" className="text-xs text-error">
          {t(error)}
        </p>
      )}
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
          {/* Only near: far cards keep `view` but no slices (canvases and observers). */}
          {!printing && near ? (
            <ReceiptPaper
              receipt={receipt}
              view={view}
              scale={scale}
              root={root}
            />
          ) : (
            <div
              style={{
                height: printing
                  ? 0
                  : Math.min(receipt.height, MAX_DRAWN_HEIGHT) * scale,
              }}
            />
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
      {!printing && (
        <div className="flex flex-col gap-2" style={{ width: paperWidth }}>
          <button
            type="button"
            aria-expanded={showCommands}
            className={`flex items-center gap-1 self-start text-xs text-accent ${INTERACTIVE}`}
            onClick={() => setShowCommands((shown) => !shown)}
          >
            {showCommands ? t('inspect.hide') : t('inspect.show')}
            <ChevronIcon
              className={cn(
                'size-3.5 transition-transform motion-reduce:transition-none',
                showCommands && 'rotate-180',
              )}
            />
          </button>
          {showCommands && <CommandsPanel id={receipt.id} width={paperWidth} />}
        </div>
      )}
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

function Dot() {
  return <span aria-hidden>·</span>;
}
