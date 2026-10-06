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
  expect(translate('receipts.empty')).toBe('Waiting for receipts');
  expect(setLocale('pt-BR')).toBe('pt-BR');
  expect(translate('receipts.empty')).toBe('Aguardando cupons');
  expect(setLocale('es')).toBe('es');
  expect(translate('receipts.empty')).toBe('Esperando recibos');
});

test('interpolates params', () => {
  expect(translate('receipts.emptyHint', { address: '10.0.0.2:9100' })).toBe(
    'Send ESC/POS jobs to 10.0.0.2:9100.',
  );
});

test('unknown locale falls back to English', () => {
  expect(setLocale('fr')).toBe('en');
  expect(translate('receipts.empty')).toBe('Waiting for receipts');
});

test('unknown key renders as the key itself', () => {
  expect(translate('receipts.missing' as TranslationScope)).toBe(
    'receipts.missing',
  );
});
