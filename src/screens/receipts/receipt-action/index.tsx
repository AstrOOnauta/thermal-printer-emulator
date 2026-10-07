import { useEffect, useState, type ReactNode } from 'react';

import { IconButton } from '@/components/ui/icon-button';
import { CheckIcon } from '@/components/ui/icons';

interface IReceiptActionProps {
  /** Tooltip and accessible name. */
  label: string;
  /** Announced to screen readers once `run` succeeds. */
  done: string;
  run: () => Promise<unknown>;
  onError: (reason: unknown) => void;
  children: ReactNode;
}

/**
 * An icon action on a receipt. Success swaps the icon for a check for a moment, in place,
 * so nothing around it moves; the receipt card shows errors as text.
 */
export function ReceiptAction({
  label,
  done,
  run,
  onError,
  children,
}: IReceiptActionProps) {
  const [succeeded, setSucceeded] = useState(false);

  useEffect(() => {
    if (!succeeded) return;
    const timer = setTimeout(() => setSucceeded(false), 1500);
    return () => clearTimeout(timer);
  }, [succeeded]);

  return (
    <>
      <IconButton
        label={label}
        className={
          succeeded ? 'size-7 text-success hover:text-success' : 'size-7'
        }
        onClick={() => {
          run()
            .then(() => setSucceeded(true))
            .catch(onError);
        }}
      >
        {succeeded ? <CheckIcon /> : children}
      </IconButton>
      {/* Rendered empty first: a live region only announces changes. */}
      <span role="status" className="sr-only">
        {succeeded ? done : ''}
      </span>
    </>
  );
}
