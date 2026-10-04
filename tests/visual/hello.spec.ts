// The hello opens with a landing squash and a bounce. It is drawn with Zeca's own parts, so every
// species rig and every seasonal look must still have them, and the squash must keep the feet on
// the wire.
import { expect, test } from "@playwright/test";

test("the hello's landing squash works for every species and look", async ({ page }) => {
  await page.goto("/lab/?still=1");
  const problems = await page.evaluate(async () => {
    const { speciesSet, SPECIES } = await import("/src/character/flock/index.ts" as string);
    const { dress, LOOK_IDS } = await import("/src/character/looks.ts" as string);
    const out: string[] = [];
    for (const s of SPECIES)
      for (const look of [null, ...LOOK_IDS]) {
        const set = dress(speciesSet(s.id), look);
        const frames = set.clips.hello.frames;
        const at = `${s.id}/${look ?? "plain"}`;
        for (const f of frames)
          for (const [p] of f.layers) if (!set.parts[p]) out.push(`${at}: no part ${p}`);
        // The squash: the fluffed body sunk, its legs under it and where the resting legs are.
        const [squash] = frames;
        const rest = set.clips.idle.frames[0];
        const legs = (f: typeof rest) => f.layers.find(([p]: [string]) => p === "legs");
        const order = squash.layers.map(([p]: [string]) => p);
        if (squash.dy <= 0 || order.indexOf("legs") > order.indexOf("body_puff")) out.push(`${at}: no squash`);
        if (squash.dy + legs(squash)[2] !== rest.dy + legs(rest)[2]) out.push(`${at}: feet off the wire`);
        // The bounce leaves the wire.
        if (!frames.some((f: typeof rest) => f.dy < 0)) out.push(`${at}: no bounce`);
      }
    return out;
  });
  expect(problems).toEqual([]);
});
