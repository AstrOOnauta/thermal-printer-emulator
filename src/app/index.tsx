import { JobsScreen } from '@/screens/jobs';
import { useTranslation } from '@/shared/hooks/use-translation';

export function App() {
  const { t } = useTranslation();

  return (
    <main className="flex h-full flex-col">
      <header className="border-b border-border px-6 py-4">
        <h1 className="text-lg font-bold text-ink">{t('app.name')}</h1>
        <p className="text-sm text-muted">{t('app.tagline')}</p>
      </header>
      <JobsScreen />
    </main>
  );
}
