import type { ButtonHTMLAttributes, ReactNode, Ref } from 'react';

import { cn } from '@/shared/styles/cn';
import { INTERACTIVE } from '@/shared/styles/patterns';
import { shortcutAria, shortcutLabel } from '@/shared/utils/shortcut';

interface IIconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** Read by screen readers and shown as the tooltip. */
  label: string;
  /** Its shortcut's key with ⌘ / Ctrl, e.g. "," or "⌫": in the tooltip, announced. */
  shortcut?: string;
  children: ReactNode;
  ref?: Ref<HTMLButtonElement>;
}

/** A square button with only an icon: the label is the tooltip and the accessible name. */
export function IconButton({
  label,
  shortcut,
  children,
  className,
  ref,
  ...props
}: IIconButtonProps) {
  return (
    <button
      ref={ref}
      type="button"
      aria-label={label}
      aria-keyshortcuts={shortcut && shortcutAria(shortcut)}
      title={shortcut ? `${label} (${shortcutLabel(shortcut)})` : label}
      className={cn(
        'inline-flex size-8 items-center justify-center rounded-md text-muted hover:bg-ink/5 hover:text-ink aria-expanded:bg-ink/5 aria-expanded:text-ink',
        INTERACTIVE,
        className,
      )}
      {...props}
    >
      {children}
    </button>
  );
}
