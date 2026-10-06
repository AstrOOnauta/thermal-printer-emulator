import { useCallback, useEffect, useRef, useState } from 'react';

import { ReceiptCard } from '@/screens/receipts/receipt-card';
import { TestReceiptButton } from '@/screens/receipts/test-receipt-button';
import { useFollowBottom } from '@/screens/receipts/use-follow-bottom';
import { useTranslation } from '@/shared/hooks/use-translation';
import type { IReceiptSummary } from '@/shared/interfaces/emulator';
import { pendingBeeps, playBeeps } from '@/shared/utils/beep';

interface IReceiptsScreenProps {
  /** `null` until the first answer from Rust. */
  receipts: IReceiptSummary[] | null;
  /** Where to print, for the empty-state hint; `null` while the port is not open. */
  address: string | null;
  /** Play the printer's beep (setting). */
  sound: boolean;
}

/** The printed receipts on paper, oldest first: the newest is at the bottom, like a roll. */
export function ReceiptsScreen({
  receipts,
  address,
  sound,
}: IReceiptsScreenProps) {
  const { t } = useTranslation();
  // State, not a ref: the cards' observers need the element once it exists.
  const [scroller, setScroller] = useState<HTMLElement | null>(null);
  // The same element as a ref, which the follow logic may mutate (scrollTop).
  const scrollerRef = useRef<HTMLElement | null>(null);
  const attachScroller = useCallback((node: HTMLElement | null) => {
    scrollerRef.current = node;
    setScroller(node);
  }, []);
  const heard = useRef<Set<number> | null>(null);
  const { unseen, jumpToEnd } = useFollowBottom(
    scrollerRef,
    scroller,
    receipts,
  );

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
    <>
      {/* overflow-anchor: none, the list scrolls itself (use-follow-bottom); the browser's
          scroll anchoring must not move it too. */}
      <section
        ref={attachScroller}
        aria-label={t('receipts.title')}
        className="flex-1 overflow-y-auto px-4 py-6 [overflow-anchor:none]"
      >
        <ol className="flex flex-col items-center gap-8">
          {receipts.map((receipt) => (
            <ReceiptCard key={receipt.id} receipt={receipt} root={scroller} />
          ))}
        </ol>
      </section>
      {unseen > 0 && (
        <button
          type="button"
          onClick={jumpToEnd}
          className="absolute bottom-4 left-1/2 z-(--z-pill) -translate-x-1/2 cursor-pointer rounded-full bg-ink px-4 py-1.5 text-sm font-medium text-surface shadow-lg transition-opacity hover:opacity-90"
        >
          ↓{' '}
          {unseen === 1
            ? t('receipts.newOne')
            : t('receipts.newMany', { count: unseen })}
        </button>
      )}
    </>
  );
}
