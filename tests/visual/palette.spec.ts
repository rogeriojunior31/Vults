import { expect, test } from "@playwright/test";

// The palette (Ctrl+K, ADR 0021, plan-zeca K9): sessions, commands and the Nest's tabs. It never
// answers a card (D1): a session that waits only leads to its card.

test("nest: the palette jumps to a session, starts a command, opens a tab", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1");
  await page.keyboard.press("Control+k");
  const palette = page.locator(".palette");
  await expect(palette).toBeVisible();
  await expect(palette.locator(".palette-foot")).toHaveText("Cards are answered on the card itself or on the phone, never from here.");
  await expect(page.locator("#nest")).toHaveScreenshot("palette-nest.png");
  // No Allow or Deny, anywhere in it.
  await expect(palette.getByRole("button", { name: "Allow" })).toHaveCount(0);
  // A waiting session leads to its card, on the Flock.
  await page.keyboard.type("quick");
  await page.keyboard.press("Enter");
  await expect(page.locator("body")).toHaveAttribute("data-review", "a");
  await expect(palette).toBeHidden();
  // A command lands in the Chat, for the user to finish.
  await page.keyboard.press("Control+k");
  await page.keyboard.type("/task");
  await page.keyboard.press("Enter");
  await expect(page.locator(".suite-tab.on .nav-label")).toHaveText("Chat");
  await expect(page.locator(".nest-chat textarea").first()).toHaveValue("/task ");
  // A tab.
  await page.keyboard.press("Control+k");
  await page.keyboard.type("Connections");
  await page.keyboard.press("Enter");
  await expect(page.locator(".suite-tab.on .nav-label")).toHaveText("Connections");
});

test("nest: without Zeca the palette offers no commands", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1&zeca=0");
  await page.keyboard.press("Control+k");
  await page.keyboard.type("/task");
  await expect(page.locator(".palette-item")).toHaveCount(0);
  await page.keyboard.press("Escape");
  await expect(page.locator(".palette")).toBeHidden();
});

// L7: Quick's header, as the prototype draws it: the search opens the same palette as Ctrl+K, and
// the Flock tab counts the sessions that need you.
test("island: the header's search opens the palette", async ({ page }) => {
  await page.goto("/lab/?still=1&t=1500&state=busy-flock&open=1");
  await expect(page.locator("#island .header-brand .app-logo")).toHaveText("vults");
  await page.getByRole("button", { name: "Command palette (Ctrl+K)" }).click();
  await expect(page.locator("#island .palette")).toBeVisible();
  await page.keyboard.press("Escape");
  await page.goto("/lab/?still=1&t=1500&state=big-flock&open=1");
  await expect(page.locator('#island .tab[aria-label^="Flock "] .tab-count')).toBeVisible();
});

test("island: the palette in Quick, and Esc puts the island back", async ({ page }) => {
  await page.goto("/lab/?still=1&t=1500&state=busy-flock&open=1");
  await page.keyboard.press("Control+k");
  await expect(page.locator("#island .palette")).toBeVisible();
  await expect(page.locator("#island")).toHaveScreenshot("palette-island.png");
  await page.keyboard.type("Tasks");
  await page.keyboard.press("Enter");
  await expect(page.locator("body")).toHaveAttribute("data-size", "room:tasks");
  await page.keyboard.press("Control+k");
  await page.keyboard.press("Escape");
  await expect(page.locator("#island .palette")).toHaveCount(0);
});
