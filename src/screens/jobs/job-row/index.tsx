import { getLocale, useTranslation } from '@/shared/hooks/use-translation';
import type { IJobState, IJobSummary } from '@/shared/interfaces/emulator';
import { cn } from '@/shared/styles/cn';
import { formatBytes, peerHost } from '@/shared/utils/format';

const STATE_DOT: Record<IJobState, string> = {
  receiving: 'bg-accent',
  done: 'bg-success',
  idle_timeout: 'bg-warning',
  too_large: 'bg-error',
  connection_error: 'bg-error',
};

interface IJobRowProps {
  job: IJobSummary;
}

export function JobRow({ job }: IJobRowProps) {
  const { t } = useTranslation();
  const locale = getLocale();
  const startedAt = new Date(job.started_at);

  return (
    <li className="flex items-center gap-4 px-4 py-3 text-sm">
      <time
        dateTime={startedAt.toISOString()}
        className="text-ink tabular-nums"
      >
        {startedAt.toLocaleTimeString(locale)}
      </time>
      <span className="flex-1 truncate text-muted">
        {t('jobs.from', { host: peerHost(job.peer) })}
      </span>
      {/* No progress events while receiving: the size is only final at the end. */}
      <span className="text-muted tabular-nums">
        {job.state === 'receiving' ? '—' : formatBytes(job.size, locale)}
      </span>
      <span className="flex w-36 items-center justify-end gap-2 text-ink">
        <span
          aria-hidden
          className={cn('size-2 rounded-full', STATE_DOT[job.state])}
        />
        {t(`jobs.state.${job.state}`)}
      </span>
    </li>
  );
}
