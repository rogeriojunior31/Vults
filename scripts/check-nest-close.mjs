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
const { savedClose } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
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
console.log("check-nest-close: ok (immediate cancellation, deferred saves, repeated requests, save/close failure, retry)");
