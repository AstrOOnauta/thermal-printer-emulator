import { useTranslation } from '@/shared/hooks/use-translation';

export function App() {
  const { t } = useTranslation();

  return (
    <main className="flex h-full flex-col items-center justify-center gap-2">
      <h1 className="text-2xl font-bold text-ink">{t('app.name')}</h1>
      <p className="text-muted">{t('app.tagline')}</p>
    </main>
  );
}
