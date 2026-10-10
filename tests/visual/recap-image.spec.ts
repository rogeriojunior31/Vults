import { expect, test } from "@playwright/test";

// The week as an image to share (docs/guide/activity.md), drawn in a canvas; the app only
// writes the PNG where the user picks.
test("the week's image, with and without the project's name", async ({ page }) => {
  await page.setViewportSize({ width: 600, height: 740 });
  await page.goto("/lab/recap-image.html");
  await expect(page.locator("body")).toHaveAttribute("data-ready", "1");
  await expect(page.locator("canvas")).toHaveScreenshot("recap-image.png");
  await page.goto("/lab/recap-image.html?hide=1");
  await expect(page.locator("body")).toHaveAttribute("data-ready", "1");
  await expect(page.locator("canvas")).toHaveScreenshot("recap-image-hidden.png");
});

test("Save as image from the week, with the project names hidden", async ({ page }) => {
  await page.goto("/lab/activity.html");
  await page.getByText("Hide project names").click();
  await page.getByRole("button", { name: "Save as image…" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-image", "hidden");
  await expect(page.locator(".week-share .note.ok")).toHaveText("Saved to ~/Pictures/vults-week-2026-10-05.png");
});
