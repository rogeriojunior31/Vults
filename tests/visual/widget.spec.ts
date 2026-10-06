import { expect, test } from "@playwright/test";

// The corner widget's states (ui/lab/widget.html), time frozen.
const lab = (params = "") => `/lab/widget.html?still=1&t=1500${params}`;

for (const state of ["empty", "one", "busy", "card", "resting", "paused"]) {
  test(`corner widget: ${state}`, async ({ page }) => {
    await page.goto(lab());
    await expect(page.locator(`[data-state="${state}"] .widget`)).toHaveScreenshot(`widget-${state}.png`);
  });
}

test("corner widget: Zeca off leaves an empty wire empty", async ({ page }) => {
  await page.goto(lab("&nozeca=1&state=empty"));
  await expect(page.locator('[data-state="empty"] .widget')).toHaveScreenshot("widget-empty-nozeca.png");
});

test("corner widget: the counts say who works and who needs you", async ({ page }) => {
  await page.goto(lab());
  await expect(page.locator('[data-state="busy"] .counts')).toHaveText("3 working");
  await expect(page.locator('[data-state="card"] .counts')).toHaveText("1 needs you2 working");
  await expect(page.locator('[data-state="card"] .widget')).toHaveClass(/needs-you/);
  await expect(page.locator('[data-state="empty"] .counts')).toHaveText("Nothing running");
  await expect(page.locator('[data-state="resting"] .counts')).toHaveText("1 resting");
  await expect(page.locator('[data-state="paused"] .counts')).toHaveText("Paused");
});
