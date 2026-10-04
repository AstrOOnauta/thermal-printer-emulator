import type { IBlock } from '@/shared/interfaces/emulator';

/** WebKit and Chromium cap canvas size; tall receipts are drawn in slices this tall. */
export const SLICE_HEIGHT = 4096;

export interface IPositionedBlock {
  /** Dots from the top of the receipt. */
  y: number;
  block: IBlock;
}

/** Each block's top, stacked from the top of the paper. */
export function positionBlocks(blocks: IBlock[]): {
  positioned: IPositionedBlock[];
  height: number;
} {
  let y = 0;
  const positioned = blocks.map((block) => {
    const top = y;
    y += block.height;
    return { y: top, block };
  });
  return { positioned, height: y };
}

/** `[top, bottom)` ranges of at most `SLICE_HEIGHT` covering `height`. */
export function slices(height: number): [number, number][] {
  const ranges: [number, number][] = [];
  for (let top = 0; top < height; top += SLICE_HEIGHT) {
    ranges.push([top, Math.min(top + SLICE_HEIGHT, height)]);
  }
  return ranges;
}

/** Blocks that overlap `[top, bottom)`. */
export function blocksIn(
  positioned: IPositionedBlock[],
  top: number,
  bottom: number,
): IPositionedBlock[] {
  return positioned.filter(
    ({ y, block }) => y < bottom && y + block.height > top,
  );
}

/** Base64 → bytes. */
export function decodeBase64(data: string): Uint8Array {
  const binary = atob(data);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

/** Whether dot (x, y) is black in 1-bit rows of `ceil(width / 8)` bytes, MSB leftmost. */
export function isBlack(
  bits: Uint8Array,
  width: number,
  x: number,
  y: number,
): boolean {
  const stride = Math.ceil(width / 8);
  const byte = bits[y * stride + (x >> 3)] ?? 0;
  return (byte & (0x80 >> (x & 7))) !== 0;
}
