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
import {
  getSettings,
  setSettings as saveSettings,
} from '@/shared/api/settings';
import { useSynced } from '@/shared/hooks/use-synced';
import { useTestReceipt } from '@/shared/hooks/use-test-receipt';
import { useUnseenBadge } from '@/shared/hooks/use-unseen-badge';
import { setLocale, useTranslation } from '@/shared/hooks/use-translation';
import type {
  IListenerStatus,
  IReceiptSummary,
  ISettings,
  ISettingsChange,
} from '@/shared/interfaces/emulator';
import {
  hasCommandKey,
  isTyping,
  shortcutLabel,
} from '@/shared/utils/shortcut';
import { stepZoom } from '@/shared/utils/zoom';

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
  const clearDialog = useRef<HTMLDialogElement>(null);
  // The latest saved settings and the save in flight: changes run one after the other,
  // each on top of the previous one, so a quick second change never undoes the first.
  const latestSettings = useRef<ISettings | null>(null);
  const savesInFlight = useRef<Promise<unknown>>(Promise.resolve());
  const testReceipt = useTestReceipt();
  useUnseenBadge(receipts);

  useEffect(() => {
    getSettings()
      .then((loaded) => {
        latestSettings.current = loaded;
        setSettings(loaded);
      })
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

  const updateSettings = useCallback((change: ISettingsChange) => {
    const save = savesInFlight.current
      .then(() => {
        const current = latestSettings.current;
        if (!current) throw new Error('settings not loaded');
        const fields = typeof change === 'function' ? change(current) : change;
        const keys = Object.keys(fields) as (keyof ISettings)[];
        // Nothing new (⌘+ held at 200 %): no write, no restart.
        if (keys.every((key) => fields[key] === current[key])) return current;
        return saveSettings({ ...current, ...fields });
      })
      .then((next) => {
        // In the chain, so the next queued change builds on this one.
        const previous = latestSettings.current;
        latestSettings.current = next;
        setSettings(next);
        if (next.language !== previous?.language) {
          // Rust resolved `system` to the OS language; ask it, so window and tray agree.
          getAppLocale()
            .then((tag) => {
              document.documentElement.lang = setLocale(tag);
              setLocaleTag(tag);
            })
            .catch(() => {
              // Not running inside Tauri.
            });
        }
        return next;
      });
    savesInFlight.current = save.catch(() => {
      // The caller shows the error; the next change still runs.
    });
    return save;
  }, []);

  const closeSettings = useCallback(() => {
    setSettingsOpen(false);
    settingsButton.current?.focus();
  }, []);

  const hasReceipts = (receipts?.length ?? 0) > 0;
  const printTest = testReceipt.print;

  // ⌘, settings · ⌘T test receipt · ⌘⌫ clear · ⌘+ ⌘− ⌘0 zoom (Ctrl on Windows and Linux).
  useEffect(() => {
    const zoom = (step: (current: number) => number) => {
      updateSettings((current) => ({ zoom: step(current.zoom) })).catch(() => {
        // Refused or not in Tauri: the zoom stays.
      });
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (!hasCommandKey(event)) return;
      if (settings && (event.key === '=' || event.key === '+')) {
        event.preventDefault();
        zoom((current) => stepZoom(current, 1));
        return;
      }
      if (settings && event.key === '-') {
        event.preventDefault();
        zoom((current) => stepZoom(current, -1));
        return;
      }
      if (settings && event.key === '0') {
        event.preventDefault();
        zoom(() => 100);
        return;
      }
      if (event.key === ',' && settings) {
        event.preventDefault();
        setSettingsOpen((open) => !open);
      } else if (event.key.toLowerCase() === 't' && listening) {
        event.preventDefault();
        printTest();
      } else if (
        event.key === 'Backspace' &&
        hasReceipts &&
        !isTyping(event.target)
      ) {
        event.preventDefault();
        clearDialog.current?.showModal();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [settings, updateSettings, listening, hasReceipts, printTest]);

  const local = settings?.bind === 'local';
  const host = local ? '127.0.0.1' : (lanAddress ?? '127.0.0.1');
  const address = listening ? `${host}:${status.port}` : null;
  const kind = local ? 'local' : lanAddress ? 'lan' : 'offline';

  return (
    <main className="flex h-full flex-col">
      <header className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 border-b border-border px-4 py-2.5">
        <StatusBar status={status} address={address} kind={kind} />
        <div className="flex shrink-0 items-center gap-2">
          {listening && hasReceipts && <TestReceiptButton test={testReceipt} />}
          {hasReceipts && <ClearButton dialogRef={clearDialog} />}
          {settings && (
            <IconButton
              ref={settingsButton}
              label={t('nav.settings')}
              shortcut={shortcutLabel(',')}
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
          testReceipt={testReceipt}
          scale={(settings?.zoom ?? 100) / 100}
        />
        {settings && (
          <SettingsPanel
            open={settingsOpen}
            settings={settings}
            update={updateSettings}
            onClose={closeSettings}
          />
        )}
      </div>
    </main>
  );
}
