import { afterEach, expect, test } from 'vitest';

import {
  setLocale,
  translate,
  type TranslationScope,
} from '@/shared/hooks/use-translation';

afterEach(() => {
  setLocale('en');
});

test('resolves keys in the active locale', () => {
  expect(translate('app.tagline')).toBe('Virtual thermal printer for ESC/POS');
  expect(setLocale('pt-BR')).toBe('pt-BR');
  expect(translate('app.tagline')).toBe(
    'Impressora térmica virtual para ESC/POS',
  );
  expect(setLocale('es')).toBe('es');
  expect(translate('app.tagline')).toBe(
    'Impresora térmica virtual para ESC/POS',
  );
});

test('unknown locale falls back to English', () => {
  expect(setLocale('fr')).toBe('en');
  expect(translate('app.tagline')).toBe('Virtual thermal printer for ESC/POS');
});

test('unknown key renders as the key itself', () => {
  expect(translate('app.missing' as TranslationScope)).toBe('app.missing');
});
