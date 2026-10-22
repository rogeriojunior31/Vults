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
  style = {};
  dataset = {};
  attributes = {};
  constructor(tag) { this.tag = tag; }
  set textContent(text) { this.text = text; this.children = []; }
  get textContent() { return this.text + this.children.map(node => node.textContent).join(""); }
  append(...nodes) { this.children.push(...nodes); }
  replaceChildren(...nodes) { this.text = ""; this.children = nodes; }
  addEventListener(event, listener) { this.listeners[event] = listener; }
  setAttribute(name, value) { this.attributes[name] = value; }
  getAttribute(name) { return this.attributes[name] ?? null; }
  hasAttribute(name) { return name in this.attributes; }
  matches(tags) { return tags.split(",").some(tag => tag.trim() === this.tag); }
  focus() { globalThis.focused = document.activeElement = this; }
}
globalThis.document = { createElementNS: (_ns, tag) => new Element(tag), createElement: tag => new Element(tag) };
const root = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({ configFile: false, root, logLevel: "silent", build: {
  write: false, minify: false,
  lib: { entry: `${root}src/surfaces/nest/records.ts`, formats: ["es"] },
} });
const chunk = bundles[0].output.find(part => part.type === "chunk");
const { createHistory, createAudit, createTraces, createUsage } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
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
assert.ok(empty.element.textContent.includes("History is off"));
const kept = { history: "failed", audit: "git status" };
const writes = [];
const query = async (tab, value) => {
  if (value !== undefined) { kept[tab] = value; writes.push([tab, value]); }
  return kept[tab];
};
let loads = 0;
const loadHistory = async () => { loads++; return { on: true, turns }; };
const restored = createHistory(loadHistory, query);
restored.refresh();
await tick();
const restoredInput = nodes(restored.element).find(node => node.tag === "input");
assert.equal(restoredInput.value, "failed");
assert.equal(count(restored.element, "record"), 1);
restoredInput.value = "Alpha";
restoredInput.listeners.input();
restoredInput.value = "Beta";
restoredInput.listeners.input();
await restored.flush();
assert.deepEqual(writes, [["history", "Alpha"], ["history", "Beta"]], "Navigation saves must settle in typing order before closing");
assert.equal(loads, 1, "Saving navigation must not reload the records");
const reopened = createHistory(loadHistory, query);
reopened.refresh();
await tick();
assert.equal(nodes(reopened.element).find(node => node.tag === "input").value, "Beta");
assert.equal(count(reopened.element, "record"), 1);
assert.ok(reopened.element.textContent.includes("Beta"));
nodes(reopened.element).find(node => node.tag === "button").listeners.click();
await reopened.flush();
assert.equal(kept.history, "");
assert.equal(kept.audit, "git status", "Clearing History must leave Audit's query alone");
let release;
const late = createHistory(loadHistory, (tab, value) => value === undefined
  ? new Promise(resolve => { release = resolve; }) : query(tab, value));
late.refresh();
const lateInput = nodes(late.element).find(node => node.tag === "input");
lateInput.value = "My new search";
lateInput.listeners.input();
release("An older search");
await late.flush();
assert.equal(lateInput.value, "My new search", "A late navigation read must not overwrite typing");
assert.equal(kept.history, "My new search");
const unavailable = createHistory(loadHistory, async () => { throw "Navigation unavailable."; });
unavailable.refresh();
await tick();
const unavailableInput = nodes(unavailable.element).find(node => node.tag === "input");
unavailableInput.value = "Alpha";
unavailableInput.listeners.input();
await assert.rejects(unavailable.flush());
assert.equal(unavailableInput.value, "Alpha", "A failed save must leave the current search editable");
assert.equal(count(unavailable.element, "record"), 1);
let available = false;
const retry = createHistory(loadHistory, async (tab, value) => {
  if (!available) throw new Error("Navigation unavailable.");
  return query(tab, value);
});
await tick();
const retryInput = nodes(retry.element).find(node => node.tag === "input");
retryInput.value = "Beta";
retryInput.listeners.input();
await assert.rejects(retry.flush());
available = true;
await retry.flush();
assert.equal(kept.history, "Beta", "Retry closing must save the retained query without another edit");
assert.equal(retryInput.value, "Beta");
console.log("check-record-search: ok (groups, status, command, clear/focus, refresh, recreation, save ordering, late read, error retention, empty, no extra record queries)");

const figures = element => nodes(element).filter(node => node.className === "activity-stat").map(node => node.children[0].textContent);
assert.deepEqual(figures(history.element), ["2", "0", "1"], "History summarizes only the returned turns");
search("Alpha");
assert.deepEqual(figures(history.element), ["2", "0", "1"], "Filtering the list does not recalculate loaded-record totals");
assert.equal(nodes(history.element).find(node => node.className === "record-count").textContent, "1 record");

const auditSummary = createAudit(async () => [
  { project: "Alpha", agent: "codex", at: 100, actor: "policy", act: "allow", tool: "Bash", target: "git diff -- src/file.ts", consent: 37 },
  { project: "Alpha", agent: "codex", at: 101, actor: "human", act: "deny", tool: "Bash", target: "rm secret.txt", consent: null },
]);
auditSummary.refresh(); await tick();
assert.deepEqual(figures(auditSummary.element), ["2", "1", "1"]);
assert.ok(auditSummary.element.textContent.includes("git diff -- src/file.ts"));
assert.ok(auditSummary.element.textContent.includes("Policy accepted as #37"));
assert.equal(nodes(auditSummary.element).find(node => node.className === "record-count").textContent, "2 records");

const traces = createTraces(async () => [
  { kind: "chat", name: "chat model", status: "ok", startMs: 100, ms: 1500, child: false, input: 100, output: 30, costUsd: 0.5 },
  ...["denied", "error"].map(status => ({ kind: "execute_tool", name: `execute_tool ${status}`, status, startMs: 101, ms: 25, child: true, input: null, output: null, costUsd: null })),
]);
traces.refresh(); await tick();
assert.deepEqual(figures(traces.element), ["1", "2", "2"]);
assert.equal(count(traces.element, "record child"), 2, "Tool rows retain their parent indentation");
assert.ok(traces.element.textContent.includes("100 in, 30 out"));
assert.ok(!traces.element.textContent.includes("0 in, 0 out"), "Missing token counts are never invented");

const usage = createUsage(async () => ({
  windows: [{ agent: "codex", minutes: 300, used_percent: 70 }, { agent: "claude", minutes: 10080, used_percent: 95 }],
  days: [{ day: "2026-10-14", usd: 1.25, tokens: 100 }, { day: "2026-10-15", usd: 2.75, tokens: 200 }],
  roosts: [], budget: 2, subscription: true,
}));
usage.refresh(); await tick();
assert.deepEqual(figures(usage.element), ["$4.00", "300", "2"]);
assert.ok(usage.element.textContent.includes("your plan does not charge it"));
assert.equal(count(usage.element, "usage-fill warning"), 1);
assert.equal(count(usage.element, "usage-fill critical"), 1);
assert.equal(count(usage.element, "usage-fill over"), 1);
console.log("check-record-search: loaded totals, filtered group counts, full audited command/consent, trace hierarchy, missing tokens, usage totals and warning thresholds");

const weekBundle = await build({ configFile: false, root, logLevel: "silent", build: {
  write: false, minify: false, lib: { entry: `${root}src/surfaces/nest/week-view.ts`, formats: ["es"] },
} });
const weekChunk = weekBundle[0].output.find(part => part.type === "chunk");
const { nestWeek } = await import(`data:text/javascript;base64,${Buffer.from(weekChunk.code).toString("base64")}`);
const actions = [];
const on = Object.fromEntries(["week", "tab", "connectors", "hideProjects", "saveImage", "history", "confirmClear", "clear"].map(name => [name, () => actions.push(name)]));
const activity = { history: true, today: "2026-10-15", weeks: ["2026-10-12", "2026-10-05"], grid: [], week: {
  monday: "2026-10-12", sunday: "2026-10-18", headline: "Recorded on this computer", turns: 2, active_secs: 9090, days: [0, 30, 60, 3600, 5400, 0, 0],
  longest_secs: 5400, steps: 5, files: 1, added: 10, removed: 2, allowed: 1, denied: 1, answered: 1, top_agent: null, top_project: null, busiest_day: null,
} };
const view = { tab: "agents", github: null, confirming: false, hideProjects: false, saved: null };
const week = new Element("main");
week.append(...nestWeek(activity, view, on, null, day => actions.push(["day", day])));
assert.deepEqual(nodes(week).filter(node => node.className === "day-total").map(node => node.textContent), ["—", "<1 min", "1 min", "1 h", "1.5 h", "—", "—"]);
const dayButtons = nodes(week).filter(node => node.className === "day-column");
assert.equal(dayButtons[4].attributes["aria-pressed"], "true");
assert.deepEqual(actions, [], "Rendering the redesign never shares, clears history or changes a setting");
dayButtons[3].listeners.click();
assert.deepEqual(actions, [["day", 3]]);
week.replaceChildren(...nestWeek({ ...activity, week: { ...activity.week, days: [0, 0, 0, 0, 0, 0, 0] } }, view, on, null, () => {}));
assert.ok(nodes(week).filter(node => node.className === "day-column").every(node => node.attributes["aria-pressed"] === "false"), "No counted time means no invented busiest day");
console.log("check-record-search: week day labels, exact detail names, busiest day, explicit selection and zero-time guard");

const quickBundle = await build({ configFile: false, root, logLevel: "silent", build: {
  write: false, minify: false, lib: { entry: `${root}src/island/quick.ts`, formats: ["es"] },
} });
const quickChunk = quickBundle[0].output.find(part => part.type === "chunk");
const { weekQuick } = await import(`data:text/javascript;base64,${Buffer.from(quickChunk.code).toString("base64")}`);
const quickActions = [];
const open = () => quickActions.push("open");
const nav = { previous: false, go: previous => quickActions.push(previous) };
const quickWeek = { ...activity.week, projects: Array.from({ length: 5 }, (_, i) => ({ name: `Project ${i}`, secs: (5 - i) * 60 })) };
const quick = weekQuick(quickWeek, true, open, false, nav);
const quickFigures = node => nodes(node).filter(node => node.className === "quick-stat").map(node => node.children[0].textContent);
assert.deepEqual(quickFigures(quick), ["2.5 h", "2", "+10 −2", "3"]);
assert.equal(nodes(quick).find(node => node.className === "quick-stat").title, "2 h 31 min");
assert.equal(count(quick, "quick-bar"), 4, "The compact preview keeps at most four projects");
assert.ok(!quick.textContent.includes("Project 4"));
const weekButtons = nodes(quick).filter(node => node.className.startsWith("quick-week-btn"));
assert.deepEqual(weekButtons.map(node => node.attributes["aria-pressed"]), ["false", "true"]);
assert.deepEqual(weekButtons.map(node => node.attributes["aria-label"]), ["Previous week", "This week"]);
assert.deepEqual(quickActions, [], "Rendering the compact view never navigates or changes history");
weekButtons[0].listeners.click();
weekButtons[1].listeners.click();
nodes(quick).find(node => node.className === "quick-foot").children[0].listeners.click();
assert.deepEqual(quickActions, [true, false, "open"]);
for (const failed of [false, true]) {
  const unread = weekQuick(null, true, open, failed, nav);
  assert.deepEqual(quickFigures(unread), [], "Unavailable measurements never show zero totals");
  assert.ok(unread.textContent.includes(failed ? "Could not read it." : "Reading…"));
}
const off = weekQuick({ ...quickWeek, turns: 0, active_secs: 0, projects: [] }, false, open, false, nav);
assert.equal(quickFigures(off)[0], "0 min");
assert.equal(count(off, "quick-weeks"), 0);
assert.ok(off.textContent.includes("local history is off"));
const byDay = weekQuick({ ...quickWeek, active_secs: 30, projects: [], days: [0, 0, 0, 0, 0, 0, 0] }, true, open);
assert.equal(quickFigures(byDay)[0], "<1 min");
assert.equal(count(byDay, "quick-day-column"), 7);
assert.ok(nodes(byDay).filter(node => node.className === "quick-fill").every(node => node.style.height === "0%"));
console.log("check-record-search: compact week totals/full duration, four-project preview, explicit navigation, loading/error/off states, zero and day fallback");

// Failed reads retain the last snapshot for every record surface and retry only on request.
for (const [make, snapshot, rowsExpected] of [
  [createHistory, { on: true, turns }, 2],
  [createAudit, [{ project: "Retained", agent: "codex", at: 1, actor: "human", act: "deny", tool: "Bash", target: "git status", consent: null }], 1],
  [createTraces, [{ kind: "chat", name: "Retained trace", status: "ok", startMs: 1, ms: 1, child: false, input: null, output: null, costUsd: null }], 1],
  [createUsage, { windows: [], days: [], roosts: [], budget: null, subscription: false }, 0],
]) {
  const pending = [];
  const part = make(() => new Promise((resolve, reject) => pending.push({ resolve, reject })));
  part.refresh(); assert.equal(count(part.element, "view-state loading"), 1);
  assert.deepEqual(figures(part.element), [], "Loading is not a zero-value summary");
  pending[0].reject(new Error("Disk unavailable")); await tick();
  assert.equal(count(part.element, "view-state error"), 1);
  assert.equal(count(part.element, "view-state loading"), 0);
  const retry = nodes(part.element).find(n => n.tag === "button" && n.textContent === "Try again");
  const detail = nodes(part.element).find(n => n.className === "view-state-detail");
  assert.notEqual(detail.open, true, "Raw errors start inside a closed disclosure");
  retry.focus(); retry.listeners.click(); retry.listeners.click();
  assert.equal(pending.length, 2, "A pending retry cannot duplicate a read");
  pending[1].resolve(snapshot); await tick();
  assert.equal(count(part.element, "view-state error"), 0); assert.equal(count(part.element, "record"), rowsExpected);
  const before = figures(part.element);
  part.refresh(); pending[2].reject(new Error("Refresh unavailable")); await tick();
  assert.equal(count(part.element, "record"), rowsExpected);
  assert.deepEqual(figures(part.element), before);
  assert.ok(part.element.textContent.includes("last loaded view is kept"));
  part.refresh(); const obsolete = pending.at(-1); part.refresh(); pending.at(-1).resolve(snapshot); await tick();
  obsolete.reject(new Error("Obsolete error")); await tick();
  assert.equal(count(part.element, "view-state error"), 0);
}
console.log("check-record-search: all record surfaces distinguish loading/error/empty, retain snapshots and summaries, explicit guarded retry and obsolete response rejection");

for (const [create, label, value] of [
  [createHistory, "Search turns", { on: true, turns }],
  [createAudit, "Search answers and commands", []],
]) {
  let reads = 0;
  const writes = [];
  const view = create(async () => { reads++; return value; }, async (_tab, text) => { if (text !== undefined) writes.push(text); return ""; });
  view.refresh(); await tick();
  const input = nodes(view.element).find(node => node.tag === "input");
  const clear = nodes(view.element).find(node => node.tag === "button");
  assert.equal(input.placeholder, label);
  assert.equal(input.getAttribute("aria-label"), label);
  assert.equal(clear.disabled, true, "An empty search has no clear action");
  input.value = "Alpha"; input.listeners.input();
  assert.equal(clear.disabled, false);
  for (const composition of [{ isComposing: true }, { keyCode: 229 }]) {
    input.listeners.keydown({ key: "Escape", ...composition, preventDefault() { assert.fail("IME keeps Escape"); }, stopPropagation() { assert.fail("IME keeps its key event"); } });
    assert.equal(input.value, "Alpha");
  }
  let prevented = 0, stopped = 0;
  input.listeners.keydown({ key: "Escape", preventDefault() { prevented++; }, stopPropagation() { stopped++; } });
  await view.flush();
  assert.equal(input.value, "");
  assert.equal(document.activeElement, input);
  assert.equal(clear.disabled, true);
  assert.equal(prevented, 1); assert.equal(stopped, 1);
  assert.deepEqual(writes, ["Alpha", ""], "Escape saves the clear after older search writes");
  input.listeners.keydown({ key: "Escape", preventDefault() { assert.fail("An empty search leaves Escape to the owning surface"); }, stopPropagation() { assert.fail("An empty search leaves Escape to the owning surface"); } });
  clear.listeners.click(); await view.flush();
  assert.deepEqual(writes, ["Alpha", ""], "A stale clear button cannot queue redundant writes");
  assert.equal(reads, 1, "Clearing stays on the already loaded records");
}
console.log("check-record-search: prototype-specific History/Audit search labels, empty-clear state, IME-safe Escape, retained focus, ordered clear persistence and no extra record queries");
