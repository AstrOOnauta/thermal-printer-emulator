import { useCallback, useEffect, useRef, useState } from 'react';

import { ListenerFailureHint } from '@/app/listener-status';
import { StatusBar } from '@/app/status-bar';
import { IconButton } from '@/components/ui/icon-button';
import { GearIcon } from '@/components/ui/icons';
import { ReceiptsScreen } from '@/screens/receipts';
import { ClearButton } from '@/screens/receipts/clear-button';
import { TestReceiptButton } from '@/screens/receipts/test-receipt-button';
import { SettingsPanel } from '@/screens/settings';
import { getAppLocale } from '@/shared/api/app';
import {
  getLanAddress,
  getListenerStatus,
  getReceipts,
  onListenerStatus,
  onReceipts,
} from '@/shared/api/emulator';
import { getSettings } from '@/shared/api/settings';
import { useSynced } from '@/shared/hooks/use-synced';
import { setLocale, useTranslation } from '@/shared/hooks/use-translation';
import type {
  IListenerStatus,
  IReceiptSummary,
  ISettings,
} from '@/shared/interfaces/emulator';

const STARTING: IListenerStatus = { state: 'starting' };
const NO_RECEIPTS: IReceiptSummary[] = [];

export function App() {
  const { t } = useTranslation();
  const status =
    useSynced(onListenerStatus, getListenerStatus, STARTING) ?? STARTING;
  const receipts = useSynced(onReceipts, getReceipts, NO_RECEIPTS);
  const [settings, setSettings] = useState<ISettings | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [lanAddress, setLanAddress] = useState<string | null>(null);
  // Bumped when the language changes: re-rendering the tree re-runs every `t()`.
  const [, setLocaleTag] = useState('');
  const settingsButton = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    getSettings()
      .then(setSettings)
      .catch(() => {
        // Not running inside Tauri (`yarn dev` in a browser): no settings.
      });
  }, []);

  const listening = status.state === 'listening';

  // The network may change while the app runs: ask again whenever the listener (re)starts.
  useEffect(() => {
    if (!listening) return;
    getLanAddress()
      .then(setLanAddress)
      .catch(() => setLanAddress(null));
  }, [listening, status]);

  const onSettingsSaved = (next: ISettings) => {
    const languageChanged = next.language !== settings?.language;
    setSettings(next);
    if (!languageChanged) return;
    // Rust resolved `system` to the OS language; ask it, so window and tray agree.
    getAppLocale()
      .then((tag) => {
        document.documentElement.lang = setLocale(tag);
        setLocaleTag(tag);
      })
      .catch(() => {
        // Not running inside Tauri.
      });
  };

  const closeSettings = useCallback(() => {
    setSettingsOpen(false);
    settingsButton.current?.focus();
  }, []);

  const local = settings?.bind === 'local';
  const host = local ? '127.0.0.1' : (lanAddress ?? '127.0.0.1');
  const address = listening ? `${host}:${status.port}` : null;
  const kind = local ? 'local' : lanAddress ? 'lan' : 'offline';
  const hasReceipts = (receipts?.length ?? 0) > 0;

  return (
    <main className="flex h-full flex-col">
      <header className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 border-b border-border px-4 py-2.5">
        <StatusBar status={status} address={address} kind={kind} />
        <div className="flex shrink-0 items-center gap-2">
          {listening && hasReceipts && <TestReceiptButton />}
          {hasReceipts && <ClearButton />}
          {settings && (
            <IconButton
              ref={settingsButton}
              label={t('nav.settings')}
              aria-expanded={settingsOpen}
              aria-controls="settings-panel"
              onClick={() => setSettingsOpen((open) => !open)}
            >
              <GearIcon />
            </IconButton>
          )}
        </div>
      </header>
      <ListenerFailureHint status={status} />
      {/* overflow-hidden: the closed settings panel waits off-screen to the right. */}
      <div className="relative flex min-h-0 flex-1 flex-col overflow-hidden">
        <ReceiptsScreen
          receipts={receipts}
          address={address}
          sound={settings?.sound ?? true}
        />
        {settings && (
          <SettingsPanel
            open={settingsOpen}
            settings={settings}
            onSaved={onSettingsSaved}
            onClose={closeSettings}
          />
        )}
      </div>
    </main>
  );
}
