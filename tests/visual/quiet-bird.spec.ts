import { expect, test } from "@playwright/test";

// A quiet bird (C8): a working session with no news is flagged; the user's answer only clears the
// flag. The lab plays core's part.
const lab = (params: string) => `/lab/?still=1&t=1500&${params}`;

test("a loud quiet bird on its card, a quiet one on its badge", async ({ page }) => {
  await page.goto(lab("state=quiet-bird"));
  await expect(page.locator("#island .badge.silent")).toHaveCount(1);
  await expect(page.locator("#island")).toHaveScreenshot("quiet-bird-compact.png");
  await page.goto(lab("state=quiet-bird&open=1"));
  await expect(page.locator(".silence.loud")).toContainText("No news for 15 minutes.");
  await expect(page.locator("#island")).toHaveScreenshot("quiet-bird.png");
  await page.getByRole("button", { name: "Keep going" }).click();
  await expect.poll(() => page.locator("body").getAttribute("data-opened")).toBe("hush claude:lab keep-going");
  await expect(page.locator(".silence")).toHaveCount(0);
});
