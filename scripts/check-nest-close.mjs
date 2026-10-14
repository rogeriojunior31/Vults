// Exercise the production close handler without starting a desktop or browser server.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";

const root = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({ configFile: false, root, logLevel: "silent", build: {
  write: false, minify: false,
  lib: { entry: `${root}src/surfaces/nest/close.ts`, formats: ["es"] },
} });
const chunk = bundles[0].output.find(part => part.type === "chunk");
const { savedClose, savedTransition } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
let release, saves = 0, closes = 0, errors = 0, prevented = 0;
let saveFails = false, closeFails = false;
const event = { preventDefault() { prevented++; } };
const close = savedClose(async () => {
  saves++;
  if (saveFails) throw new Error("Save unavailable.");
  await new Promise(resolve => { release = resolve; });
}, async () => {
  if (closeFails) throw new Error("Close unavailable.");
  closes++;
}, () => { errors++; });

const pending = close(event);
assert.equal(prevented, 1, "Cancel system destruction synchronously before saving");
await close(event);
assert.equal(prevented, 2, "Repeated requests must also cancel system destruction");
assert.equal(saves, 1);
assert.equal(closes, 0, "Keep the window and its unsent draft until saving finishes");
release();
await pending;
assert.equal(closes, 1);
assert.equal(errors, 0);

saveFails = true;
await close(event);
assert.equal(closes, 1, "Failed saves must leave the window open");
assert.equal(errors, 1);
saveFails = false;
closeFails = true;
const refused = close(event);
release();
await refused;
assert.equal(closes, 1);
assert.equal(errors, 2, "Native close failures must be reported too");
closeFails = false;
const retry = close(event);
release();
await retry;
assert.equal(closes, 2, "A failed close must allow another attempt");
assert.equal(saves, 4);
const changes = [];
let ready, transitionFails = false;
const transition = savedTransition(async () => {
  if (transitionFails) throw new Error("Save unavailable.");
  await new Promise(resolve => { ready = resolve; });
}, () => { errors++; });
const quick = transition(async () => { changes.push("quick"); });
await transition(async () => { changes.push("full"); });
assert.deepEqual(changes, [], "Competing size requests must not bypass a pending save");
ready(); await quick;
assert.deepEqual(changes, ["quick"]);
transitionFails = true;
await transition(async () => { changes.push("mini"); });
assert.equal(errors, 3);
assert.deepEqual(changes, ["quick"]);
transitionFails = false;
const full = transition(async () => { throw new Error("Native resize unavailable."); });
ready(); await full;
assert.equal(errors, 4);
const recovered = transition(async () => { changes.push("room"); });
ready(); await recovered;
assert.deepEqual(changes, ["quick", "room"]);
// A programmatic size change can trigger system CloseRequested before the size command settles.
const nested = savedTransition(async () => {}, () => { errors++; });
const nestedPending = nested(() => close(event));
await new Promise(resolve => setImmediate(resolve));
assert.equal(closes, 2);
release(); await nestedPending;
assert.equal(closes, 3, "A size command's nested close must not be dropped by its pending guard");
console.log("check-nest-close: ok (immediate cancellation, deferred saves, repeated requests, save/close failure, retry)");
