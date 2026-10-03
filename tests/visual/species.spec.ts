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
