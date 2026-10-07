import type {
  IBlock,
  IFont,
  IPlaced,
  ISegment,
} from '@/shared/interfaces/emulator';
import {
  blocksIn,
  decodeBase64,
  type IPositionedBlock,
  isBlack,
  visibleRows,
} from '@/shared/utils/receipt-layout';

/** Character cells in dots, as in `printer::Font::cell`. */
const CELL: Record<IFont, { width: number; height: number }> = {
  a: { width: 12, height: 24 },
  b: { width: 9, height: 17 },
};

/** The paper is white in both themes, so its ink is always black. */
const INK = '#000000';
const PAPER = '#ffffff';
const MONOSPACE =
  'ui-monospace, "SF Mono", Menlo, Consolas, "Liberation Mono", "DejaVu Sans Mono", monospace';

/** Advance of one glyph of `font` at its natural size, measured once. */
const advances = new Map<string, number>();

function fontFor(segment: ISegment): string {
  return `${segment.bold ? 700 : 400} ${CELL[segment.font].height}px ${MONOSPACE}`;
}

function advanceOf(context: CanvasRenderingContext2D, font: string): number {
  let advance = advances.get(font);
  if (advance === undefined) {
    context.font = font;
    advance = context.measureText('M').width || 1;
    advances.set(font, advance);
  }
  return advance;
}

/**
 * Draws one character per cell: the system monospace glyph is stretched to the printer's
 * cell (`cell × width/height` dots), so columns line up whatever font the OS has.
 */
function drawSegment(
  context: CanvasRenderingContext2D,
  segment: ISegment,
  baseline: number,
) {
  const cell = CELL[segment.font];
  const glyphWidth = cell.width * segment.width;
  const glyphHeight = cell.height * segment.height;
  const font = fontFor(segment);
  const scaleX = glyphWidth / advanceOf(context, font);

  context.font = font;
  context.textBaseline = 'bottom';
  [...segment.text].forEach((character, index) => {
    const x = segment.x + index * segment.advance;
    if (segment.reverse) {
      context.fillStyle = INK;
      context.fillRect(x, baseline - glyphHeight, segment.advance, glyphHeight);
    }
    context.fillStyle = segment.reverse ? PAPER : INK;
    context.save();
    context.translate(x, baseline);
    context.scale(scaleX, segment.height);
    context.fillText(character, 0, 0);
    context.restore();
    if (segment.underline > 0) {
      context.fillStyle = segment.reverse ? PAPER : INK;
      context.fillRect(
        x,
        baseline - segment.underline,
        segment.advance,
        segment.underline,
      );
    }
  });
}

/**
 * Paints the rows of a 1-bit bitmap that fall in the slice, through an offscreen canvas so
 * it scales without blur. Only those rows are decoded: a tall image spans many slices.
 */
function drawBitmap(
  context: CanvasRenderingContext2D,
  placed: IPlaced,
  top: number,
  sliceTop: number,
  sliceBottom: number,
) {
  const [from, to] = visibleRows(top, placed.height, sliceTop, sliceBottom);
  if (placed.width === 0 || from >= to) return;
  const bits = decodeBase64(placed.data);
  const pixels = new ImageData(placed.width, to - from);
  // One 32-bit write per pixel: opaque black where the bit is set, transparent elsewhere.
  // Little-endian (every desktop CPU Tauri targets): 0xff000000 is the bytes R,G,B,A =
  // 0, 0, 0, 255.
  const words = new Uint32Array(pixels.data.buffer);
  const black = 0xff000000;
  for (let y = from; y < to; y += 1) {
    for (let x = 0; x < placed.width; x += 1) {
      if (isBlack(bits, placed.width, x, y)) {
        words[(y - from) * placed.width + x] = black;
      }
    }
  }
  const offscreen = document.createElement('canvas');
  offscreen.width = placed.width;
  offscreen.height = to - from;
  offscreen.getContext('2d')?.putImageData(pixels, 0, 0);
  context.imageSmoothingEnabled = false;
  context.drawImage(offscreen, placed.x, top + from);
}

function drawBlock(
  context: CanvasRenderingContext2D,
  block: IBlock,
  top: number,
  sliceTop: number,
  sliceBottom: number,
) {
  // Feeds are blank paper: nothing to draw.
  if (block.type === 'line') {
    for (const image of block.images ?? []) {
      drawBitmap(context, image, top, sliceTop, sliceBottom);
    }
    for (const segment of block.segments) {
      drawSegment(context, segment, top + block.ascent);
    }
  } else if (block.type === 'image') {
    drawBitmap(context, block, top, sliceTop, sliceBottom);
  }
}

/**
 * Draws the slice `[sliceTop, sliceBottom)` of a receipt onto `canvas`, `scale` CSS pixels
 * per dot (the paper zoom), at the device's pixel ratio: redrawn, never stretched, so it
 * stays sharp at any zoom. The caller sets the canvas' CSS size.
 */
export function drawSlice(
  canvas: HTMLCanvasElement,
  positioned: IPositionedBlock[],
  width: number,
  sliceTop: number,
  sliceBottom: number,
  scale: number,
) {
  const pixels = (window.devicePixelRatio || 1) * scale;
  const height = sliceBottom - sliceTop;
  canvas.width = Math.round(width * pixels);
  canvas.height = Math.round(height * pixels);
  const context = canvas.getContext('2d');
  if (!context) return;
  context.setTransform(pixels, 0, 0, pixels, 0, -sliceTop * pixels);
  for (const { y, block } of blocksIn(positioned, sliceTop, sliceBottom)) {
    drawBlock(context, block, y, sliceTop, sliceBottom);
  }
}
