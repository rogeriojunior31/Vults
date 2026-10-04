// Zeca's seasonal looks: each one in profile, facing you and in flight, and on the island's focus
// card, where he is drawn at 3x.
import { expect, test } from "@playwright/test";

const LOOKS = ["witch-hat", "santa-hat", "party-hat", "bunny-ears", "sunglasses"];

for (const look of LOOKS) {
  test(`look: ${look}`, async ({ page }) => {
    await page.goto(`/lab/?still=1&t=1500&look=${look}`);
    await page.locator("#scale").selectOption("2");
    for (const clip of ["idle", "approval", "fly"]) {
      const card = page.locator("#clips .clip-card").filter({ has: page.locator(`h3:text-is("${clip}")`) });
      await expect(card.locator("canvas")).toHaveScreenshot(`look-${look}-${clip}.png`);
    }
    await page.goto(`/lab/?still=1&t=1500&look=${look}&state=editing&open=1`);
    await expect(page.locator("#island")).toHaveScreenshot(`look-${look}-island.png`);
  });
}

// A hat lifts the top of his head: the permission's mark over it must stay inside the card.
test("look: the mark over a hatted Zeca", async ({ page }) => {
  await page.goto("/lab/?still=1&t=1500&look=witch-hat&state=approval&open=1");
  // The approval state shows once it holds (render.ts SETTLE_MS).
  await page.waitForTimeout(1700);
  await expect(page.locator("#island")).toHaveScreenshot("look-witch-hat-approval-island.png");
});

// A condor is Zeca's tallest choice: his hat still fits the folded island.
test("look: a condor Zeca in the compact island", async ({ page }) => {
  await page.goto("/lab/?still=1&t=1500&look=witch-hat&zeca=vultur&state=editing");
  await expect(page.locator("#island")).toHaveScreenshot("look-witch-hat-condor-compact.png");
});

// Only Zeca dresses up: the vults keep their feathers, and no look leaves him undressed.
test("a look is baked into Zeca's heads and flight frames only", async ({ page }) => {
  await page.goto("/lab/?still=1");
  const result = await page.evaluate(async () => {
    const { speciesSet } = await import("/src/character/flock/index.ts" as string);
    const { dress, LOOK_IDS } = await import("/src/character/looks.ts" as string);
    const plain = speciesSet("atratus");
    const witch = dress(plain, "witch-hat");
    const changed = Object.keys(plain.parts).filter((p) => plain.parts[p].join() !== witch.parts[p].join());
    return {
      ids: LOOK_IDS,
      changed: changed.every((p: string) => p.startsWith("head") || ["fly_up", "glide", "fly_down"].includes(p)),
      everyHead: Object.keys(plain.parts).filter((p) => p.startsWith("head")).every((p: string) => changed.includes(p)),
      same: dress(plain, "witch-hat") === witch,
      none: dress(plain, null) === plain && dress(plain, "top-hat") === plain,
    };
  });
  expect(result.ids).toEqual(LOOKS);
  expect(result).toMatchObject({ changed: true, everyHead: true, same: true, none: true });
});
