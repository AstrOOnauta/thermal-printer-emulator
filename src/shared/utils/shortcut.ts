/** macOS writes shortcuts with ⌘ and the others with Ctrl, as each platform's apps do. */
export const IS_MAC =
  typeof navigator !== 'undefined' && navigator.userAgent.includes('Mac');

/** Key names where the symbol is macOS-only or not a key value. */
const KEY_NAMES: Record<string, string> = { '⌫': 'Backspace', '−': '-' };

/** "⌘," on macOS, "Ctrl+," elsewhere ("Ctrl+Backspace": ⌫ is a Mac glyph). */
export function shortcutLabel(key: string): string {
  return IS_MAC ? `⌘${key}` : `Ctrl+${key === '⌫' ? 'Backspace' : key}`;
}

/** The `aria-keyshortcuts` value: "Meta+," on macOS, "Control+," elsewhere. */
export function shortcutAria(key: string): string {
  return `${IS_MAC ? 'Meta' : 'Control'}+${KEY_NAMES[key] ?? key}`;
}

/** ⌘ on macOS, Ctrl elsewhere, with no Alt (that is a different shortcut). */
export function hasCommandKey(event: KeyboardEvent): boolean {
  return (IS_MAC ? event.metaKey : event.ctrlKey) && !event.altKey;
}

/** True when typing in a field: shortcuts like ⌘⌫ belong to the field there. */
export function isTyping(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable ||
      ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName))
  );
}
