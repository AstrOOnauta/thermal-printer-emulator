import { invoke } from '@tauri-apps/api/core';

/** The UI language Rust resolved from the OS: `en`, `es` or `pt-BR`. */
export function getAppLocale(): Promise<string> {
  return invoke<string>('app_locale');
}
