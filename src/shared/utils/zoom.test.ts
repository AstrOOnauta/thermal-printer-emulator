import { expect, test } from 'vitest';

import { stepZoom } from '@/shared/utils/zoom';

test('steps through 75, 100, 125, 150, 200 and stops at the ends', () => {
  expect(stepZoom(100, 1)).toBe(125);
  expect(stepZoom(125, -1)).toBe(100);
  expect(stepZoom(200, 1)).toBe(200);
  expect(stepZoom(75, -1)).toBe(75);
});
