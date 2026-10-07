import type { IBlock } from '@/shared/interfaces/emulator';

/** Font A's cell width: text columns are counted in it, as on the printer (48 on 80 mm). */
const COLUMN_DOTS = 12;
/** A blank line per default line spacing of fed paper. */
const LINE_DOTS = 30;

/**
 * The receipt as plain text, for bug reports and test assertions. Each segment is put at
 * its column (its `x` in font A columns), so right-aligned prices stay right-aligned.
 * Images become `[image W×H]`; barcodes keep their human-readable text, which is a line.
 */
export function receiptText(blocks: IBlock[]): string {
  const lines: string[] = [];
  for (const block of blocks) {
    if (block.type === 'feed') {
      for (
        let line = 0;
        line < Math.floor(block.height / LINE_DOTS);
        line += 1
      ) {
        lines.push('');
      }
    } else if (block.type === 'image') {
      lines.push(`[image ${block.width}×${block.height}]`);
    } else {
      // `ESC *` stripes hang from the line's top: before its text.
      lines.push(
        ...(block.images ?? []).map(
          (image) => `[image ${image.width}×${image.height}]`,
        ),
      );
      if (block.segments.length === 0) continue;
      let text = '';
      for (const segment of [...block.segments].sort((a, b) => a.x - b.x)) {
        const column = Math.round(segment.x / COLUMN_DOTS);
        text = text.padEnd(column, ' ') + segment.text;
      }
      lines.push(text.trimEnd());
    }
  }
  return lines.join('\n').replace(/\n+$/, '');
}
