import { expect, test } from 'vitest';

import { formatBytes, peerHost } from '@/shared/utils/format';

test('formats bytes in B, KB and MB', () => {
  expect(formatBytes(0, 'en')).toBe('0 B');
  expect(formatBytes(1023, 'en')).toBe('1,023 B');
  expect(formatBytes(1536, 'en')).toBe('1.5 KB');
  expect(formatBytes(16 * 1024 * 1024, 'en')).toBe('16 MB');
  expect(formatBytes(5 * 1024 ** 3, 'en')).toBe('5,120 MB');
});

test('uses the locale decimal separator', () => {
  expect(formatBytes(1536, 'pt-BR')).toBe('1,5 KB');
});

test('drops the client port', () => {
  expect(peerHost('192.168.1.10:50000')).toBe('192.168.1.10');
  expect(peerHost('[::1]:50000')).toBe('::1');
  expect(peerHost('[fe80::1%en0]:9')).toBe('fe80::1%en0');
});
