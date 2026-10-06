import { useTranslation } from '@/shared/hooks/use-translation';
import type { IListenerStatus } from '@/shared/interfaces/emulator';

interface IListenerStatusProps {
  status: IListenerStatus;
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
