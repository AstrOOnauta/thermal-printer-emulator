import { useCallback, useEffect, useRef, useState } from 'react';

import { ReceiptCard } from '@/screens/receipts/receipt-card';
import { TestReceiptButton } from '@/screens/receipts/test-receipt-button';
import { useFollowBottom } from '@/screens/receipts/use-follow-bottom';
import type { ITestReceipt } from '@/shared/hooks/use-test-receipt';
import { useTranslation } from '@/shared/hooks/use-translation';
import type { IReceiptSummary } from '@/shared/interfaces/emulator';
import { feedDuration } from '@/shared/utils/paper-feed';
import { pendingSounds, playBeeps, playPrint } from '@/shared/utils/sounds';

interface IReceiptsScreenProps {
  /** `null` until the first answer from Rust. */
  receipts: IReceiptSummary[] | null;
  /** Where to print, for the empty-state hint; `null` while the port is not open. */
  address: string | null;
  /** Play the printing sound and the printer's beep (setting). */
  sound: boolean;
  /** For the empty state's button. */
  testReceipt: ITestReceipt;
  /** Paper zoom: CSS pixels per dot. */
  scale: number;
}

/** The printed receipts on paper, oldest first: the newest is at the bottom, like a roll. */
export function ReceiptsScreen({
  receipts,
  address,
  sound,
  testReceipt,
  scale,
}: IReceiptsScreenProps) {
  const { t } = useTranslation();
  // State, not a ref: the cards' observers need the element once it exists.
  const [scroller, setScroller] = useState<HTMLElement | null>(null);
  // The same element as a ref, which the follow logic may mutate (scrollTop).
  const scrollerRef = useRef<HTMLElement | null>(null);
  const attachScroller = useCallback((node: HTMLElement | null) => {
    scrollerRef.current = node;
    setScroller(node);
    // The list replaced the empty state: the focused test button is gone, so focus would
    // fall to the page. Keep it in the content instead.
    if (node && document.activeElement === document.body) {
      node.focus({ preventScroll: true });
    }
  }, []);
  const heardRef = useRef<Set<number> | null>(null);
  const { unseen, jumpToEnd } = useFollowBottom(
    scrollerRef,
    scroller,
    receipts,
    scale,
  );

  // Receipts that finish from now on print with motion; older ones are just there.
  const [openedAt] = useState(() => Date.now());
  // The app runs for days in the tray: refreshed whenever the window comes back.
  const [today, setToday] = useState(() => new Date().toDateString());
  useEffect(() => {
    const refresh = () => setToday(new Date().toDateString());
    window.addEventListener('focus', refresh);
    return () => window.removeEventListener('focus', refresh);
  }, []);

  // The printing sound for each receipt that just finished (as long as its paper takes to
  // come out), then the beeps it asked for with `ESC B`.
  useEffect(() => {
    if (receipts === null) return;
    const { printed, beeps, seen } = pendingSounds(heardRef.current, receipts);
    heardRef.current = seen;
    if (!sound || printed.length === 0) return;
    const longest = Math.max(
      ...printed.map((receipt) => feedDuration(receipt.height)),
    );
    playPrint(longest);
    playBeeps(beeps, longest);
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
              <TestReceiptButton test={testReceipt} />
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
        // Focusable, so the keyboard can scroll it (WebKit does not focus scroll areas).
        tabIndex={0}
        className="flex-1 overflow-y-auto px-4 py-6 outline-none [overflow-anchor:none] focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-inset"
      >
        {/* w-max + min-w-full: a zoomed receipt wider than the window scrolls sideways
            from its left edge instead of being cut off by centering. */}
        <ol className="flex w-max min-w-full flex-col items-center gap-8">
          {receipts.map((receipt) => (
            <ReceiptCard
              key={receipt.id}
              receipt={receipt}
              root={scroller}
              fresh={receipt.ended_at !== null && receipt.ended_at >= openedAt}
              scale={scale}
              today={today}
            />
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
