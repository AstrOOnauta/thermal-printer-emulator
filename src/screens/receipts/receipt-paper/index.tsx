import { useEffect, useMemo, useRef, useState } from 'react';

import { getReceipt } from '@/shared/api/emulator';
import { useTranslation } from '@/shared/hooks/use-translation';
import type {
  IReceiptSummary,
  IReceiptView,
} from '@/shared/interfaces/emulator';
import { drawSlice } from '@/shared/utils/draw-receipt';
import {
  type IPositionedBlock,
  positionBlocks,
  slices,
} from '@/shared/utils/receipt-layout';

interface IReceiptPaperProps {
  receipt: IReceiptSummary;
  /** CSS pixels per dot. */
  scale: number;
}

/** Fetches the receipt's print model once it is finished and draws it, slice by slice. */
export function ReceiptPaper({ receipt, scale }: IReceiptPaperProps) {
  const { t } = useTranslation();
  // `undefined` while loading, `null` once Rust dropped it from memory.
  const [view, setView] = useState<IReceiptView | null | undefined>();

  useEffect(() => {
    if (receipt.state === 'printing') return;
    let active = true;
    getReceipt(receipt.id)
      .then((next) => active && setView(next))
      .catch(() => active && setView(null));
    return () => {
      active = false;
    };
  }, [receipt.id, receipt.state]);

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
    return <div style={{ height: receipt.height * scale }} />;
  }
  return (
    <div>
      {slices(layout.height).map(([top, bottom]) => (
        <Slice
          key={top}
          positioned={layout.positioned}
          width={view.width}
          top={top}
          bottom={bottom}
          scale={scale}
        />
      ))}
    </div>
  );
}

interface ISliceProps {
  positioned: IPositionedBlock[];
  width: number;
  top: number;
  bottom: number;
  scale: number;
}

function Slice({ positioned, width, top, bottom, scale }: ISliceProps) {
  const canvas = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (canvas.current)
      drawSlice(canvas.current, positioned, width, top, bottom, scale);
  }, [positioned, width, top, bottom, scale]);

  // The CSS size is set here, not in `drawSlice`: a canvas without one is 300×150 until
  // the effect runs, and that brief shrink moved the scroll position.
  return (
    <canvas
      ref={canvas}
      className="block"
      style={{ width: width * scale, height: (bottom - top) * scale }}
      aria-hidden
    />
  );
}
