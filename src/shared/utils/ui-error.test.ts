import { expect, test } from 'vitest';

import { uiErrorKey } from '@/shared/utils/ui-error';

test('reads the key Rust sends, falls back otherwise', () => {
  expect(uiErrorKey({ key: 'settings.errors.port' })).toBe(
    'settings.errors.port',
  );
  expect(uiErrorKey('boom')).toBe('errors.unexpected');
  expect(uiErrorKey(null)).toBe('errors.unexpected');
  expect(uiErrorKey({ key: 3 })).toBe('errors.unexpected');
});
