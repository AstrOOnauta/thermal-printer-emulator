import { useEffect, useRef, useState } from 'react';

/** How far outside the scroll area a receipt is drawn ahead of time. */
const MARGIN = '1500px 0px';

/**
 * Whether the element is within `MARGIN` of `root`'s visible area. Receipts far away are
 * not drawn, so 100 tall canvases never sit in memory at once.
 */
export function useNearViewport<T extends Element>(root: Element | null) {
  const ref = useRef<T>(null);
  const [near, setNear] = useState(false);

  useEffect(() => {
    const element = ref.current;
    if (!element || !root) return;
    const observer = new IntersectionObserver(
      // One callback may carry several entries for the element: the last is current.
      (entries) => setNear(entries.at(-1)?.isIntersecting ?? false),
      { root, rootMargin: MARGIN },
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, [root]);

  return { ref, near };
}
