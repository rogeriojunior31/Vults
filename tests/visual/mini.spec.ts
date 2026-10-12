import { expect, test } from "@playwright/test";

// Mini, the pill (ADR 0021): only the flock, no text; the edge pulses amber when someone needs you;
// a click on a vult opens the island on its session.
const lab = (params: string) => `/lab/?still=1&t=1500&${params}`;

test("mini: no text, and someone needs you", async ({ page }) => {
  await page.goto(lab("state=busy-flock"));
  const pill = page.locator("#island .compact");
  await expect(pill).toHaveClass(/(^|\s)needs(\s|$)/);
  // Nothing to read in the pill; its label says it.
  expect((await pill.innerText()).trim()).toBe("");
  await expect(pill).toHaveAttribute("aria-label", /.+/);
  await expect(page.locator("#island")).toHaveScreenshot("mini-needs-you.png");
});

test("mini: a quiet flock has no amber edge", async ({ page }) => {
  await page.goto(lab("state=idle-flock"));
  await expect(page.locator("#island .compact")).not.toHaveClass(/(^|\s)needs(\s|$)/);
  await expect(page.locator("#island")).toHaveScreenshot("mini-idle.png");
});

test("mini: a click on a vult opens the island on its session", async ({ page }) => {
  await page.goto(lab("state=busy-flock"));
  const box = (await page.locator("#island .compact").boundingBox())!;
  // The first vult roosts at the right end of the wire (scene.ts, COMPACT_SCENE).
  await page.mouse.click(box.x + 340 - 14 - 12, box.y + 24);
  await expect(page.locator("#island")).toHaveClass(/is-open/);
  await expect(page.locator("body")).toHaveAttribute("data-focus", /^[bcde]$/);
});
