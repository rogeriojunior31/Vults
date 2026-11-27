// The chat's @ menu offers the conversation folder's files and folders, and picking one puts its path in the draft as plain text.
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
const asked = [];
const panel = new ChatPanel({
  models: async () => [], pick: async () => ({}), saveDraft: async () => {}, keyboard() {},
  paths: async (folder, query) => { asked.push([folder, query]); return [{ path: "src/auth", dir: true }, { path: "src/auth token.rs", dir: false }]; },
}, () => {});
await tick();
panel.toggle(true);
panel.folder = "/work/api";
const input = nodes(panel.element).find(node => node.tag === "textarea");
const menu = () => nodes(panel.element).find(node => node.className.split(" ").includes("mention-menu"));
const typeAt = async text => { input.value = text; input.selectionStart = input.selectionEnd = text.length; input.listeners.input({}); await tick(); await tick(); };

await typeAt("Look at @aut");
assert.deepEqual(asked.at(-1), ["/work/api", "aut"], "the folder is asked for what follows @");
const rows = nodes(menu()).filter(node => node.tag === "button");
assert.ok(rows[0].textContent.includes("Folder: src/auth"), "a folder row");
assert.ok(rows[1].textContent.includes("File: src/auth token.rs"), "a file row");
rows[1].listeners.click();
assert.equal(input.value, 'Look at @"src/auth token.rs" ', "a path with a space goes in quoted, as plain text");
assert.equal(panel.mentions.length, 0, "a path is not a flock reference");

// No folder (the chat's own empty one): nothing is asked.
panel.folder = null;
const before = asked.length;
await typeAt("@src");
assert.equal(asked.length, before);
// A folder dropped on the chat goes into the draft by its path, never copied.
input.value = "Review";
panel.attach([], [], ["/work/api/src", "C:\Users\me\My Repo"]);
assert.equal(input.value, 'Review @/work/api/src @"C:\Users\me\My Repo" ');
assert.equal(panel.files.length, 0, "a folder is not an attachment");
console.log("check-chat-paths: @ lists the conversation folder's files and folders and puts the chosen path in the draft; a dropped folder goes in by its path");
