// The thread: edits show their diff, the status line says what runs, ↑/↓ walk earlier messages, Ctrl+F finds, nodes are kept.
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
const press = (key, extra = {}) => input.listeners.keydown({ key, preventDefault() {}, stopPropagation() {}, ...extra });
const send = async text => { input.value = text; input.listeners.input({}); press("Enter"); await tick(); await tick(); };
const byClass = name => nodes(panel.element).filter(node => node.classList.contains(name));

// A turn that edits a file: its row names the file, counts the lines, and shows the change.
await send("Rename the field");
panel.receive({ kind: "activity", event: { type: "start", id: "e1", tool: "Bash", detail: "cargo test --workspace" } });
panel.paint();
assert.ok(panel.element.textContent.includes("Running cargo test --workspace…"), "the status line says what runs now");
// The status line is one node for the turn: a repaint keeps the log's children (and a selection).
const kids = () => nodes(panel.element).find(n => n.classList.contains("log")).children.slice();
const before1 = kids(); panel.paint(); const after1 = kids();
assert.ok(before1.length === after1.length && before1.every((n, i) => n === after1[i]), "a repaint mid-turn keeps every node");
// An end for a tool no stretch holds is dropped, not a new empty stretch.
const stretches1 = nodes(panel.element).filter(n => n.classList.contains("chat-activity")).length;
panel.receive({ kind: "activity", event: { type: "end", id: "nobody", ok: true } }); panel.paint();
assert.equal(nodes(panel.element).filter(n => n.classList.contains("chat-activity")).length, stretches1);
panel.receive({ kind: "activity", event: { type: "end", id: "e1", ok: true } });
panel.receive({ kind: "activity", event: { type: "start", id: "e2", tool: "Edit", detail: "/w/src/lib.rs" } });
panel.receive({ kind: "activity", event: { type: "diff", id: "e2", diff: "@@ /w/src/lib.rs\n keep\n-old name\n+new name\n+one more" } });
panel.paint();
assert.ok(panel.element.textContent.includes("Editing lib.rs…"), "an edit names its file");
const diff = byClass("activity-diff").find(d => !d.hidden);
assert.ok(diff && !diff.hidden, "the change shows");
assert.equal(byClass("d-add").filter(n => n.tag === "span" && n.classList.contains("d-line")).length, 2);
assert.equal(byClass("d-del").filter(n => n.classList.contains("d-line")).length, 1);
assert.ok(byClass("tool-counts").some(n => n.textContent === "+2−1"), "the row counts what changed");
panel.receive({ kind: "activity", event: { type: "end", id: "e2", ok: true } });
panel.receive({ kind: "text", text: "Done: ~~old~~ new, see https://example.com/x.\n\n- [x] renamed\n- [ ] docs" });
panel.receive({ kind: "done" });
await tick();
assert.ok(byClass("activity-took").length === 1, "the turn's activity says how long it ran");
assert.ok(nodes(panel.element).some(n => n.tag === "s" && n.textContent === "old"), "~~ strikes through");
assert.ok(byClass("link").some(n => n.textContent === "https://example.com/x"), "a bare address shows as one, without the period");
assert.ok(byClass("task").length === 2 && byClass("done").length >= 1, "task lists get their boxes");

// A repaint keeps every message's node: nothing is drawn again.
const before = byClass("msg").filter(n => n.classList.contains("zeca"))[0];
panel.paint();
assert.equal(byClass("msg").filter(n => n.classList.contains("zeca"))[0], before, "the reply's node is kept");

// ↑ walks back through what was sent, ↓ comes forward to an empty composer.
await send("Second question");
panel.receive({ kind: "text", text: "ok" }); panel.receive({ kind: "done" }); await tick();
press("ArrowUp"); assert.equal(input.value, "Second question");
press("ArrowUp"); assert.equal(input.value, "Rename the field");
press("ArrowUp"); assert.equal(input.value, "Rename the field", "the oldest stays");
press("ArrowDown"); assert.equal(input.value, "Second question");
press("ArrowDown"); assert.equal(input.value, "", "past the newest, an empty composer");
// A recalled message of several lines: ↑ moves the caret until it is on the first line.
await send("line one\nline two");
panel.receive({ kind: "text", text: "ok" }); panel.receive({ kind: "done" }); await tick();
press("ArrowUp"); assert.equal(input.value, "line one\nline two");
input.selectionStart = input.selectionEnd = input.value.length;
let moved = false;
input.listeners.keydown({ key: "ArrowUp", preventDefault() { moved = true; }, stopPropagation() {} });
assert.ok(!moved && input.value === "line one\nline two", "on the last line, ↑ moves the caret");
input.selectionStart = input.selectionEnd = 0;
press("ArrowUp"); assert.equal(input.value, "Second question", "on the first line, ↑ goes back");
press("ArrowDown"); press("ArrowDown"); assert.equal(input.value, "");

// The last message offers Edit and send again; it puts the words back.
const edit = nodes(panel.element).find(n => n.tag === "button" && n.textContent === "Edit and send again");
assert.ok(edit, "the last message can be edited");
edit.click();
assert.equal(input.value, "line one\nline two");
input.value = ""; input.listeners.input({});

// Ctrl+F finds in the conversation and steps through the matches.
panel.element.listeners.keydown({ key: "f", ctrlKey: true, preventDefault() {}, stopPropagation() {} });
const find = nodes(panel.element).find(n => n.classList.contains("find-input"));
assert.ok(byClass("chat-find")[0].classList.contains("on"), "the find bar opens");
find.value = "question"; find.listeners.input({});
assert.equal(byClass("find-count")[0].textContent, "1 of 1");
find.value = "name"; find.listeners.input({});
const count = byClass("find-count")[0].textContent;
assert.ok(/^1 of [2-9]$/.test(count), `matches counted (${count})`);
find.listeners.keydown({ key: "Enter", shiftKey: false, preventDefault() {}, stopPropagation() {} });
assert.ok(byClass("find-count")[0].textContent.startsWith("2 of"), "Enter goes to the next match");
assert.equal(byClass("found-current").length, 1);
find.value = "zzz"; find.listeners.input({});
assert.equal(byClass("find-count")[0].textContent, "No matches");
find.listeners.keydown({ key: "Escape", preventDefault() {}, stopPropagation() {} });
assert.ok(!byClass("chat-find")[0].classList.contains("on"), "Escape closes it");
// Activity after words is a stretch of its own, below them: the turn reads in the order it ran.
await send("Look and fix");
panel.receive({ kind: "activity", event: { type: "start", id: "s1", tool: "Read", detail: "/w/a.rs" } });
panel.receive({ kind: "text", text: "Found the bug." });
panel.receive({ kind: "activity", event: { type: "start", id: "s2", tool: "Edit", detail: "/w/a.rs" } });
panel.receive({ kind: "text", text: "Fixed it." });
panel.receive({ kind: "done" });
await tick();
const order = byClass("chat-activity").length;
const log = nodes(panel.element).find(n => n.className === "log" || n.classList.contains("log"));
const tail = log.children.slice(-4).map(n => n.classList.contains("chat-activity") ? "activity" : n.textContent.includes("Found") ? "found" : n.textContent.includes("Fixed") ? "fixed" : "?");
assert.deepEqual(tail, ["activity", "found", "activity", "fixed"], `in order (${tail})`);
assert.ok(order >= 3);

// The side pane's Changes: each file the agent changed, with its counts and diffs.
const changes = panel.changes();
const lib = changes.find(c => c.path.endsWith("lib.rs"));
assert.ok(lib && lib.added === 2 && lib.removed === 1 && lib.diffs.length === 1, `changes (${JSON.stringify(changes)})`);
input.value = "look at"; input.setSelectionRange(7, 7);
panel.reference("src/my file.rs");
assert.equal(input.value, 'look at @"src/my file.rs" ', "a path goes in as @ puts it");
input.value = ""; input.listeners.input({});

// Each message says when it was said.
assert.ok(byClass("msg-time").length >= 2, "messages carry their time");

// Code is colored: keywords, strings, comments; plain past its limit.
panel.receive({ kind: "text", text: "\n\n```rust\nfn main() { let s = \"hi\"; } // done\n```" });
panel.paint();
assert.ok(byClass("tok-keyword").some(n => n.textContent === "fn"), "a keyword is colored");
assert.ok(byClass("tok-string").some(n => n.textContent === '"hi"'), "a string is colored");
assert.ok(byClass("tok-comment").some(n => n.textContent === "// done"), "a comment is colored");

// The agent's plan: a checklist in the activity, its step on the status line, its progress on the closed line.
await send("Plan it");
panel.receive({ kind: "activity", event: { type: "plan", steps: [{ text: "Read the code", status: "completed" }, { text: "Fix the bug", status: "in_progress" }, { text: "Run the tests", status: "pending" }] } });
panel.paint();
assert.equal(byClass("plan-step").length >= 3, true);
assert.ok(panel.element.textContent.includes("Fix the bug…"), "the status line says the step it is on");
assert.ok(panel.element.textContent.includes("Plan 1 of 3"));
panel.receive({ kind: "done" }); await tick();

// The context meter: hidden until the engine says, a share once it gives the window too.
const meter = byClass("context-meter")[0];
assert.ok(meter.hidden, "no meter before the engine says");
panel.receive({ kind: "context", used: 42000, window: null });
assert.ok(!meter.hidden && meter.textContent === "42k", "tokens alone");
panel.receive({ kind: "context", used: null, window: 200000 });
assert.equal(meter.textContent, "21%");
panel.receive({ kind: "context", used: 170000, window: null });
assert.ok(meter.classList.contains("full"), "past 80% it warns");
// t3code's Copy thread: the open conversation as Markdown, from the Conversations menu.
let copied = null;
Object.defineProperty(globalThis.navigator, "clipboard", { value: { writeText: async text => { copied = text; } }, configurable: true });
panel.messages = [{ who: "you", text: "Fix the build", files: [] }, { who: "note", text: "Switched engine" }, { who: "zeca", text: "Done: **cargo** passes." }];
panel.saved = [{ id: 1, title: "Build fix", label: "Claude", current: true }];
panel.historyOpen = true; panel.paintHistory();
const copyAll = nodes(panel.element).find(n => n.classList.contains("saved-copy"));
assert.ok(copyAll, "the Conversations menu offers Copy as Markdown");
copyAll.click(); await tick(); await tick();
assert.ok(copied.startsWith("# Build fix\n\n**You**\n\nFix the build\n\n**"), "title, then who said each turn");
assert.ok(copied.includes("Done: **cargo** passes.") && !copied.includes("Switched engine"), "replies as written; notes stay out");
panel.messages = []; panel.paintHistory();
assert.ok(!nodes(panel.element).some(n => n.classList.contains("saved-copy")), "nothing to copy, no button");
panel.historyOpen = false;
console.log("check-chat-thread: the context meter follows the engine; edits show their diff and counts, the status line says what runs, turns say how long they ran, nodes are kept, ↑/↓ walk earlier messages, Edit puts the last back, Ctrl+F finds and steps");
