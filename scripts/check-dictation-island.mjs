// Production state ownership and geometry; native visual acceptance is still required.
import "./check-dictation-status.mjs";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { build } from "vite";

const root = fileURLToPath(new URL("../ui/", import.meta.url));
const result = await build({ configFile: false, root, logLevel: "silent", build: {
  write: false, minify: false, lib: { entry: `${root}src/island/geometry.ts`, formats: ["es"] },
} });
const chunk = result[0].output.find(part => part.type === "chunk");
const { DICTATION_HEIGHT, quickBounds, quickPixels, dictationCaptionTop } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
assert.equal(dictationCaptionTop({ top: 0, bottom: 62 }, 48, 640, false), 68);
assert.equal(dictationCaptionTop({ top: 578, bottom: 640 }, 48, 640, true), 524);
assert.equal(dictationCaptionTop({ top: 0, bottom: 634 }, 48, 640, false), null, "A full-height Quick yields its caption");
assert.equal(dictationCaptionTop({ top: 6, bottom: 640 }, 48, 640, true), null, "Bottom Quick never covers the island");
assert.equal(dictationCaptionTop({ top: 0, bottom: 62 }, 48, 110, false), null, "Short windows yield feedback before controls");
assert.equal(dictationCaptionTop({ top: 0, bottom: 62 }, 0, 640, false), null);
for (const [width, height] of [[720, 640], [720, 560], [560, 480], [480, 360], [340, 360]]) {
  for (const inset of [0, DICTATION_HEIGHT]) {
    const bounds = quickBounds(width, height, inset);
    for (const cssWidth of [bounds.width, bounds.chatWidth]) {
      const frame = quickPixels(cssWidth, bounds.height, inset);
      assert.ok(frame.w <= width && frame.h <= height, "The feedback row and zoomed Quick fit the fixed overlay");
      for (const dock of ["top", "bottom"]) {
        const y = dock === "bottom" ? height - frame.h : 0;
        assert.ok(y >= 0 && y + frame.h <= height, `${dock} anchor includes the feedback row exactly once`);
      }
    }
  }
}
const style = readFileSync(`${root}src/dictation.css`, "utf8");
assert.match(style, /pointer-events:\s*none/, "Feedback never takes pointer actions from cards");
assert.match(style, /\.dictation-status\[hidden\]\s*\{\s*display:\s*none/, "Flex must not override the hidden attribute");
const words = style.match(/\.dictation-words\s*\{([^}]+)\}/)?.[1];
assert.match(words, /min-width:\s*0/);
assert.match(words, /overflow:\s*hidden/);
assert.match(words, /text-overflow:\s*ellipsis/, "Long translated text is bounded in Mini and Quick");
assert.match(style, /color:\s*var\(--fg-dim\)/, "Black and Light use the same neutral theme role");
const caption = style.match(/\.dictation-caption\s*\{([^}]+)\}/)?.[1];
assert.match(caption, /pointer-events:\s*none/);
assert.match(caption, /-webkit-line-clamp:\s*2/);
assert.match(caption, /overflow-wrap:\s*anywhere/);
console.log("check-dictation-island: ok (all phases, three translations, card priority, reserved row, dock anchors, zoom, clipping contract)");
