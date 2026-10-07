import { expect, test } from "@playwright/test";

// Mute, pin and hide a project from a session's quick actions (C2). The lab plays core's part.
const lab = (params: string) => `/lab/?still=1&t=1500&${params}`;
const rows = (page: import("@playwright/test").Page) => page.locator(".flock-row .name");
const front = (page: import("@playwright/test").Page) => page.locator(".card.focus .who .name");

test("pin brings a project first, hide takes it off, mute flips its item", async ({ page }) => {
  await page.goto(lab("state=editing&open=1"));
  await expect(rows(page)).toHaveText(["site", "lazyagents"]);

  // Pinned, its session is the first at work: it takes the front.
  await page.locator(".flock-row", { hasText: "lazyagents" }).click({ button: "right" });
  await page.getByRole("button", { name: "Pin this project" }).click();
  await expect(front(page)).toHaveText("lazyagents");
  await expect(rows(page)).toHaveText(["vultures-ai", "site"]);

  await page.locator(".card.focus .focus-body").click({ button: "right" });
  await expect(page.getByRole("button", { name: "Unpin this project" })).toBeVisible();
  await page.getByRole("button", { name: "Mute this project" }).click();
  await page.locator(".card.focus .focus-body").click({ button: "right" });
  await expect(page.getByRole("button", { name: "Unmute this project" })).toBeVisible();
  await expect(page.locator("#island")).toHaveScreenshot("project-prefs-menu.png");

  await page.getByRole("button", { name: "Hide this project" }).click();
  await expect(page.locator("#island")).not.toContainText("lazyagents");
});
