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
}

/** Fetches the receipt's print model once it is finished and draws it, slice by slice. */
export function ReceiptPaper({ receipt }: IReceiptPaperProps) {
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
    return <div style={{ height: receipt.height }} />;
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
}

function Slice({ positioned, width, top, bottom }: ISliceProps) {
  const canvas = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (canvas.current)
      drawSlice(canvas.current, positioned, width, top, bottom);
  }, [positioned, width, top, bottom]);

  return <canvas ref={canvas} className="block" aria-hidden />;
}
