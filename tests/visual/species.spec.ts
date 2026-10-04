// Every species at one instant: its idle, its signature and its flight, as the lab draws them.
import { expect, test } from "@playwright/test";
import { SPECIES } from "../../ui/src/character/flock/species";

for (const s of SPECIES) {
  test(`species: ${s.name}`, async ({ page }) => {
    await page.goto(`/lab/?still=1&t=1500&species=${s.id}`);
    await page.locator("#scale").selectOption("2");
    for (const clip of ["idle", "signature", "fly"]) {
      const card = page.locator("#clips .clip-card").filter({ has: page.locator(`h3:text-is("${clip}")`) });
      await expect(card.locator("canvas")).toHaveScreenshot(`species-${s.id}-${clip}.png`);
    }
  });
}

// The world's tallest vultures in the flock list: each status mark stays inside its own row.
test("island open: the world's tallest vultures in the list", async ({ page }) => {
  await page.goto("/lab/?still=1&t=1500&island=6&open=1&flock=world");
  await expect(page.locator("#island")).toHaveScreenshot("island-world-list.png");
});

// A finished bird does its own thing before it rests: the signature follows the done clip, unless
// the signature happens in the air.
test("the done clip ends with the species' signature when it is perched", async ({ page }) => {
  await page.goto("/lab/?still=1");
  const lengths = await page.evaluate(async () => {
    const path = "/src/character/flock/index.ts";
    const { speciesSet } = await import(path);
    const of = (id: string) => {
      const set = speciesSet(id);
      return { done: set.clips.done.frames.length, signature: set.clips.signature.frames.length };
    };
    return { atratus: of("atratus"), burrovianus: of("burrovianus") };
  });
  // Zeca's done clip has 8 frames; then the signature, then the done clip's last frame again.
  expect(lengths.atratus.done).toBe(8 + lengths.atratus.signature + 1);
  expect(lengths.burrovianus.done).toBe(8);
});
