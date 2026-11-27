// A permission card ignores clicks in its first moments (rule 2), with no browser server.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";

class Element {
  children = []; listeners = {}; attributes = {}; dataset = {}; style = { setProperty() {} };
  className = ""; title = "";
  classList = { add: cls => { this.className = `${this.className} ${cls}`.trim(); }, remove() {}, toggle() {}, contains: cls => this.className.split(" ").includes(cls) };
  constructor(tag) { this.tag = tag; }
  set textContent(text) { this.text = text; this.children = []; }
  get textContent() { return (this.text ?? "") + this.children.map(node => node.textContent).join(""); }
  append(...nodes) { this.children.push(...nodes); }
  replaceChildren(...nodes) { this.text = ""; this.children = nodes; }
  addEventListener(event, fn) { this.listeners[event] = fn; }
  setAttribute(name, value) { this.attributes[name] = value; }
}
const nodes = element => [element, ...element.children.flatMap(nodes)];
globalThis.document = { createElement: tag => new Element(tag), createElementNS: (_ns, tag) => new Element(tag), createTextNode: text => { const node = new Element("text"); node.textContent = text; return node; }, body: new Element("body"), addEventListener() {} };
globalThis.window = { addEventListener() {}, matchMedia: () => ({ matches: true, addEventListener() {} }), setTimeout: () => 0, clearTimeout() {} };
globalThis.matchMedia = window.matchMedia;
let now = 1000;
Object.defineProperty(globalThis, "performance", { configurable: true, value: { now: () => now } });

const root = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({ configFile: false, root, logLevel: "silent", build: {
  write: false, minify: false, lib: { entry: `${root}src/island/views.ts`, formats: ["es"] },
} });
const chunk = bundles[0].output.find(part => part.type === "chunk");
const { focusCard } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);

const decided = [];
const actions = { keys: {}, decide: (request, d) => decided.push([request, d]), decideAlways: request => decided.push([request, "always"]),
  answer() {}, release() {}, skip() {}, review() {}, keyboard() {}, relayout() {}, jump() {}, dismiss() {}, hush() {}, openChat() {} };
const session = { agent: "claude", id: "s1", status: "approval", cwd: "/work/api", project: "api", attention: "needs-you" };
const approval = { request: "r1", agent: "claude", session: "s1", tool: "Bash", target: "rm -rf build", queue: 1, questions: [], cwd: "/work/api" };
const ticker = { steps: [] };
const allow = card => nodes(card).find(node => node.tag === "button" && node.textContent.includes("Allow") && !node.textContent.includes("Always"));

let card = focusCard(session, approval, ticker, new Element("div"), actions, false);
now += 100;
allow(card).listeners.click();
assert.deepEqual(decided, [], "a click 100 ms after the card appeared is not an answer");
// A redraw is not a new card: the window runs from the first time this request was shown.
now += 400;
card = focusCard(session, approval, ticker, new Element("div"), actions, false);
now += 150;
allow(card).listeners.click();
assert.deepEqual(decided, [["r1", "allow"]], "past 600 ms the click answers");
allow(card).listeners.click();
assert.equal(decided.length, 1, "one card, one answer");
// The next request has its own window.
const next = { ...approval, request: "r2" };
card = focusCard(session, next, ticker, new Element("div"), actions, false);
allow(card).listeners.click();
assert.equal(decided.length, 1);
now += 601;
allow(card).listeners.click();
assert.deepEqual(decided.at(-1), ["r2", "allow"]);
console.log("check-card-arming: clicks in a card's first 600 ms are ignored, across redraws, per request");
