import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type {
  IListenerStatus,
  IReceiptSummary,
  IReceiptView,
} from '@/shared/interfaces/emulator';

/** Printed receipts, oldest first. */
export function getReceipts(): Promise<IReceiptSummary[]> {
  return invoke<IReceiptSummary[]>('get_receipts');
}

/** One receipt with its print model; `null` once dropped from memory. */
export function getReceipt(id: number): Promise<IReceiptView | null> {
  return invoke<IReceiptView | null>('get_receipt', { id });
}

/** The whole list again, whenever a receipt starts or ends. */
export function onReceipts(
  handler: (receipts: IReceiptSummary[]) => void,
): Promise<UnlistenFn> {
  return listen<IReceiptSummary[]>('receipts', (event) =>
    handler(event.payload),
  );
}

/** Whether the emulator is listening, and on which port. */
export function getListenerStatus(): Promise<IListenerStatus> {
  return invoke<IListenerStatus>('get_listener_status');
}

/** Fires when the listener starts, fails to bind or recovers. */
export function onListenerStatus(
  handler: (status: IListenerStatus) => void,
): Promise<UnlistenFn> {
  return listen<IListenerStatus>('listener_status', (event) =>
    handler(event.payload),
  );
}
