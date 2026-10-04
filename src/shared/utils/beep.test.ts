import { expect, test } from 'vitest';

import type { IReceiptSummary } from '@/shared/interfaces/emulator';
import { pendingBeeps } from '@/shared/utils/beep';

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

test('the first look rings nothing: those receipts were already there', () => {
  const first = pendingBeeps(null, [receipt(1, 2)]);
  expect(first.beeps).toBe(0);
  expect([...first.seen]).toEqual([1]);
});

test('rings once per new receipt that finished with beeps, at most 3', () => {
  const { seen } = pendingBeeps(null, [receipt(1, 1)]);
  const next = pendingBeeps(seen, [
    receipt(1, 1),
    receipt(2, 9),
    receipt(3, 0),
  ]);
  expect(next.beeps).toBe(3);
  expect(pendingBeeps(next.seen, [receipt(2, 9)]).beeps).toBe(0);
});

test('waits until a receipt stops printing', () => {
  const { seen } = pendingBeeps(null, []);
  const printing = pendingBeeps(seen, [receipt(4, 1, 'printing')]);
  expect(printing.beeps).toBe(0);
  expect(pendingBeeps(printing.seen, [receipt(4, 1)]).beeps).toBe(1);
});
