import type { TranslationScope } from '@/shared/hooks/use-translation';

/** The i18n key of a rejected command; anything unexpected maps to `errors.unexpected`. */
export function uiErrorKey(error: unknown): TranslationScope {
  if (
    typeof error === 'object' &&
    error !== null &&
    'key' in error &&
    typeof error.key === 'string'
  ) {
    return error.key as TranslationScope;
  }
  return 'errors.unexpected';
}
