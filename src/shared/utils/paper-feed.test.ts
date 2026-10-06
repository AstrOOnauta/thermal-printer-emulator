import { expect, test } from 'vitest';

import {
  easeOutCubic,
  FEED_MAX_MS,
  FEED_MIN_MS,
  feedDuration,
} from '@/shared/utils/paper-feed';

test('feeds at about 1200 dots per second, clamped', () => {
  expect(feedDuration(1200)).toBe(1000);
  expect(feedDuration(600)).toBe(500);
  expect(feedDuration(10)).toBe(FEED_MIN_MS);
  expect(feedDuration(100_000)).toBe(FEED_MAX_MS);
});

test('the curve starts at 0, ends at 1 and only moves forward', () => {
  expect(easeOutCubic(0)).toBe(0);
  expect(easeOutCubic(1)).toBe(1);
  let previous = 0;
  for (let step = 1; step <= 20; step += 1) {
    const value = easeOutCubic(step / 20);
    expect(value).toBeGreaterThan(previous);
    previous = value;
  }
});
