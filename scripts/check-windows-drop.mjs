// Exercises the production drag lifecycle without native input or a desktop session.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";

const root = fileURLToPath(new URL("../ui/", import.meta.url));
const result = await build({ configFile: false, root, logLevel: "silent", build: {
  write: false, minify: false, lib: { entry: `${root}src/windows-drop.ts`, formats: ["es"] },
} });
const chunk = result[0].output.find(part => part.type === "chunk");
const { installWindowsDrop } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);

const target = new EventTarget();
let ready = false;
let throws = false;
const hover = [], sent = [], failed = [];
installWindowsDrop(target, { postMessageWithAdditionalObjects(marker, objects) {
  if (throws) throw new Error("transport unavailable");
  sent.push({ marker, objects });
} }, () => ready, value => hover.push(value), files => failed.push(files));
function drag(type, files = [], types = ["Files"]) {
  const event = new Event(type, { cancelable: true });
  event.dataTransfer = { files, types, dropEffect: "uninitialized" };
  target.dispatchEvent(event);
  return event;
}
const file = { name: "fixture.txt" };
assert.equal(drag("dragover").defaultPrevented, true, "Unavailable runtime cannot navigate to a file");
assert.equal(drag("dragover").dataTransfer.dropEffect, "none");
drag("drop", [file]);
assert.equal(sent.length, 0);
ready = true;
assert.equal(drag("dragover").dataTransfer.dropEffect, "copy");
drag("dragenter"); drag("dragenter"); drag("dragleave");
assert.deepEqual(hover, [true], "Nested elements do not flicker the drop target");
drag("drop", [file]);
assert.deepEqual(hover, [true, false]);
assert.equal(sent[0].marker, "vults-file-drop-v1");
assert.equal(sent[0].objects[0], file, "Native objects pass through without fabricated path strings");
assert.equal(drag("drop", [], ["text/plain"]).defaultPrevented, false);
drag("drop", []);
assert.equal(sent.length, 1);
throws = true;
drag("drop", [file]);
assert.deepEqual(failed, [[file]], "Transport failure reports the existing attachment refusal");
throws = false;
drag("drop", Array(257).fill(file));
assert.equal(sent.length, 1, "An oversized batch is rejected without partial attachment");
drag("dragenter"); target.dispatchEvent(new Event("blur")); drag("dragleave");
assert.deepEqual(hover.slice(-2), [true, false], "Cancelled drags release hover state");
console.log("Windows file drop lifecycle checks passed");
