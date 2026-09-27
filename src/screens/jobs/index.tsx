import { JobRow } from '@/screens/jobs/job-row';
import { getJobs, onJobs } from '@/shared/api/emulator';
import { useSynced } from '@/shared/hooks/use-synced';
import { useTranslation } from '@/shared/hooks/use-translation';
import type { IJobSummary } from '@/shared/interfaces/emulator';

const NO_JOBS: IJobSummary[] = [];

interface IJobsScreenProps {
  /** For the empty-state hint. */
  port: number;
}

// ponytail: P1 screen, a plain list of jobs. P2 replaces it with the rendered receipts.
export function JobsScreen({ port }: IJobsScreenProps) {
  const { t } = useTranslation();
  const jobs = useSynced(onJobs, getJobs, NO_JOBS);

  if (jobs === null) return null;

  if (jobs.length === 0) {
    return (
      <div
        role="status"
        className="flex flex-1 flex-col items-center justify-center gap-1 px-6 text-center"
      >
        <p className="font-medium text-ink">{t('jobs.empty')}</p>
        <p className="text-sm text-muted">{t('jobs.emptyHint', { port })}</p>
      </div>
    );
  }

  return (
    <section
      aria-labelledby="jobs-title"
      className="flex-1 overflow-y-auto px-6 py-4"
    >
      <h2 id="jobs-title" className="mb-3 text-sm font-semibold text-muted">
        {t('jobs.title')}
      </h2>
      <ol className="divide-y divide-border rounded-lg border border-border">
        {[...jobs].reverse().map((job) => (
          <JobRow key={job.id} job={job} />
        ))}
      </ol>
    </section>
  );
}
