// Exercise the production By project renderer without a browser server.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";

class Element {
  children = []; listeners = {}; className = ""; value = ""; style = {};
  constructor(tag) { this.tag = tag; }
  set textContent(text) { this.text = text; this.children = []; }
  get textContent() { return (this.text ?? "") + this.children.map(node => node.textContent).join(""); }
  append(...nodes) { this.children.push(...nodes); }
  replaceChildren(...nodes) { this.text = ""; this.children = nodes; }
  addEventListener(event, fn) { this.listeners[event] = fn; }
  setAttribute() {}
  focus() { document.activeElement = this; }
}
globalThis.document = { createElementNS: (_ns, tag) => new Element(tag), createElement: tag => new Element(tag) };
const root = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({ configFile: false, root, logLevel: "silent", build: {
  write: false, minify: false, lib: { entry: `${root}src/surfaces/nest/roosts.ts`, formats: ["es"] },
} });
const chunk = bundles[0].output.find(part => part.type === "chunk");
const { createRoosts } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
const nodes = element => [element, ...element.children.flatMap(nodes)];
const input = part => nodes(part.element).find(node => node.tag === "input");
const names = part => nodes(part.element).filter(node => node.className === "roost-name").map(node => node.textContent);
const tick = () => new Promise(resolve => setImmediate(resolve));
let kept = "feature/login", loads = 0, weeks = 0, fail = false, hold = false, release;
const writes = [], actions = [];
let list = ["Alpha", "Beta"].map((name, i) => ({ name, key: name.toLowerCase(), checks: null, checksItem: null, cost: null,
  trees: [{ path: `/work/${name.toLowerCase()}`, branch: i ? "main" : "feature/login" }],
  sessions: [{ agent: i ? "Codex" : "Claude", kind: i ? "codex" : "claude", id: name, status: "working", step: "Editing" }],
  prs: [], memory: [], agents: "none" }));
const backend = {
  load: async () => { loads++; return list; }, week: async () => { weeks++; return null; },
  query: async value => {
    if (value === undefined) return kept;
    if (hold) { hold = false; await new Promise(resolve => { release = resolve; }); }
    if (fail) throw new Error("Navigation unavailable.");
    kept = value; writes.push(value); return kept;
  },
  ...Object.fromEntries(["preview", "apply", "open", "focus", "preferences"].map(name => [name, () => actions.push(name)])),
};
const mount = async () => { const part = createRoosts(backend); part.refresh(); await tick(); return part; };
const first = await mount();
assert.equal(input(first).value, "feature/login");
assert.deepEqual(names(first), ["Alpha"]);
input(first).focus();
first.refresh(); await tick();
assert.equal(document.activeElement, input(first));
assert.equal(input(first).value, "feature/login");
const before = [loads, weeks];
hold = true;
input(first).value = "claude"; input(first).listeners.input();
input(first).value = "codex"; input(first).listeners.input();
let flushed = false;
const pending = first.flush().then(() => { flushed = true; });
await tick();
assert.equal(flushed, false, "Closing must wait for pending project-query saves");
release(); await pending;
assert.deepEqual(writes, ["claude", "codex"], "Rapid typing must save in order");
assert.deepEqual(names(first), ["Beta"]);
assert.deepEqual([loads, weeks], before, "Filtering saves navigation without reloading project/history data");
const reopened = await mount();
assert.equal(input(reopened).value, "codex");
assert.deepEqual(names(reopened), ["Beta"]);
input(reopened).value = "missing"; input(reopened).listeners.input();
assert.ok(reopened.element.textContent.includes("No project matches."));
nodes(reopened.element).find(node => node.tag === "button" && node.textContent === "Clear filters").listeners.click();
await reopened.flush();
assert.equal(kept, "");
assert.equal(document.activeElement, input(reopened));
const cleared = await mount();
assert.equal(input(cleared).value, "");
assert.deepEqual(names(cleared), ["Alpha", "Beta"]);
let restore;
const late = createRoosts({ ...backend, query: value => value === undefined ? new Promise(resolve => { restore = resolve; }) : backend.query(value) });
late.refresh();
input(late).value = "Beta"; input(late).listeners.input();
restore("An old query"); await late.flush(); await tick();
assert.equal(input(late).value, "Beta");
assert.deepEqual(names(late), ["Beta"]);
fail = true;
input(late).value = "Alpha"; input(late).listeners.input();
await assert.rejects(late.flush());
assert.equal(input(late).value, "Alpha");
fail = false; await late.flush();
assert.equal(kept, "Alpha", "Recovered saves must succeed without another edit");
list = []; late.refresh(); await tick();
assert.equal(input(late).hidden, false, "A retained query stays editable when all projects leave");
input(late).value = ""; input(late).listeners.input(); await late.flush();
assert.equal(input(late).hidden, true);
assert.equal(kept, "");
assert.deepEqual(actions, [], "Restoring filters must not open projects, focus sessions or apply memory sharing");
console.log("check-project-search: ok (branch/agent filters, refresh/focus, ordered/deferred saves, recreation, clear/focus, late read, failure/retry, empty projects, no extra data reads or actions)");

const projectReads = [];
const feedback = createRoosts({ ...backend, query: undefined,
  load: () => new Promise((resolve, reject) => projectReads.push({ resolve, reject })) });
feedback.refresh();
assert.ok(feedback.element.textContent.includes("Loading By project"));
assert.ok(!feedback.element.textContent.includes("No projects yet"));
projectReads[0].reject(new Error("Projects unavailable")); await tick();
assert.ok(feedback.element.textContent.includes("Could not load By project"));
assert.ok(!feedback.element.textContent.includes("Loading By project"));
const retryProject = () => nodes(feedback.element).find(n => n.tag === "button" && n.textContent === "Try again");
retryProject().focus(); retryProject().listeners.click();
projectReads[1].resolve([{ name: "Kept", key: "kept", checks: null, checksItem: null, cost: null,
  trees: [{ path: "/work/kept", branch: "main" }], sessions: [], prs: [], memory: [], agents: "none" }]); await tick();
assert.deepEqual(names(feedback), ["Kept"]);
feedback.refresh(); projectReads[2].reject(new Error("Stale projects")); await tick();
assert.deepEqual(names(feedback), ["Kept"]);
assert.ok(feedback.element.textContent.includes("last loaded view is kept"));
assert.deepEqual(actions, []);
console.log("check-project-search: initial loading/error/retry and cached project retention without actions");
