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
) {
  const atEnd = useRef(true);
  const lastCount = useRef(0);
  const loaded = useRef(false);
  const frame = useRef<number | null>(null);
  const [unseen, setUnseen] = useState(0);

  const stopFeed = useCallback(() => {
    if (frame.current !== null) cancelAnimationFrame(frame.current);
    frame.current = null;
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
      const duration = feedDuration(distance);
      const startedAt = performance.now();
      const step = (now: number) => {
        const progress = Math.min(1, (now - startedAt) / duration);
        // Re-read the end every frame: more paper may arrive while feeding.
        const end = element.scrollHeight - element.clientHeight;
        element.scrollTop = from + (end - from) * easeOutCubic(progress);
        frame.current = progress < 1 ? requestAnimationFrame(step) : null;
      };
      frame.current = requestAnimationFrame(step);
    },
    [scrollerRef, stopFeed],
  );

  useEffect(() => {
    const element = scrollerRef.current;
    if (!element) return;
    const onScroll = () => {
      // Our own feed scrolls too: it is at the end by definition.
      if (frame.current !== null) return;
      atEnd.current =
        element.scrollHeight - element.scrollTop - element.clientHeight <
        NEAR_END;
      if (atEnd.current) setUnseen(0);
    };
    // The user takes over: stop feeding, then the scroll position decides again.
    const onUserScroll = () => stopFeed();
    element.addEventListener('scroll', onScroll, { passive: true });
    element.addEventListener('wheel', onUserScroll, { passive: true });
    element.addEventListener('touchstart', onUserScroll, { passive: true });
    element.addEventListener('keydown', onUserScroll);
    return () => {
      element.removeEventListener('scroll', onScroll);
      element.removeEventListener('wheel', onUserScroll);
      element.removeEventListener('touchstart', onUserScroll);
      element.removeEventListener('keydown', onUserScroll);
      stopFeed();
    };
  }, [scrollerRef, attached, stopFeed]);

  useLayoutEffect(() => {
    const element = scrollerRef.current;
    if (!element || !receipts) return;
    const added = receipts.length - lastCount.current;
    lastCount.current = receipts.length;
    if (atEnd.current) {
      // The list opens at the end without motion; later paper feeds.
      feed(!loaded.current);
    } else if (added > 0) {
      setUnseen((previous) => previous + added);
    }
    loaded.current = true;
  }, [scrollerRef, attached, receipts, feed]);

  const jumpToEnd = useCallback(() => {
    atEnd.current = true;
    setUnseen(0);
    feed(false);
  }, [feed]);

  return { unseen, jumpToEnd };
}
