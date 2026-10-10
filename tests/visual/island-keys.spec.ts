import { expect, test, type Page } from "@playwright/test";

// The island's keys: opened from the keyboard (the open shortcut), it takes the keys until it
// folds. The lab plays core's part and marks what was opened.
const lab = (params: string) => `/lab/?still=1&t=1500&${params}`;
type Lab = { island: { shortcut(id: string): void; last(): { front: { id: string } | null; approval: object | null } } };
const openFromKeyboard = (page: Page) => page.evaluate(() => (window as unknown as Lab).island.shortcut("open"));
const front = (page: Page) => page.evaluate(() => (window as unknown as Lab).island.last().front?.id ?? null);

test("opened from the keyboard, the island says what its keys do", async ({ page }) => {
  await page.goto(lab("state=busy-flock"));
  await openFromKeyboard(page);
  await expect(page.locator("#island")).toHaveClass(/is-open/);
  await expect(page.locator(".keys-hint")).toHaveText("↑ ↓ sessions · ⏎ terminal · M actions · C chat · Esc fold");
  await expect(page.locator("#island")).toHaveScreenshot("island-keys.png");
});

test("arrows walk the flock, Enter opens the terminal, M the actions", async ({ page }) => {
  await page.goto(lab("state=busy-flock"));
  await openFromKeyboard(page);
  await page.keyboard.press("ArrowDown");
  await expect.poll(() => front(page)).not.toBeNull();
  const first = await front(page);
  await page.keyboard.press("ArrowDown");
  await expect.poll(() => front(page)).not.toBe(first);
  const second = await front(page);
  await page.keyboard.press("ArrowUp");
  await expect.poll(() => front(page)).toBe(first);
  await page.keyboard.press("j");
  await expect.poll(() => front(page)).toBe(second);
  await page.keyboard.press("Enter");
  await expect(page.locator("body")).toHaveAttribute("data-opened", `jump claude:${second}`);
  await page.keyboard.press("m");
  await expect(page.locator(".menu-view")).toBeVisible();
});

test("Y answers the permission on screen; a number picks a question's choice", async ({ page }) => {
  await page.goto(lab("state=approval"));
  await openFromKeyboard(page);
  await expect(page.locator(".keys-hint")).toContainText("Y / N answer");
  await page.keyboard.press("y");
  await expect.poll(() => page.evaluate(() => (window as unknown as Lab).island.last().approval)).toBeNull();

  await page.goto(lab("state=question-card"));
  await openFromKeyboard(page);
  await expect(page.locator(".keys-hint")).toContainText("1–9 choose");
  // Two questions: a choice on the first goes on to the second.
  await expect(page.locator(".question-card .queue")).toHaveText("1 of 2");
  await page.keyboard.press("2");
  await expect(page.locator(".question-card .queue")).toHaveText("2 of 2");
});

test("without the open shortcut the keys do nothing, and Esc gives them back", async ({ page }) => {
  await page.goto(lab("state=busy-flock"));
  await page.locator("#island").click();
  await expect(page.locator("#island")).toHaveClass(/is-open/);
  const first = await front(page);
  await page.keyboard.press("ArrowDown");
  await page.waitForTimeout(200);
  expect(await front(page)).toBe(first);
  await expect(page.locator(".keys-hint")).toHaveCount(0);

  await openFromKeyboard(page);
  await expect(page.locator(".keys-hint")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.locator("#island")).not.toHaveClass(/is-open/);
  await openFromKeyboard(page);
  await page.evaluate(() => window.dispatchEvent(new Event("blur")));
  await expect(page.locator(".keys-hint")).toHaveCount(0);
});
