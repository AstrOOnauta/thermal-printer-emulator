import type { RefObject } from 'react';

import { IconButton } from '@/components/ui/icon-button';
import { TrashIcon } from '@/components/ui/icons';
import { clearReceipts } from '@/shared/api/emulator';
import { useTranslation } from '@/shared/hooks/use-translation';
import { BUTTON, INTERACTIVE } from '@/shared/styles/patterns';

/**
 * Clearing cannot be undone, so it asks first, in a native modal `<dialog>`: it traps
 * focus, closes on Esc, and starts on Cancel, the safe choice.
 */
export function ClearButton({
  dialogRef,
}: {
  /** Owned by the app, so the ⌘⌫ shortcut opens the same dialog. */
  dialogRef: RefObject<HTMLDialogElement | null>;
}) {
  const { t } = useTranslation();

  return (
    <>
      <IconButton
        label={t('receipts.clear')}
        shortcut="⌫"
        onClick={() => dialogRef.current?.showModal()}
      >
        <TrashIcon />
      </IconButton>
      <dialog
        ref={dialogRef}
        aria-labelledby="clear-title"
        className="m-auto w-[min(24rem,calc(100%-2rem))] rounded-lg border border-border bg-raised p-5 text-ink shadow-xl backdrop:bg-black/40"
      >
        <h2 id="clear-title" className="text-base font-semibold">
          {t('receipts.clearTitle')}
        </h2>
        <p className="mt-2 text-sm text-muted">{t('receipts.clearBody')}</p>
        <form method="dialog" className="mt-5 flex justify-end gap-2">
          {/* Buttons in a `method="dialog"` form close the dialog by themselves. */}
          <button type="submit" className={BUTTON} autoFocus>
            {t('receipts.cancel')}
          </button>
          <button
            type="submit"
            // text-surface: white on the dark theme's light red would be 2.8:1.
            className={`rounded-md bg-error px-3 py-1.5 text-sm font-medium text-surface ${INTERACTIVE}`}
            onClick={() => void clearReceipts()}
          >
            {t('receipts.clearConfirm')}
          </button>
        </form>
      </dialog>
    </>
  );
}
