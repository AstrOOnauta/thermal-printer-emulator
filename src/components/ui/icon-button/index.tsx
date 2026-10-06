import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from 'react';

import { cn } from '@/shared/styles/cn';
import { INTERACTIVE } from '@/shared/styles/patterns';

interface IIconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** Read by screen readers and shown as the tooltip. */
  label: string;
  /** Shown in the tooltip, e.g. "⌘,". */
  shortcut?: string;
  children: ReactNode;
}

/** A square button with only an icon: the label is the tooltip and the accessible name. */
export const IconButton = forwardRef<HTMLButtonElement, IIconButtonProps>(
  ({ label, shortcut, children, className, ...props }, ref) => (
    <button
      ref={ref}
      type="button"
      aria-label={label}
      aria-keyshortcuts={shortcut}
      title={shortcut ? `${label} (${shortcut})` : label}
      className={cn(
        'inline-flex size-8 items-center justify-center rounded-md text-muted hover:bg-ink/5 hover:text-ink aria-expanded:bg-ink/5 aria-expanded:text-ink',
        INTERACTIVE,
        className,
      )}
      {...props}
    >
      {children}
    </button>
  ),
);
