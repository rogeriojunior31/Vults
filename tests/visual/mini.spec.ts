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
  // The first vult roosts right after Zeca (scene.ts, compactAt).
  await page.mouse.click(box.x + 52 + 12, box.y + 24);
  await expect(page.locator("#island")).toHaveClass(/is-open/);
  // That vult's session is in front now.
  const id = await page.locator("body").getAttribute("data-focus");
  const project = { b: "site", c: "lazyagents", d: "docs", e: "api" }[id as "b" | "c" | "d" | "e"];
  expect(project).toBeTruthy();
  await expect(page.locator("#island .card.focus")).toContainText(project!);
});
