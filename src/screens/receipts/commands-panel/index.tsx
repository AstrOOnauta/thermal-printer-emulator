import { useEffect, useRef, useState } from 'react';

import { getReceiptCommands } from '@/shared/api/emulator';
import {
  type TranslationScope,
  useTranslation,
} from '@/shared/hooks/use-translation';
import type { IInspection } from '@/shared/interfaces/emulator';
import { prefersReducedMotion } from '@/shared/utils/paper-feed';

interface ICommandsPanelProps {
  id: number;
  /** Matches the paper's width. */
  width: number;
}

/**
 * What the POS sent and how the emulator read it: one row per ESC/POS command, with its
 * offset, mnemonic, meaning and bytes. Re-parsed by Rust when opened.
 */
export function CommandsPanel({ id, width }: ICommandsPanelProps) {
  const { t } = useTranslation();
  // `undefined` while loading, `null` once Rust dropped the receipt.
  const [inspection, setInspection] = useState<IInspection | null>();
  const panel = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let active = true;
    getReceiptCommands(id)
      .then((next) => active && setInspection(next))
      .catch(() => active && setInspection(null));
    return () => {
      active = false;
    };
  }, [id]);

  // Opened below the paper, often off-screen: bring the table into view, just enough.
  useEffect(() => {
    panel.current?.scrollIntoView({
      block: 'nearest',
      behavior: prefersReducedMotion() ? 'auto' : 'smooth',
    });
  }, [inspection]);

  if (inspection === undefined) {
    return <p className="text-xs text-muted">{t('inspect.loading')}</p>;
  }
  if (inspection === null) {
    return <p className="text-xs text-muted">{t('receipts.unavailable')}</p>;
  }

  return (
    <div
      ref={panel}
      className="max-h-96 overflow-auto rounded-md border border-border select-text"
      style={{ width }}
    >
      <table className="w-full border-collapse font-mono text-xs">
        <thead className="sticky top-0 bg-raised text-left text-muted">
          <tr>
            <th className="px-2 py-1.5 font-medium">{t('inspect.offset')}</th>
            <th className="px-2 py-1.5 font-medium">{t('inspect.command')}</th>
            <th className="px-2 py-1.5 font-medium">{t('inspect.meaning')}</th>
            <th className="px-2 py-1.5 font-medium">{t('inspect.bytes')}</th>
          </tr>
        </thead>
        <tbody>
          {inspection.rows.map((row) => (
            <tr key={row.offset} className="border-t border-border align-top">
              <td className="px-2 py-1 text-muted tabular-nums">
                {row.offset.toString(16).padStart(4, '0')}
              </td>
              <td className="px-2 py-1 whitespace-nowrap text-ink">
                {row.mnemonic}
              </td>
              <td className="px-2 py-1 text-ink">
                <span className="font-sans">
                  {t(`inspect.kinds.${row.kind}` as TranslationScope)}
                </span>
                {row.detail && (
                  <span className="ml-2 break-all text-muted">
                    {row.kind === 'text' ? `“${row.detail}”` : row.detail}
                  </span>
                )}
              </td>
              <td
                className="max-w-36 truncate px-2 py-1 text-muted"
                title={row.bytes}
              >
                {row.bytes}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {inspection.truncated && (
        <p className="border-t border-border px-2 py-1.5 font-sans text-xs text-muted">
          {t('inspect.truncated', { count: inspection.rows.length })}
        </p>
      )}
    </div>
  );
}
