import { expect, test } from "@playwright/test";

// The island in Brazilian Portuguese, Spanish and Simplified Chinese: the longest words still fit
// the permission card, the question card and the flock.
const lab = (params: string) => `/lab/?still=1&t=1500&${params}`;

for (const lang of ["pt-BR", "es", "zh"]) {
  test(`the island in ${lang}`, async ({ page }) => {
    await page.goto(lab(`state=approval&open=1&lang=${lang}`));
    await expect(page.locator("html")).toHaveAttribute("lang", lang);
    await expect(page.locator("#island")).toHaveScreenshot(`island-${lang}-approval.png`);
    await page.goto(lab(`state=question-card&open=1&lang=${lang}`));
    await expect(page.locator("#island")).toHaveScreenshot(`island-${lang}-question.png`);
    await page.goto(lab(`state=busy-flock&open=1&lang=${lang}&menu=lab`));
    await expect(page.locator(".menu-view")).toBeVisible();
    await expect(page.locator("#island")).toHaveScreenshot(`island-${lang}-menu.png`);
  });
}

for (const lang of ["pt-BR", "zh"]) {
  test(`Activity and the share image in ${lang}`, async ({ page }) => {
    await page.setViewportSize({ width: 760, height: 1100 });
    await page.goto(`/lab/activity.html?lang=${lang}`);
    await expect(page.locator(".page")).toHaveScreenshot(`activity-${lang}.png`);
    await page.setViewportSize({ width: 600, height: 740 });
    await page.goto(`/lab/recap-image.html?lang=${lang}`);
    await expect(page.locator("body")).toHaveAttribute("data-ready", "1");
    await expect(page.locator("canvas")).toHaveScreenshot(`recap-image-${lang}.png`);
  });
}
