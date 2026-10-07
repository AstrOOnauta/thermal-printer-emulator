import { expect, test } from 'vitest';

import type { IBlock, ISegment } from '@/shared/interfaces/emulator';
import { receiptText } from '@/shared/utils/receipt-text';

const segment = (x: number, text: string): ISegment => ({
  x,
  text,
  font: 'a',
  width: 1,
  height: 1,
  advance: 12,
  bold: false,
  underline: 0,
  reverse: false,
});

const line = (...segments: ISegment[]): IBlock => ({
  type: 'line',
  height: 30,
  ascent: 24,
  segments,
});

test('puts segments at their columns and keeps right alignment', () => {
  const text = receiptText([
    line(segment(264, 'CAFE')),
    line(segment(0, 'Total'), segment(492, 'R$ 7,50')),
  ]);
  expect(text).toBe(`${' '.repeat(22)}CAFE\nTotal${' '.repeat(36)}R$ 7,50`);
});

test('feeds become blank lines, images a placeholder, trailing blanks dropped', () => {
  const text = receiptText([
    line(segment(0, 'A')),
    { type: 'feed', height: 60 },
    { type: 'image', x: 0, width: 96, height: 32, data: '' },
    line(segment(0, 'B')),
    { type: 'feed', height: 90 },
  ]);
  expect(text).toBe('A\n\n\n[image 96×32]\nB');
});

test('a line with only column-image stripes shows them as images', () => {
  const text = receiptText([
    {
      type: 'line',
      height: 24,
      ascent: 0,
      segments: [],
      images: [{ x: 0, width: 120, height: 24, data: '' }],
    },
  ]);
  expect(text).toBe('[image 120×24]');
});
