// Zeca's looks, seasonal and not: each one in profile, facing you and in flight, and on the island's focus
// card, where he is drawn at 3x (on the empty wire: a session in front keeps its own bird).
import { expect, test } from "@playwright/test";

const LOOKS = [
  "witch-hat", "santa-hat", "party-hat", "bunny-ears", "sunglasses",
  "west-coast", "fitted-cap", "mountain-hat", "headband", "dreads",
  "front-knot", "durag", "crown", "bucket-hat", "clock-chain", "headphones", "shutter-shades", "chrome-chain", "eye-patch",
];

for (const look of LOOKS) {
  test(`look: ${look}`, async ({ page }) => {
    await page.goto(`/lab/?still=1&t=1500&look=${look}`);
    await page.locator("#scale").selectOption("2");
    for (const clip of ["idle", "approval", "fly"]) {
      const card = page.locator("#clips .clip-card").filter({ has: page.locator(`h3:text-is("${clip}")`) });
      await expect(card.locator("canvas")).toHaveScreenshot(`look-${look}-${clip}.png`);
    }
    await page.goto(`/lab/?still=1&t=1500&look=${look}&empty=1&open=1`);
    await expect(page.locator("#island")).toHaveScreenshot(`look-${look}-island.png`);
  });
}

// A hat lifts the top of his head: the chat permission's mark over it must stay inside the island.
test("look: the mark over a hatted Zeca", async ({ page }) => {
  await page.goto("/lab/?still=1&t=1500&look=witch-hat&state=chat-permission&open=1");
  await expect(page.locator("#island")).toHaveScreenshot("look-witch-hat-approval-island.png");
});

// A condor is Zeca's tallest choice: his hat still fits the folded island.
test("look: a condor Zeca in the compact island", async ({ page }) => {
  await page.goto("/lab/?still=1&t=1500&look=witch-hat&zeca=vultur&empty=1");
  await expect(page.locator("#island")).toHaveScreenshot("look-witch-hat-condor-compact.png");
});

// A look follows the species' eye: the condor's comb adds a row on top of every head, and the
// shades still cover the eye in profile, facing you and in flight.
test("look: the condor's eye", async ({ page }) => {
  await page.goto("/lab/?still=1&t=1500&look=sunglasses&species=vultur");
  await page.locator("#scale").selectOption("2");
  for (const clip of ["idle", "approval", "fly"]) {
    const card = page.locator("#clips .clip-card").filter({ has: page.locator(`h3:text-is("${clip}")`) });
    await expect(card.locator("canvas")).toHaveScreenshot(`look-sunglasses-condor-${clip}.png`);
  }
});

// Reading and thinking lean the head a cell left: the brim stays whole, the hat slips forward.
test("look: the brim while reading", async ({ page }) => {
  await page.goto("/lab/?still=1&t=1500&look=witch-hat");
  await page.locator("#scale").selectOption("2");
  for (const clip of ["read", "think"]) {
    const card = page.locator("#clips .clip-card").filter({ has: page.locator(`h3:text-is("${clip}")`) });
    await expect(card.locator("canvas")).toHaveScreenshot(`look-witch-hat-${clip}.png`);
  }
});

// A pendant hangs from the band and swings: on the dance's forward sway (a frame with dx > 0) it lags
// a cell behind; at rest it hangs straight.
test("look: the pendant swings", async ({ page }) => {
  for (const [name, t] of [["forward", 700], ["rest", 100]] as const) {
    await page.goto(`/lab/?still=1&t=${t}&look=clock-chain`);
    await page.locator("#scale").selectOption("2");
    const card = page.locator("#clips .clip-card").filter({ has: page.locator('h3:text-is("dance")') });
    await expect(card.locator("canvas")).toHaveScreenshot(`look-clock-chain-dance-${name}.png`);
  }
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
