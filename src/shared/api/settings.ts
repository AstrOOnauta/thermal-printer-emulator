import { invoke } from '@tauri-apps/api/core';

import type { ISettings } from '@/shared/interfaces/emulator';

export function getSettings(): Promise<ISettings> {
  return invoke<ISettings>('get_settings');
}

/** Saves and applies; rejects with an `IUiError` when a value is refused. */
export function setSettings(settings: ISettings): Promise<ISettings> {
  return invoke<ISettings>('set_settings', { settings });
}
