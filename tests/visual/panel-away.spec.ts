import { expect, test } from "@playwright/test";

// By the panel the open island sits in the screen's corner, over other windows. When Settings takes
// the focus the app says "away": the island folds, unless a card waits (ADR 0009).

type Lab = { island: { shortcut(id: string): void; away(): void } };

test("by the panel, the open island folds when another window takes the focus", async ({ page }) => {
  await page.goto("/lab/?state=editing&presence=panel");
  const island = page.locator("#island");
  await expect(island).toHaveClass(/is-hidden/);
  await page.evaluate(() => (window as unknown as Lab).island.shortcut("open"));
  await expect(island).toHaveClass(/is-open/);
  await page.evaluate(() => (window as unknown as Lab).island.away());
  await expect(island).toHaveClass(/is-hidden/);
});

test("by the panel, a waiting card keeps the island open", async ({ page }) => {
  await page.goto("/lab/?state=approval&presence=panel");
  const island = page.locator("#island");
  await expect(island).toHaveClass(/is-open/);
  await page.evaluate(() => (window as unknown as Lab).island.away());
  await page.waitForTimeout(300);
  await expect(island).toHaveClass(/is-open/);
});

test("at the top, the open island stays when another window takes the focus", async ({ page }) => {
  await page.goto("/lab/?state=editing");
  const island = page.locator("#island");
  await page.evaluate(() => (window as unknown as Lab).island.shortcut("open"));
  await expect(island).toHaveClass(/is-open/);
  await page.evaluate(() => (window as unknown as Lab).island.away());
  await page.waitForTimeout(300);
  await expect(island).toHaveClass(/is-open/);
});
