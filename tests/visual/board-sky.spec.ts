// The flock passes behind a connector's card (GitHub), as a visitor does, never over its text; over
// any other card it still flies in front.
import { expect, test, type Page } from "@playwright/test";

/** The bird's ink inside and outside the card over one flight, with the card a `.board` or not. */
async function flight(page: Page, board: boolean) {
  await page.clock.install();
  await page.goto("/lab/flight/");
  await page.evaluate(async (board) => {
    const path = "/src/island/sky.ts";
    const { Sky } = await import(path);
    const sky = new Sky(() => {});
    sky.canvas.style.cssText = "position:fixed;left:0;top:0;width:720px;height:560px";
    document.body.append(sky.canvas);
    const card = { left: 20, top: 40, width: 680, height: 150, radius: 20 };
    const el = document.createElement("div");
    el.className = board ? "board" : "card";
    el.style.cssText = `position:fixed;left:${card.left}px;top:${card.top}px;width:${card.width}px;height:${card.height}px`;
    document.body.append(el);
    const session = { id: "a", agent: "claude", project: "a", cwd: null, status: "idle", activity: null,
      step: null, steps: [], step_count: 0, subagents: 0, note: null, editor: null, species: "atratus" };
    sky.place(new Map([["claude:a", { x: 600, y: 24, scale: 1 }]]), { left: 0, top: 0, width: 720, height: 200, radius: 14, cards: [card] });
    sky.update([session], true);
    Object.assign(window, { boardTest: { sky, card } });
  }, board);
  const ink = () => page.evaluate(() => {
    const { sky, card } = (window as any).boardTest;
    const w = sky.canvas.width, d = w / 720;
    const data = sky.canvas.getContext("2d").getImageData(0, 0, w, sky.canvas.height).data;
    let inside = 0, outside = 0;
    for (let p = 0; p < data.length / 4; p++) {
      if (!data[p * 4 + 3]) continue;
      const x = (p % w) / d, y = Math.floor(p / w) / d;
      if (x > card.left + 1 && x < card.left + card.width - 1 && y > card.top + 1 && y < card.top + card.height - 1) inside++;
      else outside++;
    }
    return { inside, outside };
  });
  let inside = 0, outside = 0;
  // It rests 2 s, takes off, then circles the island for a lap or so.
  for (let t = 0; t < 14000; t += 250) {
    await page.clock.runFor(250);
    const now = await ink();
    inside += now.inside;
    outside += now.outside;
  }
  return { inside, outside };
}

test("a session's bird flies behind the GitHub card, never over its text", async ({ page }) => {
  const { inside, outside } = await flight(page, true);
  expect(outside).toBeGreaterThan(0);
  expect(inside).toBe(0);
});

test("over any other card the flock still flies in front", async ({ page }) => {
  const { inside } = await flight(page, false);
  expect(inside).toBeGreaterThan(0);
});
