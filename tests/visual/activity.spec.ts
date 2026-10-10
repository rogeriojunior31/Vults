import { expect, test } from "@playwright/test";

// Settings → Activity (docs/dev/plan-activity.md, A4): a week's recap, the year's grid, and the
// history's switch, drawn by the lab with made-up weeks.
const lab = (params = "") => `/lab/activity.html?${params}`;

test("a full week, its numbers and the year's grid", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 1100 });
  await page.goto(lab());
  await expect(page.locator(".week-headline")).toHaveText("41 turns, 6 h 20 min with your agents, most on site.");
  await expect(page.locator(".stat")).toHaveCount(6);
  await expect(page.locator(".activity-grid .cell")).toHaveCount(52 * 7 + 5);
  await expect(page.locator(".week .card-title")).toHaveText("This week · Oct 5 – 11, 2026");
  await expect(page.locator(".page")).toHaveScreenshot("activity.png");
});

test("paging goes to the older week; the newest has nowhere newer", async ({ page }) => {
  await page.goto(lab());
  await expect(page.getByRole("button", { name: "Newer week" })).toBeDisabled();
  await page.getByRole("button", { name: "Older week" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-week", "2026-09-28");
  await page.goto(lab("week=2026-09-28"));
  await expect(page.locator(".week .card-title")).toHaveText("Sep 28 – Oct 4, 2026");
  await expect(page.getByRole("button", { name: "Newer week" })).toBeEnabled();
});

test("an empty history says so", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 900 });
  await page.goto(lab("state=empty"));
  await expect(page.locator(".week .note")).toHaveText("No agent turns that week.");
  await expect(page.locator(".activity-grid .cell.l0")).toHaveCount(52 * 7 + 5);
  await expect(page.locator(".page")).toHaveScreenshot("activity-empty.png");
});

test("clearing asks first, and the switch says what it does", async ({ page }) => {
  await page.goto(lab());
  await page.getByRole("button", { name: "Clear history…" }).click();
  await expect(page.getByText("Clear the history?")).toBeVisible();
  await page.getByRole("button", { name: "Cancel" }).click();
  await expect(page.getByText("Clear the history?")).toHaveCount(0);
  await page.getByRole("button", { name: "Clear history…" }).click();
  await page.getByRole("button", { name: "Clear history", exact: true }).click();
  await expect(page.locator("body")).toHaveAttribute("data-cleared", "1");
  await page.locator(".toggle").click();
  await expect(page.locator("body")).toHaveAttribute("data-history", "false");
});

test("the GitHub tab: the contribution calendar, as GitHub's levels say", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 1100 });
  await page.goto(lab());
  await page.getByRole("button", { name: "GitHub" }).click();
  await expect(page.locator(".activity-grid .cell")).toHaveCount(53 * 7 - 1);
  await expect(page.getByText(/contributions in the last year/)).toBeVisible();
  await expect(page.locator(".page")).toHaveScreenshot("activity-github.png");
  await page.getByRole("button", { name: "Agents" }).click();
  await expect(page.getByText(/turns in the last year/)).toBeVisible();
});

test("the GitHub tab without the connector, with an error, and while asking", async ({ page }) => {
  await page.goto(lab("tab=github&github=off"));
  await expect(page.getByText(/once the GitHub connector is on/)).toBeVisible();
  await expect(page.locator(".activity-grid")).toHaveCount(0);
  await page.getByRole("button", { name: "Open Connectors" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-opened", "connectors");
  await page.goto(lab("tab=github&github=error"));
  await expect(page.locator(".note.error")).toContainText("gh isn't logged in");
  await page.goto(lab("tab=github&github=loading"));
  await expect(page.getByText("Asking GitHub…")).toBeVisible();
});
