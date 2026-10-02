// 8-bit sounds, synthesized on the spot with WebAudio: square and triangle blips, no audio files.
// Short and quiet on purpose: a nudge, not an alarm.

export type Cue = "approval" | "question" | "done" | "fail" | "alert" | "alertOk" | "swallow" | "tap" | "allow" | "deny" | "squawk" | "hiss" | "hello" | "open" | "close" | "peek";

type Note = [freq: number, ms: number, wave?: OscillatorType];

const CUES: Record<Cue, Note[]> = {
  // Two quick rising blips: something needs you.
  approval: [[659, 70, "square"], [880, 110, "square"]],
  // A rising chirp, like a raised eyebrow.
  question: [[523, 60, "triangle"], [784, 120, "triangle"]],
  // A small happy arpeggio.
  done: [[523, 60, "triangle"], [659, 60, "triangle"], [784, 60, "triangle"], [1047, 140, "triangle"]],
  // Two falling buzzes.
  fail: [[392, 110, "square"], [294, 180, "square"]],
  alert: [[440, 80, "square"], [440, 80, "square"]],
  alertOk: [[988, 90, "triangle"]],
  // A gulp: a low blip sliding down.
  swallow: [[330, 50, "triangle"], [220, 90, "triangle"]],
  // A tick: you picked something.
  tap: [[1175, 28, "triangle"]],
  // Allowed: a short step up. Denied: one low note.
  allow: [[784, 45, "triangle"], [1175, 80, "triangle"]],
  deny: [[262, 110, "triangle"]],
  // Startled: a short rough squawk. Annoyed: a low buzz. Hello: a two-note chirp.
  squawk: [[740, 40, "square"], [520, 70, "square"]],
  hiss: [[150, 220, "sawtooth"]],
  hello: [[660, 70, "triangle"], [990, 120, "triangle"]],
  // The island opening, folding, and coming out of hiding: soft, the quietest cues.
  open: [[523, 35, "triangle"], [784, 55, "triangle"]],
  close: [[784, 35, "triangle"], [523, 55, "triangle"]],
  peek: [[880, 40, "triangle"]],
};

const VOLUME = 0.05;

let ctx: AudioContext | null = null;
let enabled = true;
let last = 0;

export const Sound = {
  setEnabled(on: boolean): void {
    enabled = on;
  },

  isEnabled(): boolean {
    return enabled;
  },

  play(cue: Cue): void {
    if (!enabled) return;
    // Two cues in the same breath (several sessions finishing at once) would just be noise.
    const now = performance.now();
    if (now - last < 250) return;
    last = now;
    try {
      ctx ??= new AudioContext();
      void ctx.resume();
      let t = ctx.currentTime;
      for (const [freq, ms, wave = "square"] of CUES[cue]) {
        const osc = ctx.createOscillator();
        const gain = ctx.createGain();
        osc.type = wave;
        osc.frequency.setValueAtTime(freq, t);
        // A tiny attack and release so blips don't click.
        gain.gain.setValueAtTime(0, t);
        gain.gain.linearRampToValueAtTime(VOLUME, t + 0.005);
        gain.gain.setValueAtTime(VOLUME, t + ms / 1000 - 0.01);
        gain.gain.linearRampToValueAtTime(0, t + ms / 1000);
        osc.connect(gain).connect(ctx.destination);
        osc.start(t);
        osc.stop(t + ms / 1000);
        t += ms / 1000 + 0.015;
      }
    } catch {
      // No audio device: stay quiet.
    }
  },
};
