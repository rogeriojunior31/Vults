// Tokens read as t3code writes them (the unit chosen after rounding), and a plan window's pace.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";

globalThis.window = globalThis;
const root = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({ configFile: false, root, logLevel: "silent", build: {
  write: false, minify: false, lib: { entry: `${root}src/cost.ts`, formats: ["es"] },
} });
const chunk = bundles[0].output.find(part => part.type === "chunk");
const { tokens, pace, paceLabel, usageWindowLabel } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);

assert.equal(tokens(0), "0");
assert.equal(tokens(950), "950");
assert.equal(tokens(999.6), "1K", "rounds into the next unit");
assert.equal(tokens(1250), "1.25K");
assert.equal(tokens(12_500), "12.5K");
assert.equal(tokens(125_000), "125K");
assert.equal(tokens(999_999), "1M", "never 1000K");
assert.equal(tokens(1_200_000), "1.2M");
assert.equal(tokens(3_000_000_000), "3B");

// A 5-hour window, half gone.
const now = 1_000_000;
const half = used => ({ minutes: 300, used_percent: used, resets_at: now + 150 * 60 });
assert.equal(pace(half(50), now), "on");
assert.equal(pace(half(56), now), "ahead");
assert.equal(pace(half(44), now), "under");
assert.equal(pace({ minutes: 300, used_percent: 50, resets_at: null }, now), null, "no reset, no pace");
assert.equal(paceLabel("ahead"), "Ahead of pace");
assert.equal(usageWindowLabel({ minutes: 300 }), "5 hours");
assert.equal(usageWindowLabel({ minutes: 10080, scope: "Opus" }), "Opus, weekly", "a model's own weekly row");
assert.equal(usageWindowLabel({ minutes: 43200, scope: "spend" }), "Monthly spend");
assert.equal(usageWindowLabel({ minutes: 43200, scope: "credits" }), "Monthly credit limit");
console.log("check-usage-format: tokens as 950, 1.25K, 12.5K, 1M (never 1000K), 3B; pace ahead, on or under the time gone; scoped windows named");
