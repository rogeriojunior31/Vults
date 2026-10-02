import { expect, test } from "@playwright/test";

test("flight comparison scrubs, lands and respects reduced motion", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/lab/flight/");
  await expect(page.getByRole("button", { name: "Play", exact: true })).toBeVisible();
  const pixels = () => page.locator("canvas").evaluateAll(nodes => nodes.map(node => (node as HTMLCanvasElement).toDataURL()));
  const rest = await pixels();
  for (const time of [180, 1500, 1800, 4900, 8000, 11000, 12000, 13000, 14000]) {
    await page.locator("#seek").fill(String(time));
    await expect(page.locator("#time")).toHaveText(`${(time / 1000).toFixed(2)} s`);
    const frames = await pixels();
    expect(frames.every(frame => frame.length > 1000)).toBe(true);
    if (time === 8000) {
      expect(frames[0]).not.toBe(rest[0]);
      expect(frames[1]).not.toBe(rest[1]);
    }
  }
  await page.locator("#seek").fill("8000");
  const first = await pixels();
  await page.locator("#seek").fill("12000");
  await page.locator("#seek").fill("8000");
  expect(await pixels()).toEqual(first);
  await page.getByRole("button", { name: "Restart" }).click();
  expect(await pixels()).toEqual(rest);
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.getByRole("button", { name: "Play", exact: true }).click();
  await expect(page.locator("#time")).not.toHaveText("0.00 s");
  expect(errors).toEqual([]);
});
