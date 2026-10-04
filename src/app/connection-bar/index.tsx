import { useEffect, useState } from 'react';

import { useTranslation } from '@/shared/hooks/use-translation';
import { BUTTON } from '@/shared/styles/patterns';

interface IConnectionBarProps {
  /** `ip:port` the POS should print to. */
  address: string;
  /** Which sentence leads the address. */
  kind: 'lan' | 'local' | 'offline';
}

/** "Point your POS at 192.168.1.20:9100", with a copy button. */
export function ConnectionBar({ address, kind }: IConnectionBarProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => setCopied(false), 1500);
    return () => clearTimeout(timer);
  }, [copied]);

  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-2 border-b border-border px-6 py-3 text-sm text-muted">
      <span>{t(`connection.${kind}`)}</span>
      <code className="rounded bg-ink/5 px-2 py-0.5 font-mono text-ink select-text">
        {address}
      </code>
      <button
        type="button"
        className={BUTTON}
        onClick={() => {
          navigator.clipboard
            .writeText(address)
            .then(() => setCopied(true))
            .catch(() => {
              // Clipboard refused: the address stays selectable.
            });
        }}
      >
        <span role="status">
          {copied ? t('connection.copied') : t('connection.copy')}
        </span>
      </button>
    </div>
  );
}
