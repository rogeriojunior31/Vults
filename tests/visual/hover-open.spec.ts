import { expect, test } from "@playwright/test";

// Open on hover (Settings → General): resting the pointer on the pill opens the island, and an
// island opened that way folds soon after the pointer leaves, unless the user clicked in it.

type Lab = { island: { setOpenOnHover(on: boolean): void } };

const away = { x: 650, y: 900 };

async function setUp(page: import("@playwright/test").Page, on: boolean) {
  await page.goto("/lab/?state=editing");
  await page.evaluate((v) => (window as unknown as Lab).island.setOpenOnHover(v), on);
  const island = page.locator("#island");
  await expect(island).not.toHaveClass(/is-open|is-hidden/);
  return island;
}

test("off, hovering the pill only keeps it compact", async ({ page }) => {
  const island = await setUp(page, false);
  await island.hover();
  await page.waitForTimeout(700);
  await expect(island).not.toHaveClass(/is-open|is-hidden/);
});

test("on, resting on the pill opens the island, and leaving folds it", async ({ page }) => {
  const island = await setUp(page, true);
  await island.hover();
  await expect(island).toHaveClass(/is-open/);
  await page.mouse.move(away.x, away.y);
  await expect(island).not.toHaveClass(/is-open|is-hidden/, { timeout: 2000 });
});

test("on, passing over the pill opens nothing", async ({ page }) => {
  const island = await setUp(page, true);
  await island.hover();
  await page.mouse.move(away.x, away.y);
  await page.waitForTimeout(700);
  await expect(island).not.toHaveClass(/is-open|is-hidden/);
});

test("on, a click in the hover-opened island keeps it open after the pointer leaves", async ({ page }) => {
  const island = await setUp(page, true);
  await island.hover();
  await expect(island).toHaveClass(/is-open/);
  const box = (await island.boundingBox())!;
  // An empty spot near the bottom edge: nothing there to act on.
  await page.mouse.click(box.x + box.width / 2, box.y + box.height - 4);
  await page.mouse.move(away.x, away.y);
  await page.waitForTimeout(1200);
  await expect(island).toHaveClass(/is-open/);
});
