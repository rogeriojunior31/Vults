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
const { quickBounds, quickPixels, miniHeight, watchWidth, miniSessions, miniWidth, islandWidth, restoreWidth, saveWidth, OPEN_ZOOM } = await bundle("geometry");
const native = JSON.parse(readFileSync(new URL("../app/tauri.conf.json", import.meta.url), "utf8")).app.windows.find(window => window.label === "island");
assert.deepEqual([native.width, native.height], [880, 640], "Initial native size matches the mapped overlay");
const runtime = readFileSync(new URL("../app/src/runtime.rs", import.meta.url), "utf8");
assert.ok(runtime.includes("ISLAND_SIZE: (i32, i32) = (880, 640)"), "Native runtime and initial configuration share the same bounds");
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
for (const preferred of [560, 680, 760]) {
  for (const available of [880, 720, 600, 480, 320]) {
    const bounds = quickBounds(available, 640, 0, preferred);
    const quick = quickPixels(bounds.width, bounds.height);
    assert.equal(bounds.width, bounds.chatWidth, "Selected width is shared across Quick tabs");
    assert.ok(quick.w <= available);
    assert.ok(Math.abs(quick.w - miniWidth(available, preferred)) <= 1, "Mini and Quick share logical width");
    assert.equal(miniWidth(available, preferred), Math.min(available, preferred));
  }
}
for (const preferred of [0, 560, 680, 760]) {
  for (const height of [0, 24, 38, 56, 360, 480, 640]) {
    for (const inset of [0, 24]) {
      const bounds = quickBounds(880, height, inset, preferred);
      const box = quickPixels(bounds.width, bounds.height, Math.min(height, inset));
      assert.ok(box.h <= height, "Quick and its feedback fit the available height");
      if (preferred) assert.ok(box.h <= 600, "Selected Quick proportions cap the whole box at 600 logical pixels");
      assert.equal(miniHeight(height), Math.min(height, 38));
    }
  }
}
assert.deepEqual(miniSessions(340, 8), { visible: 8, hidden: 0 });
assert.deepEqual(miniSessions(340, 9), { visible: 7, hidden: 2 }, "Overflow reserves its own target instead of covering a bird");
assert.deepEqual(miniSessions(320, 30), { visible: 6, hidden: 24 });
assert.deepEqual(miniSessions(560, 30), { visible: 8, hidden: 22 });
for (const width of [100, 240, 320, 340, 480, 560, 680, 760]) {
  for (const total of [0, 1, 6, 8, 9, 30]) {
    const count = miniSessions(width, total);
    assert.equal(count.visible + count.hidden, total);
    assert.ok(count.visible >= 0 && count.visible <= 8);
    if (count.visible) assert.ok(52 + (count.visible - 1) * 32 + 38 + (count.hidden ? 40 : 0) <= width);
  }
}
for (const invalid of [null, "760", 561, -1, Infinity, {}, []]) assert.equal(islandWidth(invalid), 0);
const kept = new Map();
globalThis.localStorage = { getItem: key => kept.get(key) ?? null, setItem: (key, value) => kept.set(key, value) };
assert.equal(restoreWidth(), 0);
assert.equal(saveWidth(680), true);
assert.equal(restoreWidth(), 680, "Selected width survives recreation");
kept.set("island-width", "bad-json");
assert.equal(restoreWidth(), 0);
globalThis.localStorage = { getItem() { throw new Error("unavailable"); }, setItem() { throw new Error("full"); } };
assert.equal(restoreWidth(), 0);
assert.equal(saveWidth(760), false, "Unavailable storage must not promise a saved preference");
const widthEvents = {}, changes = [];
globalThis.window = { addEventListener: (name, callback) => { widthEvents[name] = callback; } };
globalThis.localStorage = { getItem: key => kept.get(key) ?? null, setItem: (key, value) => kept.set(key, value) };
kept.set("island-width", "560");
watchWidth(value => changes.push(value));
kept.set("island-width", "760");
widthEvents.storage({ key: "other-setting" }); assert.deepEqual(changes, []);
widthEvents.storage({ key: "island-width" }); assert.deepEqual(changes, [760]);
widthEvents.focus(); assert.deepEqual(changes, [760], "Unchanged width must not redraw");
kept.set("island-width", "680"); widthEvents.focus();
assert.deepEqual(changes, [760, 680], "Refocus recovers a missed cross-view notification");
kept.delete("island-width"); widthEvents.storage({ key: null });
assert.deepEqual(changes, [760, 680, 0], "Clearing presentation state restores Default");
saveWidth(560);
kept.delete("island-width"); widthEvents.storage({ key: "island-width" });
assert.deepEqual(changes, [760, 680, 0, 0], "An external change must win even when it returns to the watcher's original value");
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
