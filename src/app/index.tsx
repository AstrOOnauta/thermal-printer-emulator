import {
  ListenerFailureHint,
  ListenerStatusBadge,
} from '@/app/listener-status';
import { ReceiptsScreen } from '@/screens/receipts';
import { getListenerStatus, onListenerStatus } from '@/shared/api/emulator';
import { useSynced } from '@/shared/hooks/use-synced';
import { useTranslation } from '@/shared/hooks/use-translation';
import type { IListenerStatus } from '@/shared/interfaces/emulator';

const STARTING: IListenerStatus = { state: 'starting' };
/** Shown until the listener reports its port. */
const DEFAULT_PORT = 9100;

export function App() {
  const { t } = useTranslation();
  const status =
    useSynced(onListenerStatus, getListenerStatus, STARTING) ?? STARTING;
  const port = status.state === 'starting' ? DEFAULT_PORT : status.port;

  return (
    <main className="flex h-full flex-col">
      <header className="flex items-start justify-between gap-4 border-b border-border px-6 py-4">
        <div>
          <h1 className="text-lg font-bold text-ink">{t('app.name')}</h1>
          <p className="text-sm text-muted">{t('app.tagline')}</p>
        </div>
        <ListenerStatusBadge status={status} />
      </header>
      <ListenerFailureHint status={status} />
      <ReceiptsScreen port={port} />
    </main>
  );
}
