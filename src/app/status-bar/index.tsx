import { useEffect, useState } from 'react';

import { useTranslation } from '@/shared/hooks/use-translation';
import type { IListenerStatus } from '@/shared/interfaces/emulator';
import { cn } from '@/shared/styles/cn';
import { INTERACTIVE } from '@/shared/styles/patterns';

interface IStatusBarProps {
  status: IListenerStatus;
  /** `ip:port` to print to, once listening. */
  address: string | null;
  /** Which sentence leads the address. */
  kind: 'lan' | 'local' | 'offline';
}

/**
 * Whether and where the emulator listens, in one line: "● Point your POS at
 * 192.168.1.20:9100 · Copy", or "● Port 9100 is in use". A colored dot plus text,
 * never color alone.
 */
export function StatusBar({ status, address, kind }: IStatusBarProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => setCopied(false), 1500);
    return () => clearTimeout(timer);
  }, [copied]);

  return (
    <div
      role="status"
      className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-sm"
    >
      <span
        aria-hidden
        className={cn(
          'size-2 shrink-0 rounded-full',
          status.state === 'starting' && 'bg-muted',
          status.state === 'listening' && 'bg-success',
          status.state === 'failed' && 'bg-error',
        )}
      />
      {status.state === 'listening' && address ? (
        <>
          <span className="text-muted">{t(`connection.${kind}`)}</span>
          <code className="font-mono text-ink select-text">{address}</code>
          <button
            type="button"
            className={`rounded px-1.5 py-0.5 text-xs text-muted hover:bg-ink/5 hover:text-ink ${INTERACTIVE}`}
            onClick={() => {
              navigator.clipboard
                .writeText(address)
                .then(() => setCopied(true))
                .catch(() => {
                  // Clipboard refused: the address stays selectable.
                });
            }}
          >
            {copied ? t('connection.copied') : t('connection.copy')}
          </button>
        </>
      ) : (
        <span className="text-ink">
          {status.state === 'failed'
            ? t(`listener.failed.${status.error}`, { port: status.port })
            : t('listener.starting')}
        </span>
      )}
    </div>
  );
}
