import { useTranslation } from '@/shared/hooks/use-translation';
import type { IListenerStatus } from '@/shared/interfaces/emulator';
import { cn } from '@/shared/styles/cn';

interface IListenerStatusProps {
  status: IListenerStatus;
}

/** Header badge: a colored dot plus text, never color alone. */
export function ListenerStatusBadge({ status }: IListenerStatusProps) {
  const { t } = useTranslation();

  const label =
    status.state === 'starting'
      ? t('listener.starting')
      : status.state === 'listening'
        ? t('listener.listening', { port: status.port })
        : t(`listener.failed.${status.error}`, { port: status.port });

  return (
    <p role="status" className="flex items-center gap-2 text-sm text-ink">
      <span
        aria-hidden
        className={cn(
          'size-2 rounded-full',
          status.state === 'starting' && 'bg-muted',
          status.state === 'listening' && 'bg-success',
          status.state === 'failed' && 'bg-error',
        )}
      />
      {label}
    </p>
  );
}

/** Under the header while the port can't be opened: what to do about it. */
export function ListenerFailureHint({ status }: IListenerStatusProps) {
  const { t } = useTranslation();
  if (status.state !== 'failed') return null;

  return (
    <p
      role="alert"
      className="border-b border-border bg-error/10 px-6 py-3 text-sm text-ink"
    >
      {t(`listener.hint.${status.error}`)}
    </p>
  );
}
