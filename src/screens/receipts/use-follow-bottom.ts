import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type RefObject,
} from 'react';

import type { IReceiptSummary } from '@/shared/interfaces/emulator';
import {
  easeOutCubic,
  feedDuration,
  prefersReducedMotion,
} from '@/shared/utils/paper-feed';

/** How close to the end still counts as "at the end", in pixels. */
const NEAR_END = 64;

/**
 * The receipt list reads like a paper roll: the newest receipt is at the bottom. While the
 * user is at the end, the list **feeds** new paper: it scrolls to the end with the same
 * duration and curve as the receipt's print reveal, so the printed edge stays on the
 * window's bottom edge, like paper leaving a printer's slot. Once the user scrolls up to
 * read, it stays put and counts what arrived (`unseen`) instead of yanking them down.
 *
 * `scrollerRef` is the scrolling element (a ref: it is mutated); `attached` is the same
 * element as state, only to re-subscribe when it appears. `receipts` is a new array on
 * every change (a receipt arrived, or one finished printing and grew), which is exactly
 * when the height may change.
 */
export function useFollowBottom(
  scrollerRef: RefObject<HTMLElement | null>,
  attached: HTMLElement | null,
  receipts: IReceiptSummary[] | null,
  /** Paper zoom: CSS pixels per dot. The feed's speed is in printer dots. */
  scale: number,
) {
  const atEndRef = useRef(true);
  /** Newest id seen: ids only grow, so arrivals are counted right at the 100 limit too. */
  const newestIdRef = useRef(-1);
  /** Each receipt's offsetTop after the last change, to see what moved. */
  const topsRef = useRef(new Map<number, number>());
  const loadedRef = useRef(false);
  /** scrollTop as of the last scroll event: from before the browser clamps it to a shorter
   * list (scroll events come after layout). */
  const lastScrollTopRef = useRef(0);
  const frameRef = useRef<number | null>(null);
  const [unseen, setUnseen] = useState(0);

  const stopFeed = useCallback(() => {
    if (frameRef.current !== null) cancelAnimationFrame(frameRef.current);
    frameRef.current = null;
  }, []);

  /** Scrolls to the end, animated unless `instant` or reduced motion. */
  const feed = useCallback(
    (instant: boolean) => {
      const element = scrollerRef.current;
      if (!element) return;
      stopFeed();
      const from = element.scrollTop;
      const distance = element.scrollHeight - element.clientHeight - from;
      if (distance <= 0) return;
      if (instant || prefersReducedMotion()) {
        element.scrollTop = from + distance;
        return;
      }
      const duration = feedDuration(distance / scale);
      const startedAt = performance.now();
      const step = (now: number) => {
        const progress = Math.min(1, (now - startedAt) / duration);
        // Re-read the end every frame: more paper may arrive while feeding.
        const end = element.scrollHeight - element.clientHeight;
        element.scrollTop = from + (end - from) * easeOutCubic(progress);
        frameRef.current = progress < 1 ? requestAnimationFrame(step) : null;
      };
      frameRef.current = requestAnimationFrame(step);
    },
    [scrollerRef, stopFeed, scale],
  );

  useEffect(() => {
    const element = scrollerRef.current;
    if (!element) return;
    const onScroll = () => {
      lastScrollTopRef.current = element.scrollTop;
      // Our own feed scrolls too: it is at the end by definition.
      if (frameRef.current !== null) return;
      atEndRef.current =
        element.scrollHeight - element.scrollTop - element.clientHeight <
        NEAR_END;
      if (atEndRef.current) setUnseen(0);
    };
    // The user takes over: stop feeding, then the scroll position decides again.
    const onUserScroll = () => stopFeed();
    element.addEventListener('scroll', onScroll, { passive: true });
    element.addEventListener('wheel', onUserScroll, { passive: true });
    element.addEventListener('touchstart', onUserScroll, { passive: true });
    // Also a drag of the scrollbar, which sends no wheel event (a click on a card does not
    // count: it would stop the paper halfway).
    const onPointerDown = (event: PointerEvent) => {
      if (event.target === element && event.offsetX >= element.clientWidth) {
        stopFeed();
      }
    };
    element.addEventListener('pointerdown', onPointerDown);
    element.addEventListener('keydown', onUserScroll);
    return () => {
      element.removeEventListener('scroll', onScroll);
      element.removeEventListener('wheel', onUserScroll);
      element.removeEventListener('touchstart', onUserScroll);
      element.removeEventListener('pointerdown', onPointerDown);
      element.removeEventListener('keydown', onUserScroll);
      stopFeed();
    };
  }, [scrollerRef, attached, stopFeed]);

  useLayoutEffect(() => {
    const element = scrollerRef.current;
    if (!receipts) return;
    if (receipts.length === 0 || !element) {
      // Cleared (the list unmounts): the next receipt starts at the end, nothing unseen.
      atEndRef.current = true;
      topsRef.current = new Map();
      setUnseen(0);
      return;
    }
    const added = receipts.filter(
      (receipt) => receipt.id > newestIdRef.current,
    ).length;
    newestIdRef.current = Math.max(
      newestIdRef.current,
      ...receipts.map((receipt) => receipt.id),
    );
    const nextTops = new Map<number, number>();
    for (const item of element.querySelectorAll<HTMLElement>(
      '[data-receipt-id]',
    )) {
      nextTops.set(Number(item.dataset.receiptId), item.offsetTop);
    }
    if (!atEndRef.current) {
      // The oldest receipts were dropped (memory limits): what is left moved up by their
      // height. Scroll by as much, so the receipt being read stays in place.
      const kept = receipts.find((receipt) => topsRef.current.has(receipt.id));
      const before = kept && topsRef.current.get(kept.id);
      const after = kept && nextTops.get(kept.id);
      // From the scrollTop before the list got shorter: the browser has clamped it since.
      if (before !== undefined && after !== undefined && after !== before) {
        element.scrollTop = lastScrollTopRef.current + (after - before);
      }
    }
    topsRef.current = nextTops;
    if (atEndRef.current) {
      // The list opens at the end without motion; later paper feeds.
      feed(!loadedRef.current);
    } else if (added > 0) {
      setUnseen((previous) => previous + added);
    }
    loadedRef.current = true;
  }, [scrollerRef, attached, receipts, feed]);

  const jumpToEnd = useCallback(() => {
    atEndRef.current = true;
    setUnseen(0);
    feed(false);
  }, [feed]);

  return { unseen, jumpToEnd };
}
