import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

/**
 * Compose conditional Tailwind classes. `clsx` resolves conditionals,
 * `tailwind-merge` dedupes conflicting utilities (last one wins).
 */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
