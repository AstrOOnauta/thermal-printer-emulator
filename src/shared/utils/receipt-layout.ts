import type { IBlock } from '@/shared/interfaces/emulator';

/**
 * Device pixels in one slice's canvas, at most: WebKit and Chromium cap canvas size, and a
 * canvas costs 4 bytes a pixel.
 */
const SLICE_PIXELS = 4096;

/**
 * Taller than any real receipt (25 m of paper): past it the rest is not drawn, so a hostile
 * job can't make the window create thousands of canvases.
 */
export const MAX_DRAWN_HEIGHT = 200_000;

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

/** Dots per slice at `pixels` device pixels per dot (pixel ratio × zoom). */
export function sliceHeight(pixels: number): number {
  return Math.max(256, Math.floor(SLICE_PIXELS / pixels));
}

/** `[top, bottom)` ranges of at most `size` covering `height`. */
export function slices(height: number, size: number): [number, number][] {
  const ranges: [number, number][] = [];
  for (let top = 0; top < height; top += size) {
    ranges.push([top, Math.min(top + size, height)]);
  }
  return ranges;
}

/**
 * The rows `[from, to)` of an image at `top`, `height` rows tall, that fall in the slice
 * `[sliceTop, sliceBottom)`: only those are decoded and drawn. Empty when `from >= to`.
 */
export function visibleRows(
  top: number,
  height: number,
  sliceTop: number,
  sliceBottom: number,
): [number, number] {
  return [Math.max(0, sliceTop - top), Math.min(height, sliceBottom - top)];
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
