// Messages written while a turn runs wait in a queue: they go when it ends well, and come back to the draft after Stop or an error.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";

class Element {
  children = [];
  listeners = {};
  attributes = {};
  dataset = {};
  className = "";
  style = { setProperty(name, value) { this[name] = value; } };
  value = "";
  classList = {
    contains: name => this.className.split(" ").includes(name),
    add: name => { if (!this.classList.contains(name)) this.className += ` ${name}`; },
    remove: name => { this.className = this.className.split(" ").filter(value => value !== name).join(" "); },
    toggle: (name, on = !this.classList.contains(name)) => this.classList[on ? "add" : "remove"](name),
  };
  constructor(tag) { this.tag = tag; this.tagName = tag.toUpperCase(); }
  set textContent(text) { this.text = text; this.children = []; }
  get textContent() { return (this.text ?? "") + this.children.map(node => node.textContent).join(""); }
  get childNodes() { return this.children; }
  contains(node) { return nodes(this).includes(node); }
  matches(selector) { return selector.split(",").some(value => value.trim().startsWith(".") ? this.classList.contains(value.trim().slice(1)) : this.tag === value.trim()); }
  querySelectorAll(selector) { return nodes(this).slice(1).filter(node => node.matches(selector)); }
  click() { if (!this.disabled) this.listeners.click?.({ target: this }); }
  setSelectionRange(start, end) { this.selectionStart = start; this.selectionEnd = end; }
  querySelector(selector) { return this.querySelectorAll(selector)[0] ?? null; }
  append(...nodes) { for (const node of nodes) node.parentElement = this; this.children.push(...nodes); }
  prepend(...nodes) { for (const node of nodes) node.parentElement = this; this.children.unshift(...nodes); }
  replaceChildren(...nodes) { this.text = ""; this.children = []; this.append(...nodes); }
  replaceWith(node) { this.parentElement.children.splice(this.parentElement.children.indexOf(this), 1, node); node.parentElement = this.parentElement; }
  addEventListener(event, fn) { this.listeners[event] = fn; }
  setAttribute(name, value) { this.attributes[name] = value; }
  getAttribute(name) { return this.attributes[name]; }
  toggleAttribute() {}
  focus() { document.activeElement = this; }
  blur() { if (document.activeElement === this) { document.activeElement = null; this.listeners.blur?.({ target: this }); } }
  scrollTo() {}
  getBoundingClientRect() { return { height: 200 }; }
  getContext() { return { clearRect() {}, fillRect() {} }; }
  toDataURL() { return "data:image/png;base64,"; }
}
const documentEvents = {};
globalThis.document = { body: new Element("body"), dispatchEvent() {}, addEventListener: (event, fn) => (documentEvents[event] ??= []).push(fn), createElement: tag => new Element(tag), createElementNS: (_ns, tag) => new Element(tag),
  createDocumentFragment: () => new Element("fragment"),
  createTextNode: text => { const node = new Element("text"); node.textContent = text; return node; } };
globalThis.window = globalThis;
globalThis.addEventListener = () => {};
globalThis.setTimeout = globalThis.setInterval = () => 0;
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
const sent = [];
const panel = new ChatPanel({
  models: async () => [], pick: async () => ({}), saveDraft: async () => {}, keyboard() {},
  send: async (text) => { sent.push(text); },
}, () => {});
await tick();
panel.toggle(true);
const input = nodes(panel.element).find(node => node.tag === "textarea");
const queue = () => nodes(panel.element).find(node => node.className === "chat-queue");
const type = async text => { input.value = text; input.listeners.input?.({}); input.listeners.keydown({ key: "Enter", preventDefault() {}, stopPropagation() {} }); await tick(); await tick(); };

await type("First question");
assert.deepEqual(sent, ["First question"], "an idle chat sends at once");
await type("Second, written while Zeca works");
assert.deepEqual(sent, ["First question"], "while a turn runs, Enter queues instead of sending");
assert.equal(input.value, "", "the queued words leave the draft");
assert.equal(queue().hidden, false);
assert.ok(queue().textContent.includes("Second, written while Zeca works"), "the queue shows what waits");
await type("Third");
assert.equal(nodes(queue()).filter(n => n.className === "queue-item").length, 2, "each queued message is its own row");
// Removing one keeps the others.
nodes(queue()).filter(n => n.className === "queue-remove")[1].click();
assert.equal(nodes(queue()).filter(n => n.className === "queue-item").length, 1);

panel.receive({ kind: "done" });
await tick(); await tick();
assert.deepEqual(sent, ["First question", "Second, written while Zeca works"], "a turn that ended well sends the next one");
assert.equal(queue().hidden, true, "nothing waits any more");

await type("Fourth, queued");
panel.receive({ kind: "stopped" });
await tick(); await tick();
assert.deepEqual(sent.length, 2, "Stop never sends the queue on its own");
assert.equal(input.value, "Fourth, queued", "after Stop the queued words come back into the draft");
assert.equal(queue().hidden, true);
console.log("check-chat-queue: messages queue while a turn runs, go in order after it ends well, can be removed, and return to the draft after Stop");
