import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type RefObject,
} from 'react';

import type { IReceiptSummary } from '@/shared/interfaces/emulator';

/** How close to the end still counts as "at the end", in pixels. */
const NEAR_END = 64;

/**
 * The receipt list reads like a paper roll: the newest receipt is at the bottom. While the
 * user is at the end, the list follows new paper; once they scroll up to read, it stays
 * put and counts what arrived (`unseen`) instead of yanking them down.
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
  const [unseen, setUnseen] = useState(0);

  useEffect(() => {
    const element = scrollerRef.current;
    if (!element) return;
    const onScroll = () => {
      atEnd.current =
        element.scrollHeight - element.scrollTop - element.clientHeight <
        NEAR_END;
      if (atEnd.current) setUnseen(0);
    };
    element.addEventListener('scroll', onScroll, { passive: true });
    return () => element.removeEventListener('scroll', onScroll);
  }, [scrollerRef, attached]);

  useLayoutEffect(() => {
    const element = scrollerRef.current;
    if (!element || !receipts) return;
    const added = receipts.length - lastCount.current;
    lastCount.current = receipts.length;
    if (atEnd.current) {
      element.scrollTop = element.scrollHeight;
    } else if (added > 0) {
      setUnseen((previous) => previous + added);
    }
  }, [scrollerRef, attached, receipts]);

  const jumpToEnd = useCallback(() => {
    const element = scrollerRef.current;
    if (!element) return;
    atEnd.current = true;
    setUnseen(0);
    element.scrollTop = element.scrollHeight;
  }, [scrollerRef]);

  return { unseen, jumpToEnd };
}
