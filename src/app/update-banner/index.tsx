import { useState } from 'react';

import {
  getUpdate,
  installUpdate,
  onUpdate,
  openReleasePage,
} from '@/shared/api/app';
import { useSynced } from '@/shared/hooks/use-synced';
import {
  type TranslationScope,
  useTranslation,
} from '@/shared/hooks/use-translation';
import { INTERACTIVE } from '@/shared/styles/patterns';
import { uiErrorKey } from '@/shared/utils/ui-error';

/**
 * Under the top bar while a daily check has found a newer version: install it (the app
 * restarts into it), or, for a `.deb` or `.rpm`, open the release page. "Later" hides it
 * for that version until the app restarts.
 */
export function UpdateBanner() {
  const { t } = useTranslation();
  const update = useSynced(onUpdate, getUpdate, null);
  const [dismissed, setDismissed] = useState<string | null>(null);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<TranslationScope | null>(null);

  if (!update || dismissed === update.version) return null;

  const act = () => {
    if (!update.installable) {
      openReleasePage().catch(() => {
        // Not running inside Tauri.
      });
      return;
    }
    setError(null);
    setInstalling(true);
    // On success the app restarts into the new version.
    installUpdate().catch((reason: unknown) => {
      setError(uiErrorKey(reason));
      setInstalling(false);
    });
  };

  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-1 border-b border-border bg-accent/10 px-4 py-2 text-sm text-ink">
      <p role="status">{t('update.available', { version: update.version })}</p>
      <div className="flex items-center gap-3">
        <button
          type="button"
          disabled={installing}
          className={`font-medium text-accent ${INTERACTIVE}`}
          onClick={act}
        >
          {installing
            ? t('update.installing')
            : update.installable
              ? t('update.install')
              : t('update.download')}
        </button>
        {!installing && (
          <button
            type="button"
            className={`text-muted ${INTERACTIVE}`}
            onClick={() => setDismissed(update.version)}
          >
            {t('update.later')}
          </button>
        )}
      </div>
      {error && (
        <p role="alert" className="basis-full text-xs text-error">
          {t(error)}
        </p>
      )}
    </div>
  );
}
