import { expect, test } from "@playwright/test";

const lab = (params: string) => `/lab/?still=1&t=1500&${params}`;

// Zeca saying a reply aloud: his *speak* clip on the chat's perch.
test("island open: the chat while Zeca speaks", async ({ page }) => {
  await page.goto(lab("state=chat&open=1&speaking=1"));
  await expect(page.locator("#island")).toHaveScreenshot("island-chat-speaking.png");
});

// Any key or click in the island silences him at once.
for (const how of ["key", "click"]) {
  test(`a ${how} in the island stops him`, async ({ page }) => {
    await page.goto(lab("state=chat&open=1&speaking=1"));
    await expect(page.locator("body")).not.toHaveAttribute("data-hushed");
    if (how === "key") await page.keyboard.press("a");
    else await page.locator("#island .chat .log").click();
    await expect(page.locator("body")).toHaveAttribute("data-hushed", "1");
  });
}

test("he is silent until he speaks: a key then does nothing", async ({ page }) => {
  await page.goto(lab("state=chat&open=1"));
  await page.keyboard.press("a");
  await expect(page.locator("body")).not.toHaveAttribute("data-hushed");
});
