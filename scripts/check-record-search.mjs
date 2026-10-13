// Exercise the production records renderer with a small DOM stub, without a browser server.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";

class Element {
  children = [];
  listeners = {};
  value = "";
  className = "";
  text = "";
  constructor(tag) { this.tag = tag; }
  set textContent(text) { this.text = text; this.children = []; }
  get textContent() { return this.text + this.children.map(node => node.textContent).join(""); }
  append(...nodes) { this.children.push(...nodes); }
  replaceChildren(...nodes) { this.text = ""; this.children = nodes; }
  addEventListener(event, listener) { this.listeners[event] = listener; }
  setAttribute() {}
  focus() { globalThis.focused = this; }
}
globalThis.document = { createElement: tag => new Element(tag) };
const root = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({ configFile: false, root, logLevel: "silent", build: {
  write: false, minify: false,
  lib: { entry: `${root}src/surfaces/nest/records.ts`, formats: ["es"] },
} });
const chunk = bundles[0].output.find(part => part.type === "chunk");
const { createHistory, createAudit } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
const nodes = element => [element, ...element.children.flatMap(nodes)];
const count = (element, cls) => nodes(element).filter(node => node.className === cls).length;
const tick = () => new Promise(resolve => setImmediate(resolve));
const turns = ["Alpha", "Beta"].map(project => ({
  project, agent: "claude", end: 100, secs: 5, steps: 1, commands: 0, files: 0, failed: project === "Beta",
}));
let calls = 0;
const history = createHistory(async () => { calls++; return { on: true, turns }; });
history.refresh();
await tick();
const input = nodes(history.element).find(node => node.tag === "input");
const search = query => { input.value = query; input.listeners.input(); };
assert.equal(count(history.element, "record"), 2);
search(" ALPHA ");
assert.equal(count(history.element, "record"), 1);
assert.equal(count(history.element, "records-head"), 1);
search("failed");
assert.equal(count(history.element, "record"), 1);
assert.ok(history.element.textContent.includes("Beta"));
history.refresh();
await tick();
assert.equal(input.value, "failed");
assert.equal(count(history.element, "record"), 1);
search("absent");
assert.equal(count(history.element, "record"), 0);
assert.equal(count(history.element, "records-head"), 0);
assert.ok(history.element.textContent.includes("No records match"));
nodes(history.element).find(node => node.tag === "button").listeners.click();
assert.equal(count(history.element, "record"), 2);
assert.equal(globalThis.focused, input);
assert.equal(calls, 2, "Typing and clearing must not query the backend");
const audit = createAudit(async () => [{
  project: "Alpha", agent: "claude", at: 100, actor: "human", act: "deny", tool: "Bash", target: "git status", consent: null,
}]);
audit.refresh();
await tick();
const auditInput = nodes(audit.element).find(node => node.tag === "input");
for (const query of ["git status", "denied", "you", "alpha", "bash"]) {
  auditInput.value = query;
  auditInput.listeners.input();
  assert.equal(count(audit.element, "record"), 1, query);
}
const empty = createHistory(async () => ({ on: false, turns: [] }));
empty.refresh();
await tick();
assert.ok(empty.element.textContent.includes("turn it on there"));
assert.ok(empty.element.textContent.includes("No finished turn"));
console.log("check-record-search: ok (groups, status, command, clear/focus, refresh, empty, no extra queries)");
