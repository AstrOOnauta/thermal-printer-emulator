import type { IReceiptSummary } from '@/shared/interfaces/emulator';

/** At most this many beeps per receipt: a POS asking for 9 should not ring for seconds. */
const MAX_BEEPS = 3;
const TONE_HZ = 2700;
const TONE_S = 0.12;
const GAP_S = 0.08;

let context: AudioContext | undefined;

/**
 * Receipts that finished with a beep since the last call, and the ids seen so far.
 * `seen === null` is the first look: everything already there counts as heard.
 */
export function pendingBeeps(
  seen: ReadonlySet<number> | null,
  receipts: IReceiptSummary[],
): { beeps: number; seen: Set<number> } {
  const next = new Set(seen ?? []);
  let beeps = 0;
  for (const receipt of receipts) {
    if (receipt.state === 'printing' || next.has(receipt.id)) continue;
    next.add(receipt.id);
    if (seen !== null) beeps += Math.min(receipt.beeps, MAX_BEEPS);
  }
  return { beeps, seen: next };
}

/** A printer-like buzzer: short high square-ish tones, generated, no sound file. */
export function playBeeps(count: number) {
  if (count <= 0) return;
  context ??= new AudioContext();
  void context.resume();
  const start = context.currentTime + 0.01;
  for (let index = 0; index < Math.min(count, MAX_BEEPS); index += 1) {
    const at = start + index * (TONE_S + GAP_S);
    const oscillator = context.createOscillator();
    const gain = context.createGain();
    oscillator.type = 'square';
    oscillator.frequency.value = TONE_HZ;
    gain.gain.setValueAtTime(0.05, at);
    gain.gain.setValueAtTime(0, at + TONE_S);
    oscillator.connect(gain).connect(context.destination);
    oscillator.start(at);
    oscillator.stop(at + TONE_S);
  }
}
