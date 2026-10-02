// Motion for the island's shape: a real spring to grow (it overshoots a little, like a sheet of
// rubber) and a timed curve to shrink (no overshoot). A spring keeps its velocity when its target
// moves mid-flight, so an island that grows twice in a row never jerks.

const clamp = (v: number, lo: number, hi: number) =>
  Math.max(lo, Math.min(hi, v));
const lerp = (a: number, b: number, t: number) => a + (b - a) * t;

/** cubic-bezier(x1, y1, x2, y2), solved for x by bisection. */
export function cubicBezier(
  x1: number,
  y1: number,
  x2: number,
  y2: number,
): (x: number) => number {
  const cx = (t: number) =>
    3 * (1 - t) ** 2 * t * x1 + 3 * (1 - t) * t * t * x2 + t ** 3;
  const cy = (t: number) =>
    3 * (1 - t) ** 2 * t * y1 + 3 * (1 - t) * t * t * y2 + t ** 3;
  return (x) => {
    let lo = 0;
    let hi = 1;
    let t = x;
    for (let i = 0; i < 12; i++) {
      if (cx(t) < x) lo = t;
      else hi = t;
      t = (lo + hi) / 2;
    }
    return cy(t);
  };
}

const closeCurve = cubicBezier(0.45, 0, 0.2, 1);

/** A damped spring: angular frequency 2π / response, damping ratio `damping`. */
export class Spring {
  value: number;
  target: number;
  velocity = 0;
  private omega: number;
  private zeta: number;

  constructor(value: number, response = 0.5, damping = 0.72) {
    this.value = value;
    this.target = value;
    this.omega = (2 * Math.PI) / response;
    this.zeta = damping;
  }

  set(value: number): void {
    this.value = value;
    this.target = value;
    this.velocity = 0;
  }

  get settled(): boolean {
    return (
      Math.abs(this.target - this.value) < 0.01 &&
      Math.abs(this.velocity) < 0.05
    );
  }

  /** Sub-stepped at 240 Hz, so a late timer never destabilises it. */
  step(dt: number): void {
    const steps = Math.max(1, Math.ceil(dt / (1 / 240)));
    const h = dt / steps;
    for (let i = 0; i < steps; i++) {
      const acc =
        this.omega * this.omega * (this.target - this.value) -
        2 * this.zeta * this.omega * this.velocity;
      this.velocity += acc * h;
      this.value += this.velocity * h;
    }
  }
}

/** A value that springs when it grows and follows the closing curve when it shrinks. */
export class Tracked {
  private readonly spring: Spring;
  private from = 0;
  private to = 0;
  private start = 0;
  private duration = 0;
  private mode: "spring" | "curve" | "idle" = "idle";

  constructor(value: number) {
    this.spring = new Spring(value);
  }

  get value(): number {
    return this.spring.value;
  }

  get target(): number {
    return this.spring.target;
  }

  get animating(): boolean {
    return this.mode !== "idle";
  }

  jump(v: number): void {
    this.spring.set(v);
    this.mode = "idle";
  }

  springTo(v: number): void {
    if (v === this.spring.target && this.mode !== "curve") return;
    this.spring.target = v;
    this.mode = "spring";
  }

  curveTo(v: number, now: number, ms = 340): void {
    if (v === this.spring.target && this.mode === "curve") return;
    this.from = this.spring.value;
    this.to = v;
    this.start = now;
    this.duration = ms;
    this.spring.target = v;
    this.spring.velocity = 0;
    this.mode = "curve";
  }

  /** Grows with the spring, shrinks with the curve. */
  moveTo(v: number, now: number): void {
    if (v >= this.spring.value) this.springTo(v);
    else this.curveTo(v, now);
  }

  step(dt: number, now: number): void {
    if (this.mode === "spring") {
      this.spring.step(dt);
      if (this.spring.settled) {
        this.spring.set(this.spring.target);
        this.mode = "idle";
      }
    } else if (this.mode === "curve") {
      const p = clamp((now - this.start) / this.duration, 0, 1);
      this.spring.value = lerp(this.from, this.to, closeCurve(p));
      if (p >= 1) {
        this.spring.set(this.to);
        this.mode = "idle";
      }
    }
  }
}
