import { invoke } from '@tauri-apps/api/core';

/** The UI language Rust resolved from the OS: `en`, `es` or `pt-BR`. */
export function getAppLocale(): Promise<string> {
  return invoke<string>('app_locale');
}

/** The count of receipts that arrived while the window was not in front; 0 clears it. */
export function setUnseen(count: number): Promise<void> {
  return invoke<void>('set_unseen', { count });
}
