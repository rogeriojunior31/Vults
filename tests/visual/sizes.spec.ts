import { expect, test } from "@playwright/test";

// The size switch (ADR 0021): Mini, Quick, Room, Full, in the island's header and the Nest's.

test("island: the switch shows Quick, and Room takes what it shows to the Nest", async ({ page }) => {
  await page.goto("/lab/?still=1&t=1500&state=busy-flock&open=1");
  const sizes = page.locator("#island .size-switch .size");
  await expect(sizes).toHaveText(["Mini", "Quick", "Room", "Full"]);
  await expect(page.locator('#island .size[data-size="quick"]')).toHaveAttribute("aria-pressed", "true");
  await page.locator('#island .size[data-size="room"]').click();
  await expect(page.locator("body")).toHaveAttribute("data-size", "room:flock");
  await page.locator('#island .size[data-size="full"]').click();
  await expect(page.locator("body")).toHaveAttribute("data-size", "full:flock");
});

test("island: from the chat, Room opens the Nest's Chat", async ({ page }) => {
  await page.goto("/lab/?still=1&t=1500&state=chat&open=1");
  await page.locator('#island .size[data-size="room"]').click();
  await expect(page.locator("body")).toHaveAttribute("data-size", "room:chat");
});

test("nest: the switch shows Room; Full, then Esc, comes back to it", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=tasks");
  await expect(page.locator('.nest-head .size[data-size="room"]')).toHaveAttribute("aria-pressed", "true");
  await page.locator('.nest-head .size[data-size="full"]').click();
  await expect(page.locator("body")).toHaveAttribute("data-size", "full:tasks");
  await expect(page.locator('.nest-head .size[data-size="full"]')).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-full.png");
  await page.keyboard.press("Escape");
  await expect(page.locator("body")).toHaveAttribute("data-size", "room:tasks");
  await expect(page.locator('.nest-head .size[data-size="room"]')).toHaveAttribute("aria-pressed", "true");
  // Quick keeps the place too.
  await page.locator('.nest-head .size[data-size="quick"]').click();
  await expect(page.locator("body")).toHaveAttribute("data-size", "quick:tasks");
});

test("nest: Alt and a number pick a tab, skipping a hidden Chat", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1&zeca=0");
  await page.keyboard.press("Alt+3");
  await expect(page.locator(".nest-tab.on")).toHaveText("Activity");
  await page.keyboard.press("Alt+4");
  await expect(page.locator(".nest-tab.on")).toHaveText("Connections");
});
