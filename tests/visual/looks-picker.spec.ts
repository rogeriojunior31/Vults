import { expect, test } from "@playwright/test";

const lab = (params: string) => `/lab/?still=1&t=1500&${params}`;

test("Zeca's looks: grouped, the one under the pointer worn on his perch", async ({ page }) => {
  await page.goto(lab("empty=1&open=1&looks=1"));
  const picker = page.locator("#island .card.looks");
  await expect(picker.locator(".looks-label")).toHaveText(["Seasonal", "Head", "With a chain"]);
  await expect(picker.locator(".look")).toHaveCount(19);
  await expect(page.locator("#island")).toHaveScreenshot("island-looks-picker.png");
  // The live preview: the crown on Zeca while the pointer rests on it.
  await picker.getByRole("button", { name: "Crown and chain" }).hover();
  await expect(page.locator("#island")).toHaveScreenshot("island-looks-picker-preview.png");
});

test("Zeca's looks: a right-click on him opens them, Escape closes them", async ({ page }) => {
  await page.goto(lab("empty=1&open=1"));
  const perch = page.locator("#island .card.focus .perch canvas");
  const box = (await perch.boundingBox())!;
  // His body sits in the middle of the perch, a little low.
  await page.mouse.move(box.x + box.width / 2, box.y + box.height * 0.6);
  await page.mouse.click(box.x + box.width / 2, box.y + box.height * 0.6, { button: "right" });
  await expect(page.locator("#island .card.looks")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.locator("#island .card.looks")).toHaveCount(0);
});
