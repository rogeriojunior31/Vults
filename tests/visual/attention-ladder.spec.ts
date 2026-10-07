import { expect, test } from "@playwright/test";

// The attention ladder (C4): a waiting card sounds again as core counts reminders, unless do not
// disturb is on; the moon in the header ends it. The lab plays core's part.
const lab = (params: string) => `/lab/?still=1&t=1500&${params}`;
const SETTLE_MS = 1700;

/** Counts the cues the island plays, by replacing the page's audio. */
async function countCues(page: import("@playwright/test").Page): Promise<void> {
  await page.addInitScript(() => {
    const w = window as unknown as { cues: number; AudioContext: unknown };
    w.cues = 0;
    w.AudioContext = class {
      currentTime = 0;
      destination = {};
      resume() {
        return Promise.resolve();
      }
      suspend() {
        return Promise.resolve();
      }
      createOscillator() {
        w.cues++;
        return { connect: (x: unknown) => x, start() {}, stop() {}, frequency: { setValueAtTime() {} }, type: "" };
      }
      createGain() {
        return { connect: (x: unknown) => x, gain: { setValueAtTime() {}, linearRampToValueAtTime() {}, exponentialRampToValueAtTime() {} } };
      }
    };
  });
}

const remind = (page: import("@playwright/test").Page, n: number) =>
  page.evaluate((n) => {
    const island = (window as unknown as { island: { last(): { approval: object | null }; render(v: object): void } }).island;
    const v = island.last();
    island.render({ ...v, approval: v.approval && { ...v.approval, reminders: n } });
  }, n);

const cues = (page: import("@playwright/test").Page) => page.evaluate(() => (window as unknown as { cues: number }).cues);

test("each reminder sounds the card again", async ({ page }) => {
  await countCues(page);
  await page.goto(lab("state=approval&open=1"));
  await page.waitForTimeout(SETTLE_MS);
  const before = await cues(page);
  await page.waitForTimeout(300);
  await remind(page, 1);
  await expect.poll(() => cues(page)).toBeGreaterThan(before);
});

test("do not disturb: the card keeps its cue but no reminder; the moon ends it", async ({ page }) => {
  await countCues(page);
  await page.goto(lab("state=approval&open=1&dnd=1"));
  await page.waitForTimeout(SETTLE_MS);
  await page.waitForTimeout(300);
  const before = await cues(page);
  await remind(page, 1);
  await page.waitForTimeout(300);
  expect(await cues(page)).toBe(before);
  await expect(page.locator("#island .card.focus")).toContainText("cargo test");
  await expect(page.locator("#island")).toHaveScreenshot("dnd-approval.png");
  await page.locator(".icon-btn.dnd").click();
  await expect(page.locator(".icon-btn.dnd")).toHaveCount(0);
});
