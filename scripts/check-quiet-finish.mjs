// Check the island's settled cue (FF3) without a browser server: a finish whose terminal was in
// front makes no sound and no reveal; a card always sounds and opens; Quiet, Paused and mute keep
// their meaning.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";

const root = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({ configFile: false, root, logLevel: "silent", build: {
  write: false, minify: false, lib: { entry: `${root}src/island/cues.ts`, formats: ["es"] },
} });
const chunk = bundles[0].output.find(part => part.type === "chunk");
const { settledCue } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);

const done = over => ({ id: "s", agent: "claude", status: "finished", attention: "done", ...over });
const card = { request: "r1" };

assert.deepEqual(settledCue(done(), "island"), { sound: true, reveal: true }, "a finish elsewhere is news");
assert.deepEqual(settledCue(done({ watched: true }), "island"), { sound: false, reveal: false }, "seen ending: quiet");
assert.deepEqual(settledCue(done({ status: "failed", attention: "failed", watched: true }), "island"), { sound: false, reveal: false });
assert.deepEqual(settledCue(done({ watched: true, card }), "island"), { sound: true, reveal: true }, "a card is never quieted");
assert.deepEqual(settledCue(done({ watched: true, card }), "quiet"), { sound: true, reveal: true });
assert.deepEqual(settledCue(done({ muted: true }), "island"), { sound: false, reveal: true }, "mute keeps the reveal as before");
assert.deepEqual(settledCue(done({ muted: true, card }), "island"), { sound: true, reveal: true });
assert.deepEqual(settledCue(done(), "quiet"), { sound: false, reveal: true });
assert.deepEqual(settledCue(done({ card }), "paused"), { sound: false, reveal: true });
assert.deepEqual(settledCue(undefined, "island"), { sound: true, reveal: true }, "a session gone keeps today's cue");

console.log("check-quiet-finish: ok");
