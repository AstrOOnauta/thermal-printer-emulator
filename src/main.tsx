import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { App } from '@/app';
import { initLocale } from '@/shared/hooks/use-translation';

import '@/shared/styles/globals.css';

const rootElement = document.getElementById('root');
if (!rootElement) throw new Error('#root missing from index.html');

// The locale comes from Rust; resolve it before the first render so text never flips.
void initLocale().then(() => {
  createRoot(rootElement).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
});
