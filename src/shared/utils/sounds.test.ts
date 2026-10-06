import { expect, test } from 'vitest';

import type { IReceiptSummary } from '@/shared/interfaces/emulator';
import { pendingSounds } from '@/shared/utils/sounds';

const receipt = (
  id: number,
  beeps: number,
  state: IReceiptSummary['state'] = 'done',
): IReceiptSummary => ({
  id,
  peer: '127.0.0.1:1',
  started_at: 0,
  ended_at: 0,
  state,
  cut: null,
  drawer: false,
  beeps,
  size: 1,
  paper: 'mm80',
  width: 576,
  height: 30,
});

test('the first look plays nothing: those receipts were already there', () => {
  const first = pendingSounds(null, [receipt(1, 2)]);
  expect(first.printed).toEqual([]);
  expect(first.beeps).toBe(0);
  expect([...first.seen]).toEqual([1]);
});

test('each new finished receipt prints once; beeps capped at 3', () => {
  const { seen } = pendingSounds(null, [receipt(1, 1)]);
  const next = pendingSounds(seen, [
    receipt(1, 1),
    receipt(2, 9),
    receipt(3, 0),
  ]);
  expect(next.printed.map(({ id }) => id)).toEqual([2, 3]);
  expect(next.beeps).toBe(3);
  expect(pendingSounds(next.seen, [receipt(2, 9)]).printed).toEqual([]);
});

test('waits until a receipt stops printing', () => {
  const { seen } = pendingSounds(null, []);
  const printing = pendingSounds(seen, [receipt(4, 1, 'printing')]);
  expect(printing.printed).toEqual([]);
  const done = pendingSounds(printing.seen, [receipt(4, 1)]);
  expect(done.printed.map(({ id }) => id)).toEqual([4]);
  expect(done.beeps).toBe(1);
});
