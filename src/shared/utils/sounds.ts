import type { IReceiptSummary } from '@/shared/interfaces/emulator';

/** At most this many beeps per receipt: a POS asking for 9 should not ring for seconds. */
const MAX_BEEPS = 3;
const BEEP_HZ = 2700;
const BEEP_S = 0.12;
const GAP_S = 0.08;

let context: AudioContext | undefined;
let noise: AudioBuffer | undefined;

function audio(): AudioContext {
  context ??= new AudioContext();
  void context.resume();
  return context;
}

/** One second of white noise, made once. */
function noiseBuffer(audioContext: AudioContext): AudioBuffer {
  if (!noise) {
    noise = audioContext.createBuffer(
      1,
      audioContext.sampleRate,
      audioContext.sampleRate,
    );
    const samples = noise.getChannelData(0);
    for (let index = 0; index < samples.length; index += 1) {
      samples[index] = Math.random() * 2 - 1;
    }
  }
  return noise;
}

/**
 * Receipts that finished since the last call, the beeps they asked for, and the ids seen
 * so far. `seen === null` is the first look: everything already there counts as heard.
 */
export function pendingSounds(
  seen: ReadonlySet<number> | null,
  receipts: IReceiptSummary[],
): { printed: IReceiptSummary[]; beeps: number; seen: Set<number> } {
  const next = new Set(seen ?? []);
  const printed: IReceiptSummary[] = [];
  let beeps = 0;
  for (const receipt of receipts) {
    if (receipt.state === 'printing' || next.has(receipt.id)) continue;
    next.add(receipt.id);
    if (seen === null) continue;
    printed.push(receipt);
    beeps += Math.min(receipt.beeps, MAX_BEEPS);
  }
  return { printed, beeps, seen: next };
}

/**
 * The print head and its stepper motor: band-passed noise pulsing at 38 Hz, as long as the
 * paper takes to come out. Generated, no sound file; quiet.
 */
export function playPrint(durationMs: number) {
  const audioContext = audio();
  const start = audioContext.currentTime + 0.01;
  const length = Math.max(0.15, durationMs / 1000);

  const source = audioContext.createBufferSource();
  source.buffer = noiseBuffer(audioContext);
  source.loop = true;
  const band = audioContext.createBiquadFilter();
  band.type = 'bandpass';
  band.frequency.value = 2200;
  band.Q.value = 0.8;
  // Amplitude modulation: the gain swings between 0 and 1 at the motor's step rate.
  const motor = audioContext.createGain();
  motor.gain.value = 0.5;
  const steps = audioContext.createOscillator();
  steps.type = 'square';
  steps.frequency.value = 38;
  const depth = audioContext.createGain();
  depth.gain.value = 0.5;
  steps.connect(depth).connect(motor.gain);
  const volume = audioContext.createGain();
  volume.gain.setValueAtTime(0, start);
  volume.gain.linearRampToValueAtTime(0.06, start + 0.03);
  volume.gain.setValueAtTime(0.06, start + length - 0.08);
  volume.gain.linearRampToValueAtTime(0, start + length);

  source
    .connect(band)
    .connect(motor)
    .connect(volume)
    .connect(audioContext.destination);
  source.start(start);
  steps.start(start);
  source.stop(start + length + 0.05);
  steps.stop(start + length + 0.05);
}

/** A printer-like buzzer (`ESC B`), after `delayMs` (the printing sound). */
export function playBeeps(count: number, delayMs = 0) {
  if (count <= 0) return;
  const audioContext = audio();
  const start = audioContext.currentTime + 0.01 + delayMs / 1000;
  for (let index = 0; index < Math.min(count, MAX_BEEPS); index += 1) {
    const at = start + index * (BEEP_S + GAP_S);
    const oscillator = audioContext.createOscillator();
    const gain = audioContext.createGain();
    oscillator.type = 'square';
    oscillator.frequency.value = BEEP_HZ;
    gain.gain.setValueAtTime(0.05, at);
    gain.gain.setValueAtTime(0, at + BEEP_S);
    oscillator.connect(gain).connect(audioContext.destination);
    oscillator.start(at);
    oscillator.stop(at + BEEP_S);
  }
}
