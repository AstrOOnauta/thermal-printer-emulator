/** macOS writes shortcuts with ⌘ and the others with Ctrl, as each platform's apps do. */
export const IS_MAC =
  typeof navigator !== 'undefined' && navigator.userAgent.includes('Mac');

/** "⌘," on macOS, "Ctrl+," elsewhere. */
export function shortcutLabel(key: string): string {
  return IS_MAC ? `⌘${key}` : `Ctrl+${key}`;
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
