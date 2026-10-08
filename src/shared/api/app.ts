import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type { IUpdateInfo } from '@/shared/interfaces/emulator';

/** The UI language Rust resolved from the OS: `en`, `es` or `pt-BR`. */
export function getAppLocale(): Promise<string> {
  return invoke<string>('app_locale');
}

/** The count of receipts that arrived while the window was not in front; 0 clears it. */
export function setUnseen(count: number): Promise<void> {
  return invoke<void>('set_unseen', { count });
}

/** Quits the app (Ctrl+Q off macOS): the listener stops and the port is free. */
export function quitApp(): Promise<void> {
  return invoke<void>('quit_app');
}

/** Opens the project's GitHub page in the browser (a fixed URL, chosen by Rust). */
export function openRepository(): Promise<void> {
  return invoke<void>('open_repository');
}

/** The newer version the last daily check found; `null` when up to date. */
export function getUpdate(): Promise<IUpdateInfo | null> {
  return invoke<IUpdateInfo | null>('get_update');
}

/** Fires when a check finds a newer version. */
export function onUpdate(
  handler: (update: IUpdateInfo) => void,
): Promise<UnlistenFn> {
  return listen<IUpdateInfo>('update_available', (event) =>
    handler(event.payload),
  );
}

/** Downloads and installs the update; the app restarts into it. */
export function installUpdate(): Promise<void> {
  return invoke<void>('install_update');
}

/** Opens the newest release on GitHub (how a .deb or .rpm updates). */
export function openReleasePage(): Promise<void> {
  return invoke<void>('open_release_page');
}
