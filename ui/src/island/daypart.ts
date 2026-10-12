// A vulture's day (the flock board's "thermal has an hour"): no sun, no thermal. At dawn the idle
// flock opens its wings to warm up, from mid-morning to late afternoon it soars, at dusk it comes
// down, and at night it sleeps on the wire. By the local clock, once the app follows it.
export type DayPart = "dawn" | "day" | "dusk" | "night";

/** The part of the day at a local hour. */
export function partAt(hour: number): DayPart {
  if (hour >= 6 && hour < 9) return "dawn";
  if (hour >= 9 && hour < 17) return "day";
  if (hour >= 17 && hour < 19) return "dusk";
  return "night";
}

// "day" until the app follows the clock: the lab and the tests never depend on the hour.
let part: DayPart = "day";
let timer: number | undefined;
const listeners: (() => void)[] = [];

export const dayPart = (): DayPart => part;

/** Sets the part of the day (the lab's `?hour=`, the tests); `followClock` sets it by itself. */
export function setDayPart(next: DayPart): void {
  if (next === part) return;
  part = next;
  for (const l of listeners) l();
}

/** Something to redraw when the part of the day turns. */
export function onDayPart(listener: () => void): void {
  listeners.push(listener);
}

/** The island, the Nest and the widget follow the local clock, checked every minute. */
export function followClock(): void {
  const check = () => setDayPart(partAt(new Date().getHours()));
  check();
  window.clearInterval(timer);
  timer = window.setInterval(check, 60_000);
}
