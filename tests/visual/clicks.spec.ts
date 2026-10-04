import { expect, test } from "@playwright/test";

// Clicks land while the island keeps repainting: a real click takes ~100 ms between press and
// release, and an element replaced in between swallows it. Time runs here (no ?still=1).

/** A click the way a person makes one: press, a moment, release. */
async function humanClick(page: import("@playwright/test").Page, selector: string): Promise<void> {
  const box = (await page.locator(selector).boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.waitForTimeout(150);
  await page.mouse.up();
}

test("the song's controls take a click while agents keep the island busy", async ({ page }) => {
  // idle-flock, open, a song playing, and a render every 40 ms.
  await page.goto("/lab/?state=idle-flock&open=1&music=1&churn=1");
  await expect(page.locator("#island .now-playing")).toHaveClass(/(^|\s)playing(\s|$)/);
  // A pointer, not a locator hover: a node replaced every 40 ms never "settles" for Playwright.
  const song = (await page.locator("#island .now-playing").boundingBox())!;
  await page.mouse.move(song.x + song.width / 2, song.y + song.height / 2);
  await page.waitForTimeout(300);
  await humanClick(page, '#island .now-playing button[aria-label="Pause"]');
  await expect(page.locator("#island .now-playing")).not.toHaveClass(/(^|\s)playing(\s|$)/, { timeout: 1000 });
});

test("the mic's stop button takes a click while the level moves", async ({ page }) => {
  await page.goto("/lab/?state=chat&open=1");
  await page.locator("#island .mic").click();
  await expect(page.locator("#island .wave em")).toHaveText("Listening…");
  // Let the fake mic report a few levels (every 60 ms), then click stop.
  await page.waitForTimeout(400);
  await humanClick(page, "#island .mic");
  await expect(page.locator("#island .wave em")).toHaveText("Transcribing…", { timeout: 1000 });
});
