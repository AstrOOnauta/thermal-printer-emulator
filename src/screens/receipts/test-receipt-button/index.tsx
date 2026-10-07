import type { ITestReceipt } from '@/shared/hooks/use-test-receipt';
import { useTranslation } from '@/shared/hooks/use-translation';
import { BUTTON } from '@/shared/styles/patterns';
import { shortcutAria, shortcutLabel } from '@/shared/utils/shortcut';

/** Sends the test receipt; the receipt itself shows up in the list like any other. */
export function TestReceiptButton({ test }: { test: ITestReceipt }) {
  const { t } = useTranslation();

  return (
    <span className="flex items-center gap-2">
      {test.error && (
        <span role="alert" className="text-xs text-error">
          {t(test.error)}
        </span>
      )}
      <button
        type="button"
        className={BUTTON}
        disabled={test.sending}
        title={`${t('testReceipt.print')} (${shortcutLabel('T')})`}
        aria-keyshortcuts={shortcutAria('T')}
        onClick={test.print}
      >
        {t('testReceipt.print')}
      </button>
    </span>
  );
}
