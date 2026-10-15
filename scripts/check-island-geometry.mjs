// Checks the production dock module without a browser, Lab, or test dependency.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { build } from "vite";

const root = fileURLToPath(new URL("../ui/", import.meta.url));
const bundle = async entry => {
  const result = await build({ configFile: false, root, logLevel: "silent", build: {
    write: false, minify: false, lib: { entry: `${root}src/island/${entry}.ts`, formats: ["es"] },
  } });
  const chunk = result[0].output.find(part => part.type === "chunk");
  return import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
};
const { quickBounds, quickPixels, OPEN_ZOOM } = await bundle("geometry");
const native = JSON.parse(readFileSync(new URL("../app/tauri.conf.json", import.meta.url), "utf8")).app.windows.find(window => window.label === "island");
assert.deepEqual([native.width, native.height], [720, 640], "Initial native size matches the mapped overlay");
const runtime = readFileSync(new URL("../app/src/runtime.rs", import.meta.url), "utf8");
assert.ok(runtime.includes("ISLAND_SIZE: (i32, i32) = (720, 640)"), "Native runtime and initial configuration share the same bounds");
assert.deepEqual(quickBounds(720, 640), { width: 440, chatWidth: 492, height: 556 });
assert.deepEqual(quickPixels(492, 556), { w: 561, h: 634 });
assert.deepEqual(quickPixels(440, 556), { w: 502, h: 634 }, "Other Quick tabs retain their dimensions");
for (const [width, height] of [[720, 640], [720, 560], [560, 480], [480, 360], [320, 360]]) {
  const bounds = quickBounds(width, height);
  for (const cssWidth of [bounds.width, bounds.chatWidth]) {
    const rect = quickPixels(cssWidth, bounds.height);
    assert.ok(rect.w <= width && rect.h <= height, "Zoomed content stays within the mapped surface");
    for (const scale of [1, 1.25, 1.5, 2]) {
      assert.ok(Math.ceil(rect.w * scale) <= Math.ceil(width * scale));
      assert.ok(Math.ceil(rect.h * scale) <= Math.ceil(height * scale));
    }
  }
}
assert.deepEqual(quickBounds(0, 0), { width: 0, chatWidth: 0, height: 0 });
assert.ok(OPEN_ZOOM > 1);
globalThis.window = { innerHeight: 640 };
globalThis.document = { body: { classList: { toggle() {} } } };
globalThis.localStorage = { setItem() {} };
const dock = await bundle("dock");
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

// The same measured chat rectangle is used by either dock anchor, including after a resize.
window.innerHeight = 640;
const chat = quickPixels(492, 556);
const x = Math.floor((720 - chat.w) / 2);
dock.layout(x, 0, chat.w, chat.h, false);
assert.deepEqual(region, [79, 0, 561, 634]);
dock.setPlace({ presence: "panel", dock: "bottom" }, () => false);
assert.deepEqual(region, [79, 6, 561, 634]);
dock.layout(x, 0, chat.w, chat.h, true);
assert.deepEqual(region, [0, 0, 0, 0], "A hidden wider chat still lets clicks through");
console.log("check-island-geometry: wider Chat, retained Quick width, bounded viewport/zoom at 1/1.25/1.5/2 display scales, exact top/bottom regions");
