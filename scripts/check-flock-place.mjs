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
globalThis.document = { createElement: tag => new Element(tag), createElementNS: (_ns, tag) => new Element(tag), createTextNode: text => { const node = new Element("text"); node.textContent = text; return node; }, body: new Element("body"), addEventListener() {} };
globalThis.window = { addEventListener() {}, matchMedia: () => ({ matches: true, addEventListener() {} }), setTimeout: () => 0, clearTimeout() {} };
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
assert.equal(first.element.querySelector(".attention-filter").attributes["aria-pressed"], "true");
assert.equal(selectedDetail(first), "Context");
search(first).value = "Beta"; search(first).listeners.input();
search(first).value = "No match"; search(first).listeners.input();
button(first, "Activity").listeners.click();
await first.flush();
assert.deepEqual(writes.slice(0, 2).map(place => place.query), ["Beta", "No match"]);
assert.deepEqual(writes.at(-1), { query: "No match", onlyNeeds: true, status: null, detail: "activity", change: null });
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
assert.equal(selectedDetail(late), "Session");
fail = true;
await assert.rejects(late.flush());
assert.equal(search(late).value, "My new query");
fail = false;
await late.flush();
assert.equal(kept.query, "My new query");
kept = { query: "", onlyNeeds: false, detail: "zeca" };
const off = mount(); off.setZeca(false);
await off.flush();
assert.equal(selectedDetail(off), "Session");
assert.equal(kept.detail, "session");
assert.deepEqual(actions, [], "Restoration and saving must never select a session, answer a permission or start a run");
const folderCard = createFlock({ ...backend, loadPlace: async () => null });
folderCard.update({ ...view, sessions: [{ ...session, cwd: '/w/session' }], approval: { ...view.approval, cwd: '/w/actual-command' } });
assert.ok(folderCard.element.textContent.includes('Working folder: /w/actual-command'));
assert.ok(button(folderCard, 'Always allow').title.includes('/w/actual-command'));
folderCard.update({ ...view, approval: { ...view.approval, cwd: '../relative' } });
assert.ok(!button(folderCard, 'Always allow'), 'An unresolved folder cannot create a persistent rule');
folderCard.update({ ...view, approval: null, sessions: [{ ...session, status: 'working', attention: 'info', card: false, agent: 'opencode' }], front: { agent: 'opencode', id: 'one' } });
button(folderCard, 'Context').listeners.click();
assert.ok(folderCard.element.textContent.includes('Watch sessions'));
assert.ok(folderCard.element.textContent.includes('Answer permissions'));
assert.ok(folderCard.element.textContent.includes('Planned; not enabled here'), 'ACP metadata must not advertise a finished drive feature');
const decisions = [];
const keptDiff = { files: [{ path: "app.rs", hunks: [{ old_start: 1, new_start: 1, lines: ["-old", "+new"] }] }], cut: false };
const guarded = createFlock({ ...backend, loadPlace: async () => null, diff: async () => keptDiff, decide: (...args) => decisions.push(args) });
const edits = { ...session, status: "working", attention: "info", card: false, steps: ["First edit", "Second edit"], step_count: 2,
  diffs: [{ step: 1, files: 1, added: 1, removed: 0 }, { step: 2, files: 1, added: 2, removed: 0 }],
  changes: [{ text: "First edit", step: 1, files: 1, added: 1, removed: 0 }, { text: "Second edit", step: 2, files: 1, added: 2, removed: 0 }],
  touched: [{ path: "gen.rs", added: 0, removed: 0, edits: 0, step: null, file: null, outside: true }, { path: "app.rs", added: 3, removed: 0, edits: 2, step: 2, file: 0 }] };
guarded.update({ ...view, approval: null, sessions: [edits] }); await tick();
guarded.element.querySelector("#nest-detail-diff").listeners.click();
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
assert.ok(again.element.textContent.includes("Files changed in this session (2)"), "Changes lists every file the session changed");
assert.ok(again.element.textContent.includes("by a command"), "A file only a command wrote says so");
assert.ok(diffReads.some(([owner, step]) => owner === "one" && step === 1));
const trimmed = { ...edits, steps: ["Second edit"], diffs: [edits.diffs[1]], changes: [edits.changes[1]] };
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
racing.element.querySelector("#nest-detail-diff").listeners.click();
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

let stageWidth = 960, reduced = true;
Object.defineProperty(Element.prototype, "clientWidth", { get() { return this.className === "habitat-sky" ? stageWidth : 0; } });
window.matchMedia = () => ({ matches: reduced, addEventListener() {} });
globalThis.matchMedia = window.matchMedia;
const focused = [];
const many = Array.from({ length: 12 }, (_, i) => ({ ...session, id: `run-${i}`, project: `Project ${i} with a long name`,
  branch: `feature/session-${i}`, status: i === 2 ? "question" : i === 5 ? "approval" : "working",
  attention: i === 2 || i === 5 ? "needs-you" : "info", waiting: i === 10, card: false }));
const polished = createFlock({ ...backend, loadPlace: async () => null, focus: s => focused.push(s.id) });
const flockView = { sessions: many, alerts: [], approval: null, front: { agent: "claude", id: "run-5" } };
polished.update(flockView); await tick();
const birdButtons = () => nodes(polished.element).filter(node => node.className.split(" ").includes("habitat-bird"));
const roostCards = () => polished.element.querySelectorAll(".roost-card");
const filter = polished.element.querySelector(".attention-filter");
assert.equal(polished.element.querySelector(".attention-filter").querySelector(".attention-count").textContent, "3");
assert.equal(filter.attributes["aria-label"], "3 sessions need you");
assert.equal(birdButtons().length, 8, "The existing scene retains eight full-size birds at this width");
assert.equal(roostCards().length, 12, "Extra sessions keep their existing cards");
const selectedBird = birdButtons().find(node => node.attributes["aria-pressed"] === "true");
const selectedRoost = roostCards().find(node => node.attributes["aria-pressed"] === "true");
assert.equal(selectedBird.dataset.fid, "bird:claude:run-5");
assert.equal(selectedRoost.dataset.fid, "roost:claude:run-5");
assert.equal(selectedRoost.style.gridColumn, undefined, "Session rows stay independent of bird slots");
assert.ok(selectedBird.textContent.includes(many[5].project), "Wire birds have visible names");
assert.ok(selectedRoost.title.includes("Project 5 with a long name"));
assert.ok(selectedRoost.title.includes("feature/session-5"), "Truncated labels retain full project and branch context");
search(polished).value = "Project 2"; search(polished).listeners.input();
assert.equal(polished.element.querySelector(".attention-filter").querySelector(".attention-count").textContent, "3", "Search does not change the flock-wide attention count");
assert.equal(birdButtons().length, 8, "Filtering leaves every bird in its existing place");
assert.equal(polished.element.querySelector(".card-name").textContent, many[5].project, "Filtering never replaces the selected session's details");
button(polished, "Clear filters").listeners.click();
stageWidth = 480; polished.update(flockView);
assert.equal(birdButtons().length, 5, "Narrow Room uses the existing scene capacity without shrinking sprites");
polished.update({ ...flockView, front: { agent: "claude", id: "run-11" } });
assert.ok(birdButtons().every(node => node.attributes["aria-pressed"] === "false"), "An overflow session never highlights another bird");
assert.equal(roostCards().find(node => node.attributes["aria-pressed"] === "true").dataset.fid, "roost:claude:run-11");
assert.deepEqual(focused, [], "Updating selection, size and filters never changes core focus");
roostCards().find(node => node.dataset.fid === "roost:claude:run-2").listeners.click();
assert.deepEqual(focused, ["run-2"], "Only an explicit card click selects that session");
reduced = false;
button(polished, "Activity").listeners.click();
const content = polished.element.querySelector(".detail-content");
assert.equal(content.classList.contains("detail-enter"), true);
content.listeners.animationend();
polished.update({ ...flockView, front: { agent: "claude", id: "run-11" } });
assert.equal(content.classList.contains("detail-enter"), false, "Live updates never replay tab entrance motion");
reduced = true;
button(polished, "Context").listeners.click();
assert.equal(content.classList.contains("detail-enter"), false, "Reduced motion skips the entrance");
reduced = false; document.body.classList.add("still");
button(polished, "Activity").listeners.click();
assert.equal(content.classList.contains("detail-enter"), false, "Manual pause also skips the entrance");
document.body.classList.remove("still");
assert.deepEqual(actions, [], "Visual refinements never answer, start or stop a session");
console.log("check-flock-place: attention count, matching bird/card selection, long labels, 12 sessions, narrow/overflow selection, explicit focus and calm tab motion");

const deferredRequests = [];
const skipping = createFlock({ ...backend, loadPlace: async () => null, skip: request => deferredRequests.push(request) });
skipping.update(view);
const staleAllow = button(skipping, "Allow");
skipping.element.querySelector('[data-fid="act:skip"]').listeners.click();
assert.deepEqual(deferredRequests, ["permission"]);
assert.equal(skipping.shortcut(true), false, "Skipping clears the visible card before the backend round trip");
const beforeStaleClick = actions.length; staleAllow.listeners.click();
assert.equal(actions.length, beforeStaleClick, "The old Allow control cannot answer a skipped card");
skipping.update({ ...view, approval: null, sessions: [{ ...session, card: false, waiting: true, skipped: true, request: "permission" }] });
assert.ok(skipping.element.textContent.includes("The request still waits for your answer"));
button(skipping, "Show the card").listeners.click(); assert.equal(actions.at(-1), "focus");
skipping.update(view); assert.equal(skipping.shortcut(true), true, "The same request can be answered after it is explicitly reopened");
assert.equal(actions.at(-1), "decide");
console.log("check-flock-place: Skip keeps a resume action, blocks stale mouse/shortcut decisions and restores the same request");

// An overview reads only its selected session's real snapshots, not prototype formulas.
const overviewReads = [], overviewActions = [], overviewWrites = [];
const summaryUI = createFlock({ ...backend, loadPlace: async () => null,
  diff: async (...args) => { overviewReads.push(args); return keptDiff; },
  savePlace: async place => overviewWrites.push(place),
  focus: s => overviewActions.push(['focus', s.id]), decide: (...args) => overviewActions.push(['decide', ...args]),
});
const reported = { ...edits, elapsed_ms: 840_000, context_percent: 42.5, cost_usd: 0.37 };
const unknown = { ...reported, id: 'unknown', agent: 'codex', project: 'Beta', elapsed_ms: null, context_percent: null, cost_usd: null };
const summaryView = { ...normal, front: { agent: 'claude', id: 'one' }, sessions: [reported, unknown] };
summaryUI.update(summaryView); await tick();
assert.equal(selectedDetail(summaryUI), 'Session');
let metrics = summaryUI.element.querySelector('.session-metrics');
assert.ok(metrics.textContent.includes('14 min') && metrics.textContent.includes('≈$0.37') && metrics.textContent.includes('43%'));
assert.equal(metrics.querySelector('.session-context-meter').attributes.value, '42.5');
assert.ok(summaryUI.element.querySelector('.session-recent').textContent.includes('Second edit'));
assert.ok(summaryUI.element.querySelector('.session-files').textContent.includes('app.rs'));
assert.ok(summaryUI.element.querySelector('.session-files').textContent.includes('by a command'));
assert.equal(overviewReads.length, 0, 'Drawing the overview does not read a diff or perform an action');
summaryUI.element.querySelector('.roost-cards').scrollTop = 100;
summaryUI.element.querySelector('.flock-selected').scrollTop = 80;
button(summaryUI, 'All steps').listeners.click();
assert.equal(selectedDetail(summaryUI), 'Activity');
button(summaryUI, 'Session').listeners.click();
assert.equal(summaryUI.element.querySelector('.roost-cards').scrollTop, 100);
assert.equal(summaryUI.element.querySelector('.flock-selected').scrollTop, 80);
summaryUI.update({ ...summaryView, front: { agent: 'codex', id: 'unknown' } });
metrics = summaryUI.element.querySelector('.session-metrics');
assert.equal(metrics.textContent.split('Not reported').length - 1, 3);
assert.equal(metrics.querySelector('.session-context-meter'), undefined, 'Unknown context has no invented bar');
summaryUI.update({ ...summaryView, sessions: [{ ...reported, elapsed_ms: 0, context_percent: 0, cost_usd: 0 }, unknown] });
metrics = summaryUI.element.querySelector('.session-metrics');
assert.ok(metrics.textContent.includes('0 s') && metrics.textContent.includes('≈$0.00') && metrics.textContent.includes('0%'));
assert.equal(metrics.querySelector('.session-context-meter').attributes.value, '0');
const codexReport = { ...unknown, context_percent: 19.077684, context_tokens: 90619 };
summaryUI.update({ ...summaryView, front: { agent: 'codex', id: 'unknown' }, sessions: [reported, codexReport] });
assert.ok(summaryUI.element.querySelector('.session-metrics').textContent.includes('19%'));
button(summaryUI, 'Context').listeners.click();
assert.ok(summaryUI.element.querySelector('.context-dl').textContent.includes('90619'));
const opencodeReport = { ...unknown, agent: 'opencode', context_tokens: 11948, cost_usd: 0.125 };
summaryUI.update({ ...summaryView, front: { agent: 'opencode', id: 'unknown' }, sessions: [reported, opencodeReport] });
button(summaryUI, 'Session').listeners.click();
metrics = summaryUI.element.querySelector('.session-metrics');
assert.ok(metrics.textContent.includes('11948 tokens') && metrics.textContent.includes('≈$0.13'));
assert.equal(metrics.querySelector('.session-context-meter'), undefined, 'Tokens alone do not invent a model limit');
const healthy = { ...opencodeReport, metrics_age_secs: 2, metrics_checked_age_secs: 0, event_age_secs: 60, elapsed_ms: 2500 };
const withHealth = session => ({ ...summaryView, front: { agent: session.agent, id: session.id }, sessions: [session] });
summaryUI.update(withHealth(healthy));
assert.ok(summaryUI.element.textContent.includes('Metrics read 2 s ago.'));
assert.ok(summaryUI.element.querySelector('.session-metrics').textContent.includes('Model response time'));
summaryUI.update(withHealth({ ...healthy, metrics_failed: true }));
assert.ok(summaryUI.element.textContent.includes('Local session metrics could not be read.'));
assert.ok(summaryUI.element.querySelector('.session-metrics').textContent.includes('≈$0.13'), 'Failed reads retain the last value, visibly marked');
summaryUI.update(withHealth({ ...healthy, metrics_checked_age_secs: 16 }));
assert.ok(summaryUI.element.textContent.includes('Metrics checks are delayed.'));
summaryUI.update(withHealth({ ...reported, status: 'working', metrics_checked_age_secs: 31 }));
assert.ok(summaryUI.element.textContent.includes('No recent metrics. Check the Claude status line'));
summaryUI.update(withHealth({ ...reported, status: 'idle', metrics_age_secs: 31, metrics_checked_age_secs: 31 }));
assert.ok(!summaryUI.element.textContent.includes('No recent metrics.'), 'A quiet agent is not a disconnected agent');
summaryUI.update(withHealth({ ...healthy, metrics_age_secs: null }));
assert.ok(summaryUI.element.textContent.includes('Waiting for the first metrics report.'));
summaryUI.update(withHealth(healthy));
button(summaryUI, 'Context').listeners.click();
assert.ok(summaryUI.element.querySelector('.context-dl').textContent.includes('Local OpenCode database'));
assert.ok(summaryUI.element.textContent.includes('In OpenCode 2, answer questions in its terminal.'));
button(summaryUI, 'Session').listeners.click();
summaryUI.update(summaryView);
summaryUI.element.querySelector('[data-fid="filter:failed"]').listeners.click();
assert.equal(summaryUI.element.querySelectorAll('.roost-card').length, 0);
assert.equal(summaryUI.element.querySelector('.card-name').textContent, 'Alpha', 'A filter does not replace the selected owner');
await summaryUI.flush();
assert.equal(overviewWrites.at(-1).status, 'failed');
const restoredStatus = createFlock({ ...backend, loadPlace: async () => overviewWrites.at(-1) });
restoredStatus.update(summaryView); await tick();
assert.equal(restoredStatus.element.querySelector('[data-fid="filter:failed"]').attributes['aria-pressed'], 'true');
assert.equal(restoredStatus.element.querySelectorAll('.roost-card').length, 0);
const detailEvents = summaryUI.element.querySelector('.detail-tabs').listeners;
detailEvents.keydown({ key: 'End', preventDefault() {} });
assert.equal(selectedDetail(summaryUI), 'Zeca');
detailEvents.keydown({ key: 'Home', preventDefault() {} });
assert.equal(selectedDetail(summaryUI), 'Session');
summaryUI.element.querySelector('.flock-back').listeners.click();
assert.equal(summaryUI.element.classList.contains('choosing-session'), true);
summaryUI.element.querySelector('.flock-selected').scrollTop = 150;
summaryUI.update({ ...view, sessions: [{ ...reported, status: 'approval', attention: 'needs-you', card: true }] });
assert.equal(summaryUI.element.querySelector('.flock-selected').scrollTop, 0, 'A new permission brings its card into view, even after scrolling the overview');
summaryUI.element.querySelector('.flock-back').listeners.click();
summaryUI.update({ ...view, sessions: [{ ...reported, status: 'approval', attention: 'needs-you', card: true }] });
assert.equal(summaryUI.element.classList.contains('choosing-session'), true, 'An unchanged request does not interrupt explicit list browsing');
summaryUI.element.querySelector('.flock-back').listeners.click();
summaryUI.update({ ...view, approval: { ...view.approval, request: 'new-overview-permission' }, sessions: [{ ...reported, status: 'approval', attention: 'needs-you', card: true }] });
assert.equal(summaryUI.element.classList.contains('choosing-session'), false, 'An arriving request restores its selected card on narrow screens');
assert.ok(summaryUI.element.textContent.includes('Review this permission'));
assert.deepEqual(overviewActions, [], 'Metrics, filters, overview links and navigation never answer, focus or start a session');
console.log('check-flock-place: reported/unknown/zero metrics, selected ownership, diff-derived files, read-only overview, retained scroll/status, tab Home/End and narrow permission priority');

// Command/script edits are visible from the repository even when the agent sent no patch events.
const repoRequests = [];
const repoUI = createFlock({ ...backend, loadPlace: async () => ({ query: '', onlyNeeds: false, status: null, detail: 'diff' }),
  workspaceDiff: s => new Promise((resolve, reject) => repoRequests.push({ id: s.id, resolve, reject })),
});
const repoSession = { ...unknown, id: 'repo', agent: 'codex', changes: [], touched: [] };
const repoView = s => ({ ...summaryView, approval: null, sessions: [s], front: { agent: s.agent, id: s.id } });
repoUI.update(repoView(repoSession)); await tick();
assert.equal(repoRequests.length, 1);
assert.ok(repoUI.element.textContent.includes('Repository changes'));
repoRequests[0].resolve({ files: [{ path: 'script-edited.rs', added: 1, removed: 1, hunks: [{ old_start: 1, new_start: 1, lines: ['-before', '+after-script'] }] }], cut: false });
await tick();
assert.ok(repoUI.element.textContent.includes('+after-script'));
repoUI.element.querySelector('[data-fid="act:workspace-focus"]').listeners.click();
assert.equal(repoUI.element.querySelector('.app-body').classList.contains('details-focus'), true, 'Command-only diffs can expand');
repoUI.element.querySelector('[data-fid="act:workspace-focus"]').listeners.click();
assert.ok(repoUI.element.textContent.includes('including command edits and pre-existing changes'));
button(repoUI, 'Refresh changes').listeners.click(); await tick();
assert.equal(repoRequests.length, 2);
repoRequests[1].reject(Error('read failed')); await tick();
assert.ok(repoUI.element.textContent.includes('Could not read repository changes.'));
assert.ok(repoUI.element.textContent.includes('+after-script'), 'Failure keeps the last preview with its error');
button(repoUI, 'Refresh changes').listeners.click(); await tick();
repoUI.update(repoView({ ...repoSession, id: 'other-repo', cwd: '/other' })); await tick();
assert.equal(repoRequests.length, 4);
repoRequests[2].resolve({ files: [{ path: 'stale-owner.rs', added: 1, removed: 0, hunks: [{ old_start: 1, new_start: 1, lines: ['+stale-owner'] }] }], cut: false });
repoRequests[3].resolve({ files: [], cut: false }); await tick();
assert.ok(!repoUI.element.textContent.includes('+stale-owner'), 'An old result cannot overwrite the selected session');
assert.ok(repoUI.element.textContent.includes('No local text changes found.'));
repoUI.setActive(false);
console.log('check-flock-place: repository command edits, explicit provenance, refresh/failure retention and stale-owner fencing');

const hermesUI = createFlock({ ...backend, loadPlace: async () => null });
const hermesSession = { ...reported, agent: 'other', agent_name: 'hermes', id: 'hermes/sample',
  status: 'working', card: false, cost_usd: 0.0042, elapsed_ms: 1250, context_tokens: 420, context_percent: 42,
  metrics_age_secs: 1, metrics_checked_age_secs: 100 };
hermesUI.update(repoView(hermesSession)); await tick();
const hermesMetrics = hermesUI.element.querySelector('.session-metrics');
assert.ok(hermesMetrics.textContent.includes('Model response time'));
assert.ok(hermesMetrics.textContent.includes('≈$0.0042'), 'A nonzero sub-cent cost must not look like zero');
assert.ok(hermesMetrics.textContent.includes('42%'));
assert.ok(hermesUI.element.textContent.includes('Hermes Agent'));
assert.ok(!hermesUI.element.textContent.includes('Metrics checks are delayed.'), 'Hermes emits metrics on completed calls, not periodic status-line ticks');
button(hermesUI, 'Context').listeners.click();
assert.ok(hermesUI.element.textContent.includes('Hermes plugin and local cost database'));

const nativeMetricsUI = createFlock({ ...backend, loadPlace: async () => null });
const nativeMetrics = { ...hermesSession, metric_info: { model: 'unit', provider: 'native-provider',
  time_kind: 'model-response', cost_source: 'agent-estimate', input_tokens: 12000, output_tokens: 40,
  cached_tokens: 8000, cache_write_tokens: 10 }, cost_age_secs: 1, elapsed_age_secs: 30, context_age_secs: 50 };
nativeMetricsUI.update(repoView(nativeMetrics)); await tick();
assert.ok(nativeMetricsUI.element.textContent.includes('Session consumption'));
assert.ok(nativeMetricsUI.element.textContent.includes('Session input tokens'));
button(nativeMetricsUI, 'Context').listeners.click();
assert.ok(nativeMetricsUI.element.textContent.includes('Estimated by agent'));
assert.ok(nativeMetricsUI.element.textContent.includes('unit'));
assert.ok(nativeMetricsUI.element.textContent.includes('Cost last read1 s ago'));
assert.ok(nativeMetricsUI.element.textContent.includes('Context last read50 s ago'));
assert.ok(!nativeMetricsUI.element.textContent.includes('models.dev'), 'A native estimate must not be attributed to our catalog');
const tokenOnly = createFlock({ ...backend, loadPlace: async () => null });
tokenOnly.update(repoView({ ...nativeMetrics, id: 'droid/native', agent_name:'droid', elapsed_ms:null,
  context_tokens:null, context_percent:null, cost_usd:null, metric_info:{input_tokens:35, output_tokens:3, cached_tokens:20} })); await tick();
assert.ok(tokenOnly.element.textContent.includes('Session consumption'), 'Native consumption is visible even without cost/time/context');
assert.ok(!tokenOnly.element.textContent.includes('Metrics are not available for this agent yet.'));
console.log('check-flock-place: metric provenance, separate read ages and cumulative consumption without invented context');
