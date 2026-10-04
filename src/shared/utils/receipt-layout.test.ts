import { expect, test } from 'vitest';

import type { IBlock } from '@/shared/interfaces/emulator';
import {
  blocksIn,
  decodeBase64,
  isBlack,
  positionBlocks,
  SLICE_HEIGHT,
  slices,
} from '@/shared/utils/receipt-layout';

const feed = (height: number): IBlock => ({ type: 'feed', height });

test('stacks blocks from the top of the paper', () => {
  const { positioned, height } = positionBlocks([feed(30), feed(10), feed(5)]);
  expect(positioned.map(({ y }) => y)).toEqual([0, 30, 40]);
  expect(height).toBe(45);
});

test('slices tall receipts and finds the blocks in each slice', () => {
  expect(slices(0)).toEqual([]);
  expect(slices(100)).toEqual([[0, 100]]);
  expect(slices(SLICE_HEIGHT + 1)).toEqual([
    [0, SLICE_HEIGHT],
    [SLICE_HEIGHT, SLICE_HEIGHT + 1],
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
