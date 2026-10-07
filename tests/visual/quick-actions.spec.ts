import { expect, test, type Page } from "@playwright/test";

// A session's quick actions (ADR 0011): a right-click on its bird or row, only what works here.
const lab = (params: string) => `/lab/?still=1&t=1500&${params}`;
const opened = (page: Page) => page.locator("body").getAttribute("data-opened");
const SETTLE_MS = 1700;

test("the menu on KDE, with VS Code installed", async ({ page }) => {
  await page.goto(lab("state=live-diff&open=1&menu=lab&raise=1&editor=1"));
  await expect(page.locator(".menu-view")).toBeVisible();
  await expect(page.locator("#island")).toHaveScreenshot("quick-actions.png");
  await page.getByRole("button", { name: "Open in Cursor" }).click();
  await expect.poll(() => opened(page)).toBe("jump claude:lab");
  await expect(page.locator(".menu-view")).toHaveCount(0);
});

test("where a window cannot be raised, the menu says so and offers the folder", async ({ page }) => {
  await page.goto(lab("state=live-diff&open=1&menu=lab"));
  await expect(page.getByRole("button", { name: "Open terminal" })).toBeDisabled();
  await expect(page.locator(".menu-view .hint")).toContainText("can't bring a terminal forward");
  await expect(page.locator("#island")).toHaveScreenshot("quick-actions-no-raise.png");
  await page.getByRole("button", { name: "Open folder" }).click();
  await expect.poll(() => opened(page)).toBe("folder claude:lab");
});

test("the last diff, its file, and the activity", async ({ page }) => {
  await page.goto(lab("state=live-diff&open=1&menu=lab&editor=1"));
  await page.getByRole("button", { name: "Open its file in VS Code" }).click();
  await expect.poll(() => opened(page)).toBe("file claude:lab 11 0");

  await page.evaluate(() => (window as unknown as { island: { openMenu(a: string, id: string): void } }).island.openMenu("claude", "lab"));
  await page.getByRole("button", { name: "View the last diff" }).click();
  await expect(page.locator(".diff-view .diff-line")).toHaveCount(11);
  await page.keyboard.press("Escape");

  await page.evaluate(() => (window as unknown as { island: { openMenu(a: string, id: string): void } }).island.openMenu("claude", "lab"));
  await page.getByRole("button", { name: "Activity" }).click();
  await expect(page.locator(".activity-row")).toHaveCount(3);
  await expect(page.locator("#island")).toHaveScreenshot("quick-actions-activity.png");
  // A diff from the list, and Esc back to the list.
  await page.locator(".activity-row .tick-diff").click();
  await expect(page.locator(".diff-view")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.locator(".activity-row")).toHaveCount(3);
  await page.keyboard.press("Escape");
  await expect(page.locator(".activity-view")).toHaveCount(0);
});

test("a right-click on a row opens that session's menu; Zeca keeps his looks", async ({ page }) => {
  await page.goto(lab("state=editing&open=1"));
  await page.locator(".flock-row", { hasText: "site" }).click({ button: "right" });
  await expect(page.locator(".menu-view .who .name")).toHaveText("site");
  await page.getByRole("button", { name: "Keep in front" }).click();
  await expect(page.locator(".menu-view")).toHaveCount(0);
  await expect(page.locator(".card.focus .who .name")).toHaveText("site");
});

test("a waiting card keeps the island: no menu over it", async ({ page }) => {
  await page.goto(lab("state=approval&open=1"));
  await page.waitForTimeout(SETTLE_MS);
  await page.evaluate(() => (window as unknown as { island: { openMenu(a: string, id: string): void } }).island.openMenu("codex", "b"));
  await expect(page.locator(".menu-view")).toHaveCount(0);
  await expect(page.locator(".card.focus")).toContainText("cargo test");
});
