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
const asked = [], pastes = [];
const panel = new ChatPanel({
  models: async () => [], pick: async () => ({}), saveDraft: async () => {}, keyboard() {},
  paste: async (name, bytes) => { pastes.push([name, bytes.length]); return `/inbox/${name}`; },
  paths: async (folder, query) => { asked.push([folder, query]); return [{ path: "src/auth", dir: true }, { path: "src/auth token.rs", dir: false }]; },
  checkFolder: async path => { if (path === "/nowhere") throw "That folder doesn't exist."; return path.replace(/\/$/, ""); },
  recentFolders: async () => ["/work/site", "/work/api"],
  pickFolder: async () => "/work/picked",
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
// One folder dropped on a chat that hasn't started is where it works.
panel.attach([], [], ["/work/dropped"]);
assert.equal(panel.folder, "/work/dropped", "a single folder dropped first sets the place");
assert.equal(input.value, "@src", "and goes nowhere in the draft");
// The place menu: recent folders, a typed path checked, the chooser.
const place = panel.place;
place.listeners.click({});
await tick(); await tick();
const placeMenu = () => nodes(panel.element).filter(n => n.tag === "button" && n.className.includes("menu-item"));
assert.ok(placeMenu().some(b => b.textContent.includes("site")), `a recent folder is offered (${placeMenu().map(b => b.textContent)})`);
const field = nodes(panel.element).find(n => n.className === "place-input");
field.value = "/nowhere"; field.listeners.keydown({ key: "Enter", preventDefault() {}, stopPropagation() {} });
await tick(); await tick();
assert.ok(panel.element.textContent.includes("That folder doesn't exist."), "a wrong path says so");
field.value = "/work/typed/"; field.listeners.keydown({ key: "Enter", preventDefault() {}, stopPropagation() {} });
await tick(); await tick();
assert.equal(panel.folder, "/work/typed", "a typed path, checked, is the place");
place.listeners.click({}); await tick();
placeMenu().find(b => b.textContent.includes("Choose a folder")).listeners.click({});
await tick(); await tick();
assert.equal(panel.folder, "/work/picked", "the chooser's folder is the place");
// A project's Chat here: the chat that hasn't started works there.
assert.equal(panel.workIn("/work/roost"), true);
assert.equal(panel.folder, "/work/roost");
panel.folder = null;
input.value = "";
// Once a conversation has words, a folder dropped on the chat goes into the draft by its path, never copied.
panel.messages.push({ who: "you", text: "hi", files: [] });
input.value = "Review";
panel.attach([], [], ["/work/api/src", "C:\Users\me\My Repo"]);
assert.equal(input.value, 'Review @/work/api/src @"C:\Users\me\My Repo" ');
assert.equal(panel.files.length, 0, "a folder is not an attachment");
// "/" at the start lists the chat's commands; picking one puts it in the draft.
await typeAt("/re");
const commands = nodes(menu()).filter(node => node.tag === "button");
assert.deepEqual(commands.map(b => b.children[0].textContent), ["/remember"], "only the commands that match");
commands[0].listeners.click();
assert.equal(input.value, "/remember ", "the command waits for its words");
await typeAt("/");
assert.equal(nodes(menu()).filter(node => node.tag === "button").length, 4, "all four commands");
await typeAt("Say /re");
assert.ok(!menu().className.split(" ").includes("on") || !nodes(menu()).some(n => n.tag === "button" && n.textContent.startsWith("/")), "a / mid-sentence is just text");
// A long paste becomes an attached file; a short one stays text in the draft.
let prevented = false;
input.listeners.paste({ clipboardData: { files: [], getData: () => "x".repeat(40 * 1024) }, preventDefault() { prevented = true; } });
await tick(); await tick();
assert.ok(prevented, "the long text does not land in the draft");
assert.deepEqual(pastes.at(-1), ["pasted.txt", 40 * 1024]);
assert.ok(panel.files.includes("/inbox/pasted.txt"), "it is attached");
prevented = false;
input.listeners.paste({ clipboardData: { files: [], getData: () => "short" }, preventDefault() { prevented = true; } });
assert.equal(prevented, false, "a short paste is left to the draft");
// A pasted image is attached too.
input.listeners.paste({ clipboardData: { files: [{ type: "image/png", name: "shot.png", arrayBuffer: async () => new ArrayBuffer(3) }], getData: () => "" }, preventDefault() {} });
await tick(); await tick(); await tick();
assert.deepEqual(pastes.at(-1), ["shot.png", 3]);
console.log("check-chat-paths: @ lists the conversation folder's files and folders and puts the chosen path in the draft; a folder dropped first sets the place, later it goes in by its path; the place menu offers recent folders, a checked typed path and the chooser; / lists the chat's commands; long pastes and images attach");
