import { expect, test } from "@playwright/test";

/** The lab's island states, by index (see ui/lab/lab.ts). */
const STATES = ["working", "searching", "web", "approval", "done-with-alerts", "chat"];
const CLIPS = ["idle", "think", "read", "search", "edit", "run", "approval", "question", "done", "fail", "sleep", "swallow", "fly"];

const lab = (params: string) => `/lab/?still=1&t=1500&${params}`;
/** States the island announces only once they hold (render.ts SETTLE_MS). */
const SETTLES: Record<string, number> = { approval: 1700, "done-with-alerts": 3200 };

test("island folded", async ({ page }) => {
  await page.goto(lab("island=0"));
  await expect(page.locator("#island")).toHaveScreenshot("island-folded.png");
});

for (const [i, name] of STATES.entries()) {
  test(`island open: ${name}`, async ({ page }) => {
    await page.goto(lab(`island=${i}&open=1`));
    if (SETTLES[name]) await page.waitForTimeout(SETTLES[name]);
    await expect(page.locator("#island")).toHaveScreenshot(`island-${name}.png`);
  });
}

test("every clip at one instant", async ({ page }) => {
  await page.goto(lab("island=0"));
  const cards = page.locator("#clips .clip-card canvas");
  await expect(cards).toHaveCount(CLIPS.length);
  for (const [i, name] of CLIPS.entries()) {
    await expect(cards.nth(i)).toHaveScreenshot(`clip-${name}.png`);
  }
});
