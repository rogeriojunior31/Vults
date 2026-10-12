import { expect, test } from "@playwright/test";

// The size switch (ADR 0021): Mini, Quick, Room, Full, in the island's header and the Nest's.

test("island: the switch shows Quick, and Room takes what it shows to the Nest", async ({ page }) => {
  await page.goto("/lab/?still=1&t=1500&state=busy-flock&open=1");
  const sizes = page.locator("#island .size-switch .size");
  // L7: Full is the button beside the switch, as in the prototype.
  await expect(sizes).toHaveText(["Mini", "Quick", "Room"]);
  await expect(page.locator('#island .size[data-size="quick"]')).toHaveAttribute("aria-pressed", "true");
  await page.locator('#island .size[data-size="room"]').click();
  await expect(page.locator("body")).toHaveAttribute("data-size", "room:flock");
  await page.locator("#island .island-fullscreen").click();
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
  await expect(page.locator('.app-header .size[data-size="room"]')).toHaveAttribute("aria-pressed", "true");
  await page.locator(".nest-fullscreen").click();
  await expect(page.locator("body")).toHaveAttribute("data-size", "full:tasks");
  await expect(page.locator(".nest-fullscreen")).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-full.png");
  await page.keyboard.press("Escape");
  await expect(page.locator("body")).toHaveAttribute("data-size", "room:tasks");
  await expect(page.locator('.app-header .size[data-size="room"]')).toHaveAttribute("aria-pressed", "true");
  // From Full, Room (shown pressed, as in the prototype) still goes back to the window.
  await page.locator(".nest-fullscreen").click();
  await expect(page.locator("body")).toHaveAttribute("data-size", "full:tasks");
  await page.locator('.app-header .size[data-size="room"]').click();
  await expect(page.locator("body")).toHaveAttribute("data-size", "room:tasks");
  // Quick keeps the place too.
  await page.locator('.app-header .size[data-size="quick"]').click();
  await expect(page.locator("body")).toHaveAttribute("data-size", "quick:tasks");
});

test("nest: Alt and a number pick a tab, skipping a hidden Chat", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1&zeca=0");
  await page.keyboard.press("Alt+3");
  await expect(page.locator(".suite-tab.on .nav-label")).toHaveText("Activity");
  await page.keyboard.press("Alt+4");
  await expect(page.locator(".suite-tab.on .nav-label")).toHaveText("Connections");
});

// Quick's short views (ADR 0021): this week and the gear's settings, each with a way to the Nest.
test("island: Quick's settings, and Settings' other pages in the Nest", async ({ page }) => {
  await page.goto("/lab/?still=1&t=1500&state=idle-flock&open=1");
  await page.getByRole("button", { name: "Settings" }).click();
  await expect(page.locator("#island .quick-setting")).toHaveCount(2);
  await expect(page.locator("#island")).toHaveScreenshot("quick-settings.png");
  // Presence unfolds in place (the island takes the mouse only over itself) and changes at once.
  await page.locator("#island .quick-setting").first().locator(".quick-pick-btn").click();
  await page.getByRole("button", { name: "Quiet" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-opened", "presence quiet");
  await expect(page.locator("#island .quick-setting").first().locator(".quick-pick-btn")).toHaveText("Quiet");
  // Another page opens Settings in the Nest.
  await page.locator("#island .quick-settings-head .quick-pick-btn").click();
  await page.getByRole("button", { name: "Agents" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-opened", "settings agents");
});

test("island: Quick's week", async ({ page }) => {
  await page.goto("/lab/?still=1&t=1500&state=idle-flock&open=1");
  await page.getByRole("button", { name: "Activity" }).click();
  await expect(page.locator("#island .quick-stat strong").first()).toHaveText("6 h 20 min");
  await expect(page.locator("#island .quick-bar")).toHaveCount(7);
  await expect(page.locator("#island")).toHaveScreenshot("quick-week.png");
  // Room from here keeps the place.
  await page.locator('#island .size[data-size="room"]').click();
  await expect(page.locator("body")).toHaveAttribute("data-size", "room:activity");
});
