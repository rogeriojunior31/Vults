// The agent's questions get option buttons; typing while a card waits answers it with those words.
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
const replies = [], decisions = [], sent = [];
const panel = new ChatPanel({
  models: async () => [], pick: async () => ({}), saveDraft: async () => {}, keyboard() {},
  send: async text => { sent.push(text); },
  decide: async (id, answer) => { decisions.push([id, answer]); },
  reply: async (id, text) => { replies.push([id, text]); },
}, () => {});
await tick();
panel.toggle(true);
const input = nodes(panel.element).find(node => node.tag === "textarea");
const buttons = () => nodes(panel.element).filter(node => node.tag === "button");

panel.receive({ kind: "permission", id: "q1", tool: "AskUserQuestion", target: "AskUserQuestion · Which database?", description: null, full: null,
  added: 0, removed: 0, always: false, outside: null, questions: [{ question: "Which database?", options: ["Postgres", "SQLite"] }] });
await tick();
assert.ok(panel.element.textContent.includes("Which database?"), "the question shows");
const postgres = buttons().find(b => b.textContent === "Postgres");
assert.ok(postgres, "each option is a button");
assert.ok(!buttons().some(b => b.textContent.startsWith("Allow")), "a question has no Allow");
postgres.click();
await tick();
assert.deepEqual(replies, [["q1", "I chose: Postgres"]], "the pick goes back as the user's words");
assert.deepEqual(decisions, []);

// A permission card waits: typing and Enter answers it with those words, instead of sending a message.
panel.receive({ kind: "permission", id: "p1", tool: "Bash", target: "Bash · rm -rf build", description: null, full: null,
  added: 0, removed: 0, always: false, outside: null });
await tick();
input.value = "Don't delete build, clean only the cache";
input.listeners.input({});
input.listeners.keydown({ key: "Enter", preventDefault() {}, stopPropagation() {} });
await tick(); await tick();
assert.deepEqual(replies.at(-1), ["p1", "Don't delete build, clean only the cache"]);
assert.deepEqual(sent, [], "it is an answer, not a new message");
assert.equal(input.value, "");
// A tool's output shows under it in the turn's activity.
input.value = "Run the tests"; input.listeners.input({});
input.listeners.keydown({ key: "Enter", preventDefault() {}, stopPropagation() {} });
await tick(); await tick();
panel.receive({ kind: "activity", event: { type: "start", id: "t9", tool: "Bash", detail: "cargo test" } });
panel.receive({ kind: "activity", event: { type: "output", id: "t9", text: "test result: ok. 42 passed" } });
panel.receive({ kind: "activity", event: { type: "end", id: "t9", ok: true } });
panel.paint(); // the harness runs no timers: the throttled repaint, at once
await tick();
const outputs = nodes(panel.element).filter(node => node.className === "activity-output");
assert.ok(outputs.some(o => !o.hidden && o.textContent === "test result: ok. 42 passed"), "the output shows under the tool");
console.log("check-chat-asks: questions get option buttons and go back as the user's words; typing while a card waits answers it; a tool's output shows under it");
