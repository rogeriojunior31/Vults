// Exercise the production Flock renderer's navigation without a browser server.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";

class Element {
  children = []; listeners = {}; attributes = {}; dataset = {}; style = { setProperty() {} };
  value = ""; className = ""; isConnected = true; offsetParent = {};
  classes = new Set();
  classList = { add: cls => this.classes.add(cls), remove: cls => this.classes.delete(cls),
    toggle: (cls, on) => on ? this.classes.add(cls) : this.classes.delete(cls), contains: cls => this.classes.has(cls) };
  constructor(tag) { this.tag = tag; }
  set textContent(text) { this.text = text; this.children = []; }
  get textContent() { return (this.text ?? "") + this.children.map(node => node.textContent).join(""); }
  append(...nodes) { this.children.push(...nodes); }
  replaceChildren(...nodes) { this.text = ""; this.children = nodes; }
  addEventListener(event, fn) { this.listeners[event] = fn; }
  setAttribute(name, value) { this.attributes[name] = value; }
  contains(node) { return nodes(this).includes(node); }
  querySelectorAll(selector) { return nodes(this).filter(node => selector.startsWith(".") ? node.className.split(" ").includes(selector.slice(1)) : node.id === selector.slice(1)); }
  querySelector(selector) { const fid = selector.match(/^\[data-fid="([^"]+)"\]/)?.[1]; return fid ? nodes(this).find(node => node.dataset.fid === fid) : this.querySelectorAll(selector)[0]; }
  focus() { document.activeElement = this; }
  getContext() { return {}; }
}
const nodes = element => [element, ...element.children.flatMap(nodes)];
globalThis.document = { createElement: tag => new Element(tag), createTextNode: text => { const node = new Element("text"); node.textContent = text; return node; }, body: new Element("body"), addEventListener() {} };
globalThis.window = { addEventListener() {}, matchMedia: () => ({ matches: true }), setTimeout: () => 0, clearTimeout() {} };
globalThis.matchMedia = window.matchMedia;
globalThis.ResizeObserver = class { observe() {} };
globalThis.CSS = { escape: value => value };
const root = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({ configFile: false, root, logLevel: "silent", build: {
  write: false, minify: false, lib: { entry: `${root}src/surfaces/nest/flock.ts`, formats: ["es"] },
} });
const chunk = bundles[0].output.find(part => part.type === "chunk");
const { createFlock } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
const tick = () => new Promise(resolve => setImmediate(resolve));
const actions = [];
let kept = { query: "Alpha", onlyNeeds: true, detail: "context" }, fail = false;
const writes = [];
const backend = {
  ...Object.fromEntries(["focus", "jump", "review", "decide", "decideAlways", "answer", "release", "hush", "openFolder", "projectPref", "ask"].map(name => [name, () => actions.push(name)])),
  diff: async () => null, usage: async () => [], loadPlace: async () => ({ ...kept }),
  savePlace: async place => { if (fail) throw new Error("Navigation unavailable."); kept = place; writes.push(place); },
};
const session = { id: "one", agent: "claude", project: "Alpha", cwd: null, status: "approval", attention: "needs-you", card: true,
  activity: null, step: null, steps: [], step_count: 0, subagents: 0, note: null, editor: null, species: "black", bird_chosen: false };
const view = { sessions: [session], alerts: [], front: { agent: "claude", id: "one" },
  approval: { agent: "claude", session: "one", request: "permission", tool: "Bash", target: "git status", description: "Review this permission", questions: [], queue: 1, always: false } };
const mount = () => { const part = createFlock(backend); part.update(view); return part; };
const search = part => nodes(part.element).find(node => node.tag === "input");
const button = (part, text) => nodes(part.element).find(node => node.tag === "button" && node.textContent === text);
const selectedDetail = part => nodes(part.element).find(node => node.attributes["aria-selected"] === "true")?.textContent;
const first = mount();
await tick();
assert.equal(search(first).value, "Alpha");
assert.equal(button(first, "Needs you").attributes["aria-pressed"], "true");
assert.equal(selectedDetail(first), "Context");
search(first).value = "Beta"; search(first).listeners.input();
search(first).value = "No match"; search(first).listeners.input();
button(first, "Activity").listeners.click();
await first.flush();
assert.deepEqual(writes.slice(0, 2).map(place => place.query), ["Beta", "No match"]);
assert.deepEqual(writes.at(-1), { query: "No match", onlyNeeds: true, detail: "activity", change: null });
assert.ok(first.element.textContent.includes("Review this permission"), "A filter must leave the current permission in sight");
const reopened = mount();
await tick();
assert.equal(search(reopened).value, "No match");
assert.equal(selectedDetail(reopened), "Activity");
button(reopened, "Clear filters").listeners.click();
await reopened.flush();
assert.equal(kept.query, "");
assert.equal(kept.onlyNeeds, false);
assert.equal(button(reopened, "Clear filters").hidden, true);
assert.equal(document.activeElement, search(reopened));
let release;
const late = createFlock({ ...backend, loadPlace: () => new Promise(resolve => { release = resolve; }) });
late.update(view);
search(late).value = "My new query"; search(late).listeners.input();
release({ query: "Old query", onlyNeeds: true, detail: "context" });
await late.flush();
assert.equal(search(late).value, "My new query");
assert.equal(selectedDetail(late), "Changes");
fail = true;
await assert.rejects(late.flush());
assert.equal(search(late).value, "My new query");
fail = false;
await late.flush();
assert.equal(kept.query, "My new query");
kept = { query: "", onlyNeeds: false, detail: "zeca" };
const off = mount(); off.setZeca(false);
await off.flush();
assert.equal(selectedDetail(off), "Changes");
assert.equal(kept.detail, "diff");
assert.deepEqual(actions, [], "Restoration and saving must never select a session, answer a permission or start a run");
const decisions = [];
const keptDiff = { files: [{ path: "app.rs", hunks: [{ old_start: 1, new_start: 1, lines: ["-old", "+new"] }] }], cut: false };
const guarded = createFlock({ ...backend, loadPlace: async () => null, diff: async () => keptDiff, decide: (...args) => decisions.push(args) });
const edits = { ...session, status: "working", attention: "info", card: false, steps: ["First edit", "Second edit"], step_count: 2,
  diffs: [{ step: 1, files: 1, added: 1, removed: 0 }, { step: 2, files: 1, added: 2, removed: 0 }] };
guarded.update({ ...view, approval: null, sessions: [edits] }); await tick();
const expand = button(guarded, "Focus diff ↗");
assert.equal(expand.disabled, false);
expand.listeners.click();
const body = nodes(guarded.element).find(node => node.className === "app-body");
assert.equal(body.classList.contains("details-focus"), true);
guarded.update({ ...view, sessions: [{ ...edits, status: "approval", attention: "needs-you", card: true }] });
assert.equal(body.classList.contains("details-focus"), false, "An arriving request must bring its card back");
assert.equal(button(guarded, "Focus diff ↗").disabled, true);
expand.listeners.click();
assert.equal(body.classList.contains("details-focus"), false, "A stale expand control must not hide a new request");
const card = nodes(guarded.element).find(node => node.className === "focus-block");
card.offsetParent = null;
assert.equal(guarded.shortcut(true), false, "An invisible card must not accept a permission shortcut");
card.offsetParent = {};
guarded.setActive(false);
assert.equal(guarded.shortcut(true), false, "An inactive Flock tab must not accept a permission shortcut");
guarded.setActive(true);
assert.deepEqual(decisions, []);
assert.equal(guarded.shortcut(false), true);
assert.equal(guarded.shortcut(false), true);
assert.deepEqual(decisions, [["permission", "deny"]], "A visible card accepts one explicit decision");
guarded.update({ ...view, approval: null, sessions: [{ ...edits, status: "question", attention: "needs-you" }] });
assert.equal(button(guarded, "Focus diff ↗").disabled, true, "A question waiting in its terminal also keeps the card visible");
kept = { query: "", onlyNeeds: false, detail: "diff", change: null };
const diffReads = [];
const diffBackend = { ...backend, diff: async (s, step) => { diffReads.push([s.id, step]); return keptDiff; } };
const normal = { ...view, approval: null, sessions: [edits] };
const mountDiff = async (v = normal) => { const part = createFlock(diffBackend); part.update(v); await tick(); return part; };
const pick = part => nodes(part.element).find(node => node.tag === "select");
const pickedStep = part => pick(part).children.find(option => option.selected)?.value;
const expanded = part => nodes(part.element).find(node => node.className === "app-body").classList.contains("details-focus");
const chosen = await mountDiff();
pick(chosen).value = "1"; pick(chosen).listeners.change();
button(chosen, "Focus diff ↗").listeners.click();
await chosen.flush();
assert.deepEqual(kept.change, { owner: "claude:one", step: 1, focused: true });
const choice = { ...kept, change: { ...kept.change } };
const again = await mountDiff();
assert.equal(pickedStep(again), "1");
assert.equal(expanded(again), true);
assert.ok(again.element.textContent.includes("app.rs"));
assert.ok(diffReads.some(([owner, step]) => owner === "one" && step === 1));
const trimmed = { ...edits, steps: ["Second edit"], diffs: [edits.diffs[1]] };
again.update({ ...normal, sessions: [trimmed] });
assert.equal(pickedStep(again), "2");
assert.equal(expanded(again), false, "An expired change must fall back to the latest kept step and card");
await again.flush();
assert.equal(kept.change.step, 2);
kept = choice;
const another = await mountDiff({ ...normal, front: { agent: "claude", id: "two" }, sessions: [{ ...edits, id: "two" }] });
assert.equal(pickedStep(another), "2");
assert.equal(expanded(another), false, "Another session must not inherit expansion or an old change");
kept = choice;
const waiting = await mountDiff({ ...view, sessions: [{ ...edits, status: "approval", card: true }] });
assert.equal(pickedStep(waiting), "1");
assert.equal(expanded(waiting), false, "Restoring a change must not hide a pending permission");
assert.equal(button(waiting, "Focus diff ↗").disabled, true);
await waiting.flush();
assert.equal(kept.change.focused, false);
let older;
const racing = createFlock({ ...diffBackend, loadPlace: () => new Promise(resolve => { older = resolve; }) });
racing.update(normal);
pick(racing).value = "2"; pick(racing).listeners.change();
older(choice); await racing.flush();
assert.equal(pickedStep(racing), "2", "A late read must not replace the user's new change selection");
assert.equal(expanded(racing), false);
assert.deepEqual(actions, [], "Diff restoration only reads changes; it never focuses or answers a session");
kept = choice;
const unavailableDiff = createFlock({ ...diffBackend, diff: async () => null });
unavailableDiff.update(normal); await tick();
assert.equal(expanded(unavailableDiff), false, "A missing diff body must keep the session card available");
assert.equal(button(unavailableDiff, "Focus diff ↗").disabled, true);
console.log("check-flock-place: ok (filters/detail, ordered saves, recreation, clear/focus, late read, retry, Zeca off, diff selection/expansion, changed owner/expired step/missing body, card visibility, shortcut guards, no restoration actions)");
