// Check production task actions without a browser server or test dependency.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";

class Element {
  children = [];
  listeners = {};
  className = "";
  text = "";
  constructor(tag) { this.tag = tag; }
  set textContent(text) { this.text = text; this.children = []; }
  get textContent() { return this.text + this.children.map(node => node.textContent).join(""); }
  append(...nodes) { this.children.push(...nodes); }
  replaceChildren(...nodes) { this.children = nodes; }
  addEventListener(event, listener) { this.listeners[event] = listener; }
  setAttribute() {}
}
globalThis.document = { createElement: tag => new Element(tag) };
const root = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({ configFile: false, root, logLevel: "silent", build: {
  write: false, minify: false, lib: { entry: `${root}src/surfaces/nest/tasks.ts`, formats: ["es"] },
} });
const chunk = bundles[0].output.find(part => part.type === "chunk");
const { createTasks } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
const nodes = element => [element, ...element.children.flatMap(nodes)];
const tick = () => new Promise(resolve => setImmediate(resolve));
let task = {
  id: 7, goal: "Fix login", criteria: [{ text: "Pass tests", done: false }], status: "doing", owner: "codex",
  roost: "Project", roostKey: "project", costUsd: null, tokens: null,
  running: false, canStart: true, hasSession: true, sessionEngine: "claude", hasWorktree: true,
};
const calls = [];
const backend = {
  load: async () => [task], today: async () => null, zeca: () => false, folders: () => [], agents: () => [],
  ...Object.fromEntries(["start", "startFresh", "review", "stop", "edit", "remove", "removeWorktree", "visit"].map(
    name => [name, async (...args) => { calls.push([name, ...args]); }],
  )),
};
const tasks = createTasks(backend);
const refresh = async () => { tasks.refresh(); await tick(); };
await refresh();
const maintenance = () => nodes(tasks.element).find(node => node.className === "task-maintenance");
const button = text => nodes(tasks.element).find(node => node.tag === "button" && node.textContent === text);
const click = async text => { const node = button(text); assert.ok(node, text); node.listeners.click(); await tick(); };
assert.equal(maintenance().open, false);
assert.ok(tasks.element.textContent.includes("This turn ended"));
assert.ok(!nodes(maintenance()).includes(button("Go on")), "Continuation must stay outside disclosure");
assert.ok(nodes(maintenance()).includes(button("Start fresh")));
assert.ok(nodes(maintenance()).includes(button("Review with Codex")));
assert.ok(nodes(maintenance()).includes(button("Hand off to Codex")));
maintenance().open = true;
await refresh();
assert.equal(maintenance().open, true, "Refresh must retain disclosure");
maintenance().open = false;
await refresh();
assert.equal(maintenance().open, false, "Refresh must also retain a closed disclosure");
maintenance().open = true;
await click("Go on");
assert.deepEqual(calls.pop(), ["start", 7, "claude"], "Continue uses session engine, not selected owner");
await click("Hand off to Codex");
assert.deepEqual(calls.pop(), ["startFresh", 7, "codex"]);
await click("Review with Codex");
assert.deepEqual(calls.pop(), ["review", 7]);
await click("Delete");
assert.equal(calls.length, 0);
assert.equal(maintenance().open, true);
await click("Delete for good?");
assert.deepEqual(calls.pop(), ["remove", 7]);
await click("Remove its folder");
assert.equal(calls.length, 0);
await click("Uncommitted work in it is lost. Remove?");
assert.deepEqual(calls.pop(), ["removeWorktree", 7]);
task = { ...task, running: true, canStart: false };
await refresh();
assert.ok(!tasks.element.textContent.includes("This turn ended"));
assert.ok(!nodes(maintenance()).includes(button("Stop")));
assert.equal(button("Remove its folder"), undefined);
await click("Stop");
assert.deepEqual(calls.pop(), ["stop", 7]);
task = { ...task, running: false, canStart: true, hasSession: false };
await refresh();
assert.ok(!nodes(maintenance()).includes(button("Start with Claude Code")));
await click("Start with Codex");
assert.deepEqual(calls.pop(), ["start", 7, "codex"]);
task = { ...task, status: "blocked", hasSession: true, reply: "Partial work\n\nThe CLI failed." };
await refresh();
assert.ok(!tasks.element.textContent.includes("This turn ended"), "A failure must not look like a normal finished turn");
const lastReply = nodes(tasks.element).find(node => node.className === "task-reply");
assert.equal(lastReply.open, true, "Blocked task output must be visible without another click");
assert.ok(lastReply.textContent.includes("Partial work"));
assert.ok(lastReply.textContent.includes("The CLI failed."));
lastReply.open = false;
await refresh();
assert.equal(nodes(tasks.element).find(node => node.className === "task-reply").open, false, "Refresh must keep the user's closed output");
task = { ...task, reply: "A new failure" };
await refresh();
assert.equal(nodes(tasks.element).find(node => node.className === "task-reply").open, true, "A new failure must be shown");
console.log("check-task-actions: ok (primary actions, disclosure refresh, session owner, handoff/review, distinct confirmations)");
