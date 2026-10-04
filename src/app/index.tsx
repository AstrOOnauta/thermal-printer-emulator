import { useEffect, useState } from 'react';

import { ConnectionBar } from '@/app/connection-bar';
import {
  ListenerFailureHint,
  ListenerStatusBadge,
} from '@/app/listener-status';
import { ReceiptsScreen } from '@/screens/receipts';
import { SettingsScreen } from '@/screens/settings';
import {
  getLanAddress,
  getListenerStatus,
  onListenerStatus,
} from '@/shared/api/emulator';
import { getSettings } from '@/shared/api/settings';
import { useSynced } from '@/shared/hooks/use-synced';
import { useTranslation } from '@/shared/hooks/use-translation';
import type { IListenerStatus, ISettings } from '@/shared/interfaces/emulator';
import { BUTTON } from '@/shared/styles/patterns';

const STARTING: IListenerStatus = { state: 'starting' };

export function App() {
  const { t } = useTranslation();
  const status =
    useSynced(onListenerStatus, getListenerStatus, STARTING) ?? STARTING;
  const [settings, setSettings] = useState<ISettings | null>(null);
  const [screen, setScreen] = useState<'receipts' | 'settings'>('receipts');
  const [lanAddress, setLanAddress] = useState<string | null>(null);

  useEffect(() => {
    getSettings()
      .then(setSettings)
      .catch(() => {
        // Not running inside Tauri (`yarn dev` in a browser): no settings screen.
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

  const local = settings?.bind === 'local';
  const host = local ? '127.0.0.1' : (lanAddress ?? '127.0.0.1');
  const address = listening ? `${host}:${status.port}` : null;
  const kind = local ? 'local' : lanAddress ? 'lan' : 'offline';

  return (
    <main className="flex h-full flex-col">
      <header className="flex items-start justify-between gap-4 border-b border-border px-6 py-4">
        <div>
          <h1 className="text-lg font-bold text-ink">{t('app.name')}</h1>
          <p className="text-sm text-muted">{t('app.tagline')}</p>
        </div>
        <div className="flex flex-col items-end gap-2">
          <ListenerStatusBadge status={status} />
          {settings && (
            <button
              type="button"
              className={BUTTON}
              onClick={() =>
                setScreen(screen === 'receipts' ? 'settings' : 'receipts')
              }
            >
              {screen === 'receipts' ? t('nav.settings') : t('nav.receipts')}
            </button>
          )}
        </div>
      </header>
      <ListenerFailureHint status={status} />
      {address && <ConnectionBar address={address} kind={kind} />}
      {screen === 'settings' && settings ? (
        <SettingsScreen settings={settings} onSaved={setSettings} />
      ) : (
        <ReceiptsScreen address={address} />
      )}
    </main>
  );
}
