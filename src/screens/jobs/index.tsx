import { JobRow } from '@/screens/jobs/job-row';
import { useJobs } from '@/shared/hooks/use-jobs';
import { useTranslation } from '@/shared/hooks/use-translation';

// ponytail: P1 screen, a plain list of jobs. P2 replaces it with the rendered receipts.
const PORT = 9100;

export function JobsScreen() {
  const { t } = useTranslation();
  const jobs = useJobs();

  if (jobs === null) return null;

  if (jobs.length === 0) {
    return (
      <div
        role="status"
        className="flex flex-1 flex-col items-center justify-center gap-1 px-6 text-center"
      >
        <p className="font-medium text-ink">{t('jobs.empty')}</p>
        <p className="text-sm text-muted">
          {t('jobs.emptyHint', { port: PORT })}
        </p>
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
