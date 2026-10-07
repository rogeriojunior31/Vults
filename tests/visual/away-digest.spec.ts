import { expect, test } from "@playwright/test";

// "While you were away" (C5): back from a locked screen, a digest opens the island once and stays
// until dismissed. The lab plays core's part.
const lab = (params: string) => `/lab/?still=1&t=1500&${params}`;

test("the digest over the news, dismissed with ×", async ({ page }) => {
  await page.goto(lab("state=done&open=1&digest=1"));
  await page.waitForTimeout(3200);
  await expect(page.locator(".digest")).toContainText("While you were away: 2 finished");
  await expect(page.locator("#island")).toHaveScreenshot("away-digest.png");
  await page.locator(".digest .icon-btn").click();
  await expect(page.locator(".digest")).toHaveCount(0);
});

test("a new digest opens the folded island once", async ({ page }) => {
  await page.goto(lab("state=editing"));
  await expect(page.locator("#island")).not.toHaveClass(/is-open/);
  await page.evaluate(() => {
    const island = (window as unknown as { island: { last(): object; render(v: object): void } }).island;
    island.render({ ...island.last(), digest: { seq: 7, text: "While you were away: 1 finished.", finished: 1, failed: 0, waiting: 0 } });
  });
  await expect(page.locator("#island")).toHaveClass(/is-open/);
  await expect(page.locator(".digest")).toContainText("1 finished");
});
