import { expect, test, type Page } from "@playwright/test";

// A project's bird (plan-breeds-and-flocks R2): a session's quick action opens every species but
// the king's; a pick is the whole project's, and Automatic goes back to the pool's draw.
const lab = (params: string) => `/lab/?still=1&t=1500&${params}`;

type Lab = { island: { openMenu(a: string, id: string): void; last(): { sessions: { id: string; species: string; bird_chosen: boolean }[] } } };
const session = (page: Page, id: string) =>
  page.evaluate((id) => (window as unknown as Lab).island.last().sessions.find((s) => s.id === id)!, id);
const openBirds = async (page: Page) => {
  await page.evaluate(() => (window as unknown as Lab).island.openMenu("claude", "lab"));
  await page.getByRole("button", { name: "This project's bird…" }).click();
  await expect(page.locator(".birds")).toBeVisible();
};

test("a project's bird is picked from every species but the king's", async ({ page }) => {
  await page.goto(lab("state=live-diff&open=1"));
  await openBirds(page);
  await expect(page.locator(".birds .bird")).toHaveCount(22);
  await expect(page.getByRole("button", { name: "King vulture" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Automatic" })).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator("#island")).toHaveScreenshot("birds.png");

  await page.getByRole("button", { name: "Andean condor" }).click();
  await expect(page.locator(".birds")).toHaveCount(0);
  expect(await session(page, "lab")).toMatchObject({ species: "vultur", bird_chosen: true });

  // Marked the next time, and Automatic gives it back to the draw.
  await openBirds(page);
  await expect(page.getByRole("button", { name: "Andean condor" })).toHaveAttribute("aria-pressed", "true");
  await page.getByRole("button", { name: "Automatic" }).click();
  expect(await session(page, "lab")).toMatchObject({ bird_chosen: false });
});

test("Escape closes the birds without a pick", async ({ page }) => {
  await page.goto(lab("state=live-diff&open=1"));
  const before = await session(page, "lab");
  await openBirds(page);
  await page.keyboard.press("Escape");
  await expect(page.locator(".birds")).toHaveCount(0);
  expect(await session(page, "lab")).toMatchObject({ species: before.species, bird_chosen: false });
});
