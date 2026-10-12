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

// The world's tallest vultures in the habitat: each status mark stays inside the island.
test("island open: the world's tallest vultures in the list", async ({ page }) => {
  await page.goto("/lab/?still=1&t=1500&state=busy-flock&open=1&flock=world");
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
    const zecaPath = "/src/character/zeca/index.ts";
    const { ZECA } = await import(zecaPath);
    return { zeca: ZECA.clips.done.frames.length, atratus: of("atratus"), burrovianus: of("burrovianus") };
  });
  // Zeca's done clip, then the signature, then the done clip's last frame again.
  expect(lengths.atratus.done).toBe(lengths.zeca + lengths.atratus.signature + 1);
  expect(lengths.burrovianus.done).toBe(lengths.zeca);
});

// Dozing off, waking up, courtship and the full crop are built from each species' own frames:
// every frame must draw parts the set has, with or without a look, and the crop only on a perch.
test("the moment clips draw only parts each species has", async ({ page }) => {
  await page.goto("/lab/?still=1");
  const problems = await page.evaluate(async () => {
    const flockPath = "/src/character/flock/index.ts";
    const looksPath = "/src/character/looks.ts";
    const { speciesSet, SPECIES: all } = await import(flockPath);
    const { dress, LOOK_IDS } = await import(looksPath);
    const out: string[] = [];
    for (const s of all)
      for (const look of [null, ...LOOK_IDS]) {
        const set = dress(speciesSet(s.id), look);
        for (const name of ["doze", "wake", "court", "fed"]) {
          const clip = set.clips[name];
          if (!clip?.frames.length) out.push(`${s.id} ${look} ${name}: no frames`);
          for (const f of clip?.frames ?? []) {
            if (!(f.ms > 0)) out.push(`${s.id} ${look} ${name}: a frame of ${f.ms} ms`);
            for (const [p] of f.layers) if (!set.parts[p]) out.push(`${s.id} ${look} ${name}: no part ${p}`);
            const crop = f.layers.some(([p]: [string]) => p.startsWith("crop_"));
            if (crop && !f.layers.some(([p]: [string]) => p.startsWith("body"))) out.push(`${s.id} ${look} ${name}: crop off the perch`);
          }
        }
      }
    return out;
  });
  expect(problems).toEqual([]);
});
