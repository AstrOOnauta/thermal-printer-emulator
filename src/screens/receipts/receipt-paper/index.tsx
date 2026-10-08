import { useEffect, useMemo } from 'react';

import { useNearViewport } from '@/shared/hooks/use-near-viewport';
import { useTranslation } from '@/shared/hooks/use-translation';
import type {
  IReceiptSummary,
  IReceiptView,
} from '@/shared/interfaces/emulator';
import { drawSlice } from '@/shared/utils/draw-receipt';
import {
  type IPositionedBlock,
  MAX_DRAWN_HEIGHT,
  positionBlocks,
  sliceHeight,
  slices,
} from '@/shared/utils/receipt-layout';

interface IReceiptPaperProps {
  receipt: IReceiptSummary;
  /** The print model: `undefined` while loading, `null` once Rust dropped it. */
  view: IReceiptView | null | undefined;
  /** CSS pixels per dot. */
  scale: number;
  /** The scrolling element: only slices near its visible area are drawn. */
  root: Element | null;
}

/**
 * Draws the receipt's print model, slice by slice; a slice far from the visible area keeps
 * its size but no pixels.
 */
export function ReceiptPaper({
  receipt,
  view,
  scale,
  root,
}: IReceiptPaperProps) {
  const { t } = useTranslation();

  const layout = useMemo(
    () => (view ? positionBlocks(view.blocks) : null),
    [view],
  );

  if (view === null) {
    return (
      <p className="px-4 py-6 text-center text-sm text-muted">
        {t('receipts.unavailable')}
      </p>
    );
  }
  if (!layout || !view) {
    return (
      <div
        style={{ height: Math.min(receipt.height, MAX_DRAWN_HEIGHT) * scale }}
      />
    );
  }
  const size = sliceHeight((window.devicePixelRatio || 1) * scale);
  return (
    <div>
      {slices(Math.min(layout.height, MAX_DRAWN_HEIGHT), size).map(
        ([top, bottom], index) => (
          <Slice
            // By index: slices are positions, not items that move. A zoom changes every
            // slice's range, and a remount would start "not near" and paint a blank frame.
            // eslint-disable-next-line @eslint-react/no-array-index-key
            key={index}
            positioned={layout.positioned}
            width={view.width}
            top={top}
            bottom={bottom}
            scale={scale}
            root={root}
          />
        ),
      )}
      {layout.height > MAX_DRAWN_HEIGHT && (
        <p className="py-3 text-center text-sm text-muted">
          {t('receipts.tooTall')}
        </p>
      )}
    </div>
  );
}

interface ISliceProps {
  positioned: IPositionedBlock[];
  width: number;
  top: number;
  bottom: number;
  scale: number;
  root: Element | null;
}

function Slice({ positioned, width, top, bottom, scale, root }: ISliceProps) {
  const { ref, near } = useNearViewport<HTMLCanvasElement>(root);

  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    if (near) {
      drawSlice(canvas, positioned, width, top, bottom, scale);
    } else {
      // Far away: give its pixels back (a slice canvas is up to 4096 device pixels tall).
      canvas.width = 0;
      canvas.height = 0;
    }
  }, [ref, near, positioned, width, top, bottom, scale]);

  // The CSS size is set here, not in `drawSlice`: a canvas without one is 300×150 until
  // the effect runs, and that brief shrink moved the scroll position.
  return (
    <canvas
      ref={ref}
      className="block"
      style={{ width: width * scale, height: (bottom - top) * scale }}
      aria-hidden
    />
  );
}
