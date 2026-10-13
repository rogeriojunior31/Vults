// Checks the production dock module without a browser, Lab, or test dependency.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";

const root = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({
  configFile: false,
  root,
  logLevel: "silent",
  build: {
    write: false,
    minify: false,
    lib: { entry: `${root}src/island/dock.ts`, formats: ["es"] },
  },
});
const chunk = bundles[0].output.find((part) => part.type === "chunk");
globalThis.window = { innerHeight: 640 };
globalThis.document = { body: { classList: { toggle() {} } } };
globalThis.localStorage = { setItem() {} };
const dock = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
let region;

// The initial measurement can happen before the bridge is attached.
dock.layout(110, 0, 500, 600, false);
dock.routeLayout((...rect) => { region = rect; });
assert.deepEqual(region, [110, 0, 500, 600]);
dock.setPlace({ presence: "panel", dock: "bottom" }, () => false);
assert.deepEqual(region, [110, 40, 500, 600]);
window.innerHeight = 800;
dock.layout(110, 0, 500, 600, false);
assert.deepEqual(region, [110, 200, 500, 600]);
// Logical dimensions arrive with CSS zoom already included; the dock must not scale twice.
dock.layout(189, 0, 502, 634, false);
assert.deepEqual(region, [189, 166, 502, 634]);
dock.layout(189, 0, 502, 634, true);
assert.deepEqual(region, [0, 0, 0, 0]);
dock.setPlace({ presence: "island", dock: "top" }, () => false);
assert.deepEqual(region, [189, 0, 502, 634]);
console.log("check-island-geometry: ok (initial region, anchors, viewport height, zoom, hidden panel)");
