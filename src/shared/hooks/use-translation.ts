import { getAppLocale } from '@/shared/api/app';
import en, { type TranslationKeys } from '@/shared/translations/en';
import es from '@/shared/translations/es';
import ptBR from '@/shared/translations/pt-BR';

type DotNotation<T, Prefix extends string = ''> = {
  [K in keyof T]: T[K] extends object
    ? DotNotation<T[K], `${Prefix}${Prefix extends '' ? '' : '.'}${string & K}`>
    : `${Prefix}${Prefix extends '' ? '' : '.'}${string & K}`;
}[keyof T];

export type TranslationScope = DotNotation<TranslationKeys>;

/** Keyed by the tags Rust's `app_locale` returns. */
const LOCALES: Record<string, TranslationKeys> = { en, es, 'pt-BR': ptBR };

let messages: TranslationKeys = en;
let currentTag = 'en';

/** Switches the active locale and returns the tag in effect: unknown tags fall back to English. */
export function setLocale(tag: string): string {
  const found = LOCALES[tag];
  messages = found ?? en;
  currentTag = found ? tag : 'en';
  return currentTag;
}

/** The active locale tag, for `Intl` formatting. */
export function getLocale(): string {
  return currentTag;
}

/**
 * Asks Rust for the OS language, so the webview matches the native tray menu.
 * Outside Tauri (`yarn dev` in a browser) there is no IPC: stays English.
 */
export async function initLocale(): Promise<void> {
  let tag = 'en';
  try {
    tag = await getAppLocale();
  } catch {
    // Not running inside Tauri.
  }
  document.documentElement.lang = setLocale(tag);
}

/** Resolves a key; an unknown key renders as the key itself. */
export function translate(
  scope: TranslationScope,
  params?: Record<string, string | number>,
): string {
  const resolved = scope
    .split('.')
    .reduce<unknown>(
      (node, part) =>
        node && typeof node === 'object'
          ? (node as Record<string, unknown>)[part]
          : undefined,
      messages,
    );
  const template = typeof resolved === 'string' ? resolved : scope;
  return template.replace(/%\{(\w+)\}/g, (match, name: string) =>
    params?.[name] != null ? String(params[name]) : match,
  );
}

// ponytail: no store; a language change re-renders the whole tree from `App`
// (`setLocaleTag`). Add one if a part of the UI must follow the language on its own. It keeps
// the hook name so that change stays local.
// eslint-disable-next-line @eslint-react/no-unnecessary-use-prefix
export function useTranslation() {
  return { t: translate };
}
