// Check the island's update question and notice (FF4) without a browser server: who comes first,
// what each button asks the app for, and that nothing else is offered.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";

class Element {
  children = [];
  listeners = {};
  attributes = {};
  className = "";
  constructor(tag) { this.tag = tag; }
  set textContent(text) { this.text = text; this.children = []; }
  get textContent() { return (this.text ?? "") + this.children.map(node => node.textContent).join(""); }
  append(...nodes) { this.children.push(...nodes); }
  replaceChildren(...nodes) { this.children = []; this.append(...nodes); }
  addEventListener(event, fn) { this.listeners[event] = fn; }
  setAttribute(name, value) { this.attributes[name] = value; }
  getAttribute(name) { return this.attributes[name]; }
  click() { this.listeners.click?.({ target: this }); }
  get classList() { return { add: c => { this.className = `${this.className} ${c}`.trim(); }, contains: c => this.className.split(" ").includes(c) }; }
}
globalThis.document = {
  createElement: tag => new Element(tag),
  createElementNS: (_ns, tag) => new Element(tag),
  createTextNode: text => { const node = new Element("text"); node.textContent = text; return node; },
};
globalThis.window = globalThis;
globalThis.localStorage = { getItem: () => null, setItem() {} };
Object.defineProperty(globalThis, "navigator", { value: { language: "en-US", languages: ["en-US"] }, configurable: true });

const root = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({ configFile: false, root, logLevel: "silent", build: {
  write: false, minify: false, lib: { entry: `${root}src/island/update.ts`, formats: ["es"] },
} });
const chunk = bundles[0].output.find(part => part.type === "chunk");
const { updateBox, updateShown, firstLine } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
const all = node => [node, ...node.children.flatMap(all)];
const buttons = box => all(box).filter(node => node.tag === "button");

const status = (over = {}) => ({ available: true, consent: { state: "unasked" }, ask: true, current: "0.1.9", checkedAtMs: null, checking: false, failed: false, notice: null, ...over });
const view = (over = {}) => ({ sessions: [], approval: null, alerts: [], ...over });
const session = over => ({ id: "s", agent: "claude", project: "p", status: "working", attention: "quiet", ...over });

// Who comes first.
assert.equal(updateShown(status(), view(), false, false), true, "nothing waits: the question shows");
assert.equal(updateShown(null, view(), false, false), false);
assert.equal(updateShown(status({ available: false }), view(), false, false), false, "distro builds never ask");
assert.equal(updateShown(status({ ask: false }), view(), false, false), false, "answered: nothing to show");
assert.equal(updateShown(status(), view({ approval: { request: "r" } }), false, false), false, "an approval comes first");
assert.equal(updateShown(status(), view({ sessions: [session({ attention: "needs-you" })] }), false, false), false);
assert.equal(updateShown(status(), view({ sessions: [session({ card: { request: "r" } })] }), false, false), false);
assert.equal(updateShown(status(), view({ sessions: [session({ waiting: true })] }), false, false), false);
assert.equal(updateShown(status(), view({ locked: true }), false, false), false, "locked: nothing on screen");
assert.equal(updateShown(status(), view(), true, false), false, "paused: nothing new");
assert.equal(updateShown(status(), view(), false, true), false, "a chat card comes first");
assert.equal(updateShown(status(), view({ sessions: [session({ status: "finished", attention: "done" })] }), false, false), true);

// The question: two answers, and reported as shown.
const calls = [];
const actions = {
  updateChecks: on => calls.push(["checks", on]),
  updateQuestionShown: () => calls.push(["shown"]),
  updateLater: () => calls.push(["later"]),
  openUpdatePage: () => calls.push(["open"]),
};
let shown = 0;
const ask = updateBox(status(), actions, () => { shown += 1; });
assert.equal(shown, 1, "building the question reports it shown");
assert.ok(ask.className.includes("update-ask"));
assert.equal(ask.getAttribute("role"), "group");
assert.equal(ask.getAttribute("aria-label"), "May Vults look at its releases on GitHub?");
assert.match(ask.textContent, /Nothing about you is sent/);
const [yes, no] = buttons(ask);
assert.deepEqual(buttons(ask).map(b => b.textContent), ["Yes, once a day", "No"]);
yes.click();
no.click();
assert.deepEqual(calls, [["checks", true], ["checks", false]]);
assert.equal(buttons(ask).filter(b => b.className.includes("primary")).length, 1, "one filled action");

// The notice: version, its first line, release page and Later; the question is not reported.
calls.length = 0;
const notes = "## What's new\n\n- **Faster** island\n- More";
const notice = updateBox(status({ ask: false, notice: { version: "0.2.0", name: "Vults v0.2.0", notes, url: "https://github.com/rogeriojunior31/Vults/releases/tag/v0.2.0" } }), actions, () => { shown += 1; });
assert.equal(shown, 1, "a notice is not the question");
assert.equal(notice.getAttribute("aria-label"), "Vults 0.2.0 is available");
assert.match(notice.textContent, /Vults 0\.2\.0 is available/);
assert.match(notice.textContent, /What's new/);
assert.ok(!notice.textContent.includes("https://"), "the link is the app's to open, never shown or passed");
assert.deepEqual(buttons(notice).map(b => b.textContent), ["Open release page", "Later"]);
buttons(notice).forEach(b => b.click());
assert.deepEqual(calls, [["open"], ["later"]]);
assert.equal(buttons(notice).find(b => b.textContent === "Later").title, "Hide this version for a day");

assert.equal(firstLine(""), "");
assert.equal(firstLine("\r\n  \r\n> **Fixes** for `Windows`\r\n"), "Fixes for Windows");
assert.equal(firstLine("- **Faster** island"), "Faster island");
assert.equal(firstLine("1. First"), "First");
assert.equal(firstLine("*emphasis* stays readable"), "*emphasis* stays readable");
assert.equal(firstLine("### 0.2.0\n"), "0.2.0");

console.log("check-update-notice: ok");
