// Check the production Chat panel's failed model reads without a browser server.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";

class Element {
  children = [];
  listeners = {};
  attributes = {};
  dataset = {};
  className = "";
  style = {};
  value = "";
  classList = { add() {}, remove() {}, toggle() {} };
  constructor(tag) { this.tag = tag; }
  set textContent(text) { this.text = text; this.children = []; }
  get textContent() { return (this.text ?? "") + this.children.map(node => node.textContent).join(""); }
  append(...nodes) { this.children.push(...nodes); }
  prepend(...nodes) { this.children.unshift(...nodes); }
  replaceChildren(...nodes) { this.text = ""; this.children = nodes; }
  addEventListener(event, fn) { this.listeners[event] = fn; }
  setAttribute(name, value) { this.attributes[name] = value; }
  toggleAttribute() {}
  focus() { document.activeElement = this; }
  scrollTo() {}
  getBoundingClientRect() { return { height: 200 }; }
}
globalThis.document = { createElement: tag => new Element(tag), createElementNS: (_ns, tag) => new Element(tag) };
globalThis.window = globalThis;
globalThis.requestAnimationFrame = () => 0;
globalThis.getComputedStyle = () => ({ lineHeight: "20", borderTopWidth: "1", borderBottomWidth: "1" });
const root = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({ configFile: false, root, logLevel: "silent", build: {
  write: false, minify: false, lib: { entry: `${root}src/island/chat.ts`, formats: ["es"] },
} });
const chunk = bundles[0].output.find(part => part.type === "chunk");
const { ChatPanel } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
const nodes = element => [element, ...element.children.flatMap(nodes)];
const tick = () => new Promise(resolve => setImmediate(resolve));
let calls = 0, fail = true, sends = 0;
const panel = new ChatPanel({
  models: () => {
    calls++;
    assert.ok(calls <= 3, "A failed model read must not trigger an automatic retry loop");
    return fail ? Promise.reject(new Error("Models unavailable."))
      : Promise.resolve([{ id: "sonnet", label: "Sonnet", efforts: [] }]);
  },
  pick: async () => ({}),
  send: async () => { sends++; },
}, () => {});
await tick();
assert.equal(calls, 1);
const input = nodes(panel.element).find(node => node.tag === "textarea");
input.value = "Keep this draft";
input.listeners.input();
panel.relabel();
await tick();
assert.equal(calls, 1, "Typing and repainting must not repeat a failed read");
const picker = nodes(panel.element).find(node => node.className.split(" ").includes("model"));
assert.ok(picker, "The model picker must be accessible");
picker.listeners.click();
await tick();
assert.equal(calls, 2, "Explicitly opening the picker retries once");
assert.ok(panel.element.textContent.includes("did not list its models"));
picker.listeners.click();
fail = false;
picker.listeners.click();
await tick();
assert.equal(calls, 3);
assert.ok(panel.element.textContent.includes("Sonnet"));
assert.ok(!panel.element.textContent.includes("did not list its models"));
panel.relabel();
await tick();
assert.equal(calls, 3, "A successful read is reused");
assert.equal(input.value, "Keep this draft");
assert.equal(sends, 0);
console.log("check-chat-models: ok (failed read settles, repaint without retry, explicit retry, recovery, draft retained, no send)");

const kept = { text: "Saved before opening", files: ["/work/note.txt"], folder: "/work", provider: "codex" };
const conversation = { provider: "codex", label: "Codex", folder: "/work", started: true,
  model: "gpt", effort: "high", lines: [], busy: false, partial: null, waiting: [] };
let reads = 0, readFails = true, currentFails = false;
const writes = [];
const restoring = new ChatPanel({
  models: async () => [], pick: async () => ({}),
  current: async () => { if (currentFails) throw new Error("Conversation unavailable."); return conversation; },
  draft: async () => { reads++; if (readFails) throw new Error("Draft unavailable."); return kept; },
  saveDraft: async draft => { writes.push(draft); },
}, () => {}, { window: "nest" });
await tick();
await assert.rejects(restoring.flushDraft(), /Draft unavailable/);
assert.deepEqual(writes, [], "A failed initial read must never save an empty draft");
assert.equal(reads, 2, "A save retries the failed startup read once, with no automatic loop");
readFails = false; currentFails = true;
await assert.rejects(restoring.flushDraft(), /Conversation unavailable/);
assert.deepEqual(writes, [], "A failed conversation read must also block saving");
currentFails = false;
await restoring.flushDraft();
assert.deepEqual(writes, [kept], "Retry restores text, files and bound source before saving");
await restoring.flushDraft();
assert.equal(reads, 4, "Successful restoration is reused");
console.log("check-chat-models: initial draft/conversation failure, no empty write, explicit retry, source/attachments retained");
