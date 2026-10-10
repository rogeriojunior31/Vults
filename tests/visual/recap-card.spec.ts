import { expect, test } from "@playwright/test";

// The Monday card (docs/guide/activity.md): last week's recap opens the top island once and
// stays until read; Open Activity shows the whole week. The lab plays core's part.
const lab = (params: string) => `/lab/?still=1&t=1500&${params}`;

test("last week's recap over the news, with Open Activity", async ({ page }) => {
  await page.goto(lab("state=done&open=1&recap=1"));
  await page.waitForTimeout(3200);
  await expect(page.locator(".recap")).toContainText("Last week: 41 turns, 6 h 20 min with your agents, most on site.");
  await expect(page.locator("#island")).toHaveScreenshot("recap-card.png");
  await page.getByRole("button", { name: "Open Activity" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-opened", "settings activity");
  await page.locator(".recap .icon-btn").click();
  await expect(page.locator(".recap")).toHaveCount(0);
});

test("a new recap opens the folded island once", async ({ page }) => {
  await page.goto(lab("state=editing"));
  await expect(page.locator("#island")).not.toHaveClass(/is-open/);
  await page.evaluate(() => {
    const island = (window as unknown as { island: { last(): object; render(v: object): void } }).island;
    island.render({ ...island.last(), recap: { seq: 3, monday: "2026-10-05", text: "Last week: 2 turns, 12 min with your agents." } });
  });
  await expect(page.locator("#island")).toHaveClass(/is-open/);
  await expect(page.locator(".recap")).toContainText("2 turns");
});
