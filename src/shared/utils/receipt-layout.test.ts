import { expect, test } from 'vitest';

import type { IBlock } from '@/shared/interfaces/emulator';
import {
  blocksIn,
  decodeBase64,
  isBlack,
  positionBlocks,
  sliceHeight,
  slices,
  visibleRows,
} from '@/shared/utils/receipt-layout';

const feed = (height: number): IBlock => ({ type: 'feed', height });

test('stacks blocks from the top of the paper', () => {
  const { positioned, height } = positionBlocks([feed(30), feed(10), feed(5)]);
  expect(positioned.map(({ y }) => y)).toEqual([0, 30, 40]);
  expect(height).toBe(45);
});

test('slices tall receipts and finds the blocks in each slice', () => {
  expect(slices(0, 4096)).toEqual([]);
  expect(slices(100, 4096)).toEqual([[0, 100]]);
  expect(slices(4097, 4096)).toEqual([
    [0, 4096],
    [4096, 4097],
  ]);
  const { positioned } = positionBlocks([feed(30), feed(30), feed(30)]);
  expect(blocksIn(positioned, 40, 60).map(({ y }) => y)).toEqual([30]);
  expect(blocksIn(positioned, 30, 31).map(({ y }) => y)).toEqual([30]);
  expect(blocksIn(positioned, 25, 35).map(({ y }) => y)).toEqual([0, 30]);
});

test('reads 1-bit rows', () => {
  // 10 dots wide: 2 bytes per row. Row 0: dots 0 and 9; row 1: dot 1.
  const bits = decodeBase64(btoa(String.fromCharCode(0x80, 0x40, 0x40, 0x00)));
  expect(isBlack(bits, 10, 0, 0)).toBe(true);
  expect(isBlack(bits, 10, 9, 0)).toBe(true);
  expect(isBlack(bits, 10, 1, 0)).toBe(false);
  expect(isBlack(bits, 10, 1, 1)).toBe(true);
  expect(isBlack(bits, 10, 5, 9)).toBe(false);
});

test('keeps each slice canvas at most 4096 device pixels tall', () => {
  expect(sliceHeight(1)).toBe(4096);
  expect(sliceHeight(2)).toBe(2048);
  expect(sliceHeight(4)).toBe(1024);
  expect(sliceHeight(100)).toBe(256);
});

test('decodes only the image rows inside a slice', () => {
  // An image 100 rows tall at y = 50.
  expect(visibleRows(50, 100, 0, 4096)).toEqual([0, 100]);
  expect(visibleRows(50, 100, 100, 120)).toEqual([50, 70]);
  expect(visibleRows(50, 100, 120, 4096)).toEqual([70, 100]);
  const [from, to] = visibleRows(50, 100, 200, 300);
  expect(from >= to).toBe(true);
});
