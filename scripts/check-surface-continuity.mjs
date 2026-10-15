// Exercise production Nest, Chat, island state and input regions together; no browser or desktop server.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";

const nodes = node => [node, ...node.children.flatMap(nodes)];
class Element {
  children = []; listeners = {}; attributes = {}; dataset = {}; style = { setProperty() {} };
  className = ""; value = ""; hidden = false; scrollTop = 0;
  constructor(tag) { this.tag = tag; this.tagName = tag.toUpperCase(); }
  classList = {
    contains: name => this.className.split(" ").includes(name),
    add: name => { if (!this.classList.contains(name)) this.className += ` ${name}`; },
    remove: name => { this.className = this.className.split(" ").filter(value => value !== name).join(" "); },
    toggle: (name, force = !this.classList.contains(name)) => { this.classList[force ? "add" : "remove"](name); return force; },
  };
  set textContent(text) { this.replaceChildren(); this.text = text; }
  get textContent() { return (this.text ?? "") + this.children.map(node => node.textContent).join(""); }
  append(...children) { for (const node of children) { node.remove(); node.parentElement = this; this.children.push(node); } }
  prepend(...children) { this.append(...children); this.children = [...children, ...this.children.filter(node => !children.includes(node))]; }
  replaceChildren(...children) { this.text = ""; for (const node of this.children) node.parentElement = null; this.children = []; this.append(...children); }
  remove() { if (this.parentElement) this.parentElement.children = this.parentElement.children.filter(node => node !== this); this.parentElement = null; }
  contains(node) { return nodes(this).includes(node); }
  get firstElementChild() { return this.children[0] ?? null; }
  get lastElementChild() { return this.children.at(-1) ?? null; }
  get isConnected() { return this === document.body || !!this.parentElement?.isConnected; }
  get offsetParent() { return this.isConnected && !this.hidden && this.parentElement?.offsetParent !== null ? this.parentElement ?? this : null; }
  addEventListener(event, fn) { (this.listeners[event] ??= []).push(fn); }
  fire(event, value = {}) { for (const fn of this.listeners[event] ?? []) fn({ target: this, ...value }); }
  setAttribute(name, value) { this.attributes[name] = value; }
  getAttribute(name) { return this.attributes[name]; }
  removeAttribute(name) { delete this.attributes[name]; }
  hasAttribute(name) { return name === "hidden" ? this.hidden : name in this.attributes; }
  toggleAttribute(name, force) { if (force) this.setAttribute(name, ""); else this.removeAttribute(name); }
  click() { if (!this.disabled) this.fire("click"); }
  matches(selector) {
    if (selector.includes(",")) return selector.split(",").some(value => this.matches(value.trim()));
    const attr = selector.match(/\[([^=\]]+)(?:="([^"]*)")?\]/);
    if (attr) { const value = attr[1].startsWith("data-") ? this.dataset[attr[1].slice(5)] : this.getAttribute(attr[1]); if (attr[2] !== undefined ? value !== attr[2] : value === undefined) return false; }
    const plain = selector.replace(/\[[^\]]*\]/g, "");
    if (!plain) return true;
    if (plain.startsWith(".")) return plain.slice(1).split(".").every(name => this.classList.contains(name));
    if (plain.startsWith("#")) return this.id === plain.slice(1);
    return this.tag === plain;
  }
  querySelectorAll(selector) { return nodes(this).slice(1).filter(node => selector.split(",").some(value => node.matches(value.trim()))); }
  querySelector(selector) { return this.querySelectorAll(selector)[0] ?? null; }
  closest(selector) { return this.matches(selector) ? this : this.parentElement?.closest(selector) ?? null; }
  focus() { document.activeElement = this; }
  setSelectionRange() {}
  scrollTo() {}
  scrollIntoView(options) { this.scrolledIntoView = options; }
  getBoundingClientRect() { return { left: 0, top: 0, width: 480, height: 200 }; }
  getContext() { return { clearRect() {}, fillRect() {} }; }
  attachShadow() { const shadow = new Element("shadow"); shadow.parentElement = this; return shadow; }
  toDataURL() { return "data:image/png;base64,"; }
}
const windowEvents = {}, documentEvents = {}, storage = new Map(), mediaChanges = [];
globalThis.document = { hidden: false, body: new Element("body"),
  createElement: tag => new Element(tag), createElementNS: (_ns, tag) => new Element(tag),
  createTextNode: text => { const node = new Element("text"); node.textContent = text; return node; },
  createDocumentFragment: () => new Element("fragment"),
  querySelector: selector => document.body.querySelector(selector),
  addEventListener: (event, fn) => (documentEvents[event] ??= []).push(fn),
};
globalThis.window = { innerWidth: 480, innerHeight: 640,
  setTimeout: () => 0, clearTimeout() {}, setInterval: () => 0, clearInterval() {},
  addEventListener: (event, fn) => (windowEvents[event] ??= []).push(fn),
  matchMedia: query => ({ get matches() {
    const rule = query.match(/(min|max)-(width|height):\s*(\d+)px/);
    if (!rule) return false;
    const value = rule[2] === "width" ? window.innerWidth : window.innerHeight;
    return rule[1] === "min" ? value >= Number(rule[3]) : value <= Number(rule[3]);
  }, addEventListener: (_event, fn) => mediaChanges.push({ query, fn }) }),
};
globalThis.matchMedia = window.matchMedia;
globalThis.localStorage = { getItem: key => storage.get(key) ?? null, setItem: (key, value) => storage.set(key, value) };
globalThis.requestAnimationFrame = () => 0;
globalThis.ResizeObserver = class { observe() {} };
globalThis.CSS = { escape: value => value };
globalThis.HTMLElement = globalThis.Node = Element;
globalThis.getComputedStyle = () => ({ lineHeight: "20", borderTopWidth: "1", borderBottomWidth: "1" });
const root = fileURLToPath(new URL("../ui/", import.meta.url));
const source = path => JSON.stringify(`${root}src/${path}`.replaceAll("\\", "/"));
const bundles = await build({ configFile: false, root, logLevel: "silent",
  plugins: [{ name: "continuity-entry", resolveId: id => id.endsWith("continuity-entry") ? "\0continuity-entry" : null,
    load: id => id === "\0continuity-entry" ? `
      export { createMemory } from ${source("surfaces/nest/memory.ts")};
      export { createConnections } from ${source("surfaces/nest/connections.ts")};
      export { createRecent } from ${source("surfaces/nest/recent.ts")};
      export { createNest } from ${source("surfaces/nest/render.ts")};
      export { ChatPanel } from ${source("island/chat.ts")};
      export { Scene } from ${source("island/scene.ts")};
      export { Clock } from ${source("clock.ts")};
      export { savedClose, savedTransition } from ${source("surfaces/nest/close.ts")};
      export * as fsm from ${source("island/fsm.ts")};
      export * as dock from ${source("island/dock.ts")};` : null }],
  build: { write: false, minify: false, lib: { entry: "continuity-entry", formats: ["es"] } },
});
const chunk = bundles[0].output.find(part => part.type === "chunk");
const { createMemory, createConnections, createRecent, createNest, ChatPanel, Scene, Clock, savedClose, savedTransition, fsm, dock } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
const scenes = new WeakMap(), active = new WeakMap();
const updateScene = Scene.prototype.update, activateScene = Scene.prototype.setActive;
Scene.prototype.update = function (...args) { scenes.set(this.canvas, this); return updateScene.apply(this, args); };
Scene.prototype.setActive = function (on) { active.set(this.canvas, on); return activateScene.call(this, on); };
const tick = () => new Promise(resolve => setImmediate(resolve));
const visibility = hidden => { document.hidden = hidden; for (const fn of documentEvents.visibilitychange ?? []) fn(); };
const row = { item: "org/alpha#7", group: "to-review", name: "org/alpha#7", title: "Review alpha", checks: null, review: null, link: true, merge: false,
  facts: { repo: "org/alpha", branch: "feature/alpha", diff: null, author: null, reviewers: [], updated: null, runs: [], release: null, command: null } };

for (const width of [480, 760, 900, 1180]) {
  window.innerWidth = width;
  for (const presence of ["island", "panel", "quiet"]) {
    fsm.setPresence(presence); fsm.setNestOpen(false); fsm.setNestHolds(false);
    const machine = new fsm.IslandMachine();
    const session = { id: "owned-run", agent: "codex", project: "Alpha", cwd: "/work/alpha", status: "approval", attention: "needs-you", card: true,
      activity: null, step: null, steps: [], step_count: 0, subagents: 0, note: null, editor: null, species: "black", bird_chosen: false };
    const view = { sessions: [session], alerts: [], focus: { agent: "codex", id: "owned-run" }, front: { agent: "codex", id: "owned-run" },
      approval: { agent: "codex", session: "owned-run", request: "pending-request", tool: "Bash", target: "git status", description: "Current permission", questions: [], queue: 1, always: false },
      boards: [{ connector: "github", rows: [row] }] };
    const originalView = structuredClone(view), actions = [], voiceCalls = [];
    let draft = { text: "Unsent", files: ["/inbox/note.txt"], folder: "/work/alpha", provider: "codex" };
    let place = "audit", flockPlace = { query: "Alpha", onlyNeeds: true, detail: "context", change: null };
    let connectionPlace = { chosen: "github", picked: row.item, repo: "org/alpha", tab: "prs", detailOpen: width < 900 };
    const queries = { history: "History query", audit: "Audit query", projects: "Project query" };
    const current = { provider: "codex", label: "Codex", folder: "/work/alpha", started: true, model: "gpt", effort: "high", lines: [], busy: true, partial: "", waiting: [] };
    let nest, page, quick, requested, saveFails = false, transitionErrors = 0;
    const chat = from => ({ current: async () => current, draft: async () => structuredClone(draft),
      saveDraft: async value => { if (saveFails) throw new Error("Draft save unavailable."); draft = structuredClone(value); (from === "nest" ? quick : nest?.chat)?.sync({ kind: "draft", from, draft }); },
      models: async () => [], pick: async () => ({}), conversations: async () => [], keyboard() {},
      voice: { start: async tap => { voiceCalls.push(["start", from, tap]); },
        stop: async () => { voiceCalls.push(["stop", from]); return ""; }, cancel: () => voiceCalls.push(["cancel", from]) },
      ...Object.fromEntries(["send", "reset", "open", "decide", "stop"].map(name => [name, () => { actions.push(`chat:${name}`); throw new Error(`Unexpected ${name}`); }])),
    });
    quick = new ChatPanel(chat("island"), () => {});
    const quickRoot = new Element("main"); quickRoot.append(quick.element); document.body.replaceChildren(quickRoot);
    await quick.restore();
    const backend = {
      chat: chat("nest"), savePlace: async value => { place = value; },
      flock: { loadPlace: async () => structuredClone(flockPlace), savePlace: async value => { flockPlace = structuredClone(value); },
        diff: async () => null, usage: async () => [], ...Object.fromEntries(["focus", "jump", "review", "decide", "decideAlways", "answer", "release", "hush", "openFolder", "projectPref"].map(name => [name, () => actions.push(name)])) },
      connections: { loadPlace: async () => structuredClone(connectionPlace), savePlace: async value => { connectionPlace = structuredClone(value); },
        status: async () => [], refresh: async () => {}, onMerged() {}, open: () => actions.push("connection:open"), merge: () => actions.push("merge"), settings() {} },
      records: { query: async (tab, value) => { if (value !== undefined) queries[tab] = value; return queries[tab]; }, history: async () => ({ on: true, turns: [] }), audit: async () => [], usage: async () => [], traces: async () => [] },
      roosts: { query: async value => { if (value !== undefined) queries.projects = value; return queries.projects; }, load: async () => [], week: async () => null },
      tasks: { load: async () => [], folders: () => [], agents: () => [], today: async () => null },
      memory: { load: async () => ({ on: false, facts: [], proposals: [] }) }, budget: { today: async () => null },
      week: { activity: async () => ({ history: false, today: "2026-10-14", weeks: ["2026-10-12"], week: { monday: "2026-10-12", sunday: "2026-10-18", turns: 0, headline: "No completed turns.", projects: [], days: [0, 0, 0, 0, 0, 0, 0] }, grid: [] }) },
      settings() {}, setZeca() {}, island: () => backend.size("quick"), sound() {}, media() {}, endDnd() {}, look() {},
      size: size => { requested = resize(async () => {
        if (size === "room" || size === "full") nest.setFull(size === "full");
        else { await close({ preventDefault() {} }); if (size === "quick") machine.open(0); else machine.fold(0); }
      }); },
    };
    const mount = async zeca => {
      page?.remove(); page = new Element("main"); document.body.append(page);
      nest = createNest(page, backend, zeca); nest.view(view); nest.open(place);
      await tick(); await nest.chat.restore();
    };
    let region;
    dock.routeLayout((...value) => { region = value; });
    const hiddenIsland = () => { fsm.setNestOpen(true); fsm.setNestHolds(true); machine.unpin(0); machine.away(0); dock.layout(110, 0, 500, 600, true); assert.deepEqual(region, [0, 0, 0, 0]); };
    const close = savedClose(() => nest.flushState(), async () => { page.remove(); fsm.setNestOpen(false); fsm.setNestHolds(false); }, () => { transitionErrors++; });
    const resize = savedTransition(() => nest.flushState(), () => { transitionErrors++; });
    await mount(true); hiddenIsland();
    assert.equal(nest.current(), "audit");
    const auditTab = nodes(page).find(node => node.dataset.sub === "audit");
    assert.equal(auditTab.attributes["aria-pressed"], "true");
    assert.deepEqual(auditTab.scrolledIntoView, { block: "nearest", inline: "nearest" }, "Restoring a sub-tab keeps its selection visible in the navigation strip");
    nest.open("chat"); nest.chat.draft("More context");
    const character = nest.chat.perchSlot.querySelector(".chat-character");
    const canvas = character.querySelector("canvas");
    const portrait = scenes.get(canvas);
    assert.ok(portrait, "Room uses the production animated scene in the chat's own perch");
    assert.equal(active.get(canvas), true);
    Clock.freeze(Clock.now() + 1000);
    assert.equal(portrait.playing("zeca"), "think", "A restored running conversation animates its own companion");
    character.fire("click");
    assert.equal(portrait.playing("zeca"), "think", "Greeting never replaces an active reply");
    nest.chat.load({ ...current, busy: false });
    Clock.freeze(Clock.now() + 1000);
    document.body.classList.add("still");
    character.fire("click");
    assert.equal(portrait.playing("zeca"), "idle", "Paused motion does not play a greeting");
    document.body.classList.remove("still");
    character.fire("click");
    assert.equal(portrait.playing("zeca"), "hello");
    nest.chat.receive({ kind: "permission", id: "portrait-card", tool: "Bash", target: "git status", description: null, full: null, added: 0, removed: 0, always: false, outside: null });
    character.fire("click");
    assert.equal(portrait.playing("zeca"), "question", "A waiting card outranks a cosmetic reaction");
    assert.deepEqual(actions, [], "The companion's reaction never sends, starts or answers anything");
    nest.chat.load(current);
    Clock.freeze(null);
    nest.chat.setVoiceReady(true);
    nest.open("flock"); nest.shortcut("talk"); await tick();
    assert.equal(nest.current(), "chat", "Room's talk shortcut opens its own chat");
    assert.deepEqual(voiceCalls.at(-1), ["start", "nest", false]);
    nest.shortcut("talk-up"); await tick();
    assert.deepEqual(voiceCalls.at(-1), ["stop", "nest"]);
    const mic = nest.chat.element.querySelector(".mic");
    const area = nest.chat.element.querySelector("textarea");
    mic.fire("click"); await tick();
    const speakingEscape = { key: "Escape", target: area, preventDefault() { this.defaultPrevented = true; } };
    for (const fn of windowEvents.keydown ?? []) fn(speakingEscape);
    assert.deepEqual(voiceCalls.at(-1), ["cancel", "nest"], "Escape cancels speech even with focus on the hidden input in Room");
    assert.equal(speakingEscape.defaultPrevented, true);
    assert.equal(nest.current(), "chat");
    for (const leave of [() => nest.open("flock"), () => visibility(true), () => nest.view({ ...view, locked: true }), () => nest.flushState()]) {
      mic.fire("click"); await tick();
      await leave();
      assert.deepEqual(voiceCalls.at(-1), ["cancel", "nest"], "Leaving, hiding, locking or saving the surface cancels recording");
      const calls = voiceCalls.length;
      nest.shortcut("talk-up"); await tick();
      assert.equal(voiceCalls.length, calls, "A late release never transcribes a cancelled recording");
      visibility(false); nest.view(view); nest.open("chat");
    }
    visibility(true); assert.equal(active.get(canvas), false);
    visibility(false); assert.equal(active.get(canvas), true);
    if (width < 600) {
      page.querySelector(".nest-memory-toggle").fire("click");
      assert.equal(active.get(canvas), false, "Memory hides the narrow conversation and stops its portrait");
      page.querySelector(".nest-memory-back").fire("click");
      assert.equal(active.get(canvas), true);
    }
    nest.view({ ...view, locked: true }); assert.equal(active.get(canvas), false);
    nest.view(view); assert.equal(active.get(canvas), true);
    window.innerHeight = 360;
    for (const media of mediaChanges.filter(media => media.query.includes("height"))) media.fn();
    assert.equal(active.get(canvas), false, "Short windows stop drawing the hidden portrait");
    window.innerHeight = 640;
    for (const media of mediaChanges.filter(media => media.query.includes("height"))) media.fn();
    assert.equal(active.get(canvas), true);
    const beforeHistory = structuredClone(draft);
    page.querySelector(".recent-toggle").fire("click");
    if (width < 900) {
      assert.equal(page.querySelector(".nest-chat").classList.contains("recent-open"), true);
      assert.equal(active.get(canvas), false, "Narrow history stops the hidden portrait");
      assert.equal(document.activeElement, page.querySelector(".nest-recent-search"));
      page.querySelector(".nest-recent-back").fire("click");
      assert.equal(page.querySelector(".nest-chat").classList.contains("recent-open"), false);
      assert.equal(active.get(canvas), true);
      assert.equal(document.activeElement, page.querySelector(".recent-toggle"));
    } else {
      assert.equal(page.querySelector(".nest-chat").classList.contains("no-recent"), true);
      page.querySelector(".recent-toggle").fire("click");
      assert.equal(page.querySelector(".nest-chat").classList.contains("no-recent"), false);
    }
    assert.deepEqual(draft, beforeHistory, "History navigation never rewrites the draft or files");
    const expectedDraft = { ...draft, text: "Unsent\n\nMore context" };
    for (const full of [true, false, true, false]) {
      page.querySelector(full ? ".nest-fullscreen" : '[data-size="room"]').fire("click"); await requested;
      assert.equal(page.classList.contains("full"), full); assert.deepEqual(draft, expectedDraft);
      assert.equal(nest.chat.perchSlot.querySelector("canvas"), canvas, "Room/Full keeps the same scene and animation phase");
    }
    page.querySelector(".nest-fullscreen").fire("click"); await requested;
    const escape = { key: "Escape", target: page, preventDefault() { this.defaultPrevented = true; } };
    for (const fn of windowEvents.keydown ?? []) fn(escape);
    await requested; assert.equal(page.classList.contains("full"), false); assert.equal(nest.current(), "chat");
    nest.open("connections"); await nest.flushState();
    assert.equal(active.get(canvas), false, "Leaving Chat stops its renderer");
    assert.deepEqual(connectionPlace, { chosen: "github", picked: row.item, repo: "org/alpha", tab: "prs", detailOpen: width < 900 });
    if (width >= 900) assert.equal(page.querySelector('[data-fid="row:org/alpha#7"]').getAttribute("aria-pressed"), "true");
    else assert.ok(page.textContent.includes("Review alpha"), "Narrow recreation restores the selected connection detail");
    const technical = page.querySelector(".conn-technical");
    assert.ok(technical, "Selected connections expose technical details separately");
    assert.ok(!technical.open);
    technical.open = true;
    const changedView = structuredClone(view);
    changedView.boards[0].rows[0].title = "Review alpha updated";
    nest.view(changedView);
    assert.equal(page.querySelector(".conn-technical").open, true, "Polling retains expanded technical context");
    nest.view(view);
    nest.open("audit");
    saveFails = true;
    page.querySelector('[data-size="quick"]').fire("click"); await requested;
    assert.equal(transitionErrors, 1); assert.equal(page.isConnected, true);
    assert.equal(fsm.nestIsOpen(), true); assert.deepEqual(region, [0, 0, 0, 0]);
    saveFails = false;
    page.querySelector('[data-size="quick"]').fire("click"); await requested;
    assert.equal(page.isConnected, false); assert.equal(machine.mode, "open");
    assert.deepEqual(draft, expectedDraft); assert.equal(place, "audit");
    await mount(true); hiddenIsland();
    assert.equal(nest.current(), "audit");
    assert.deepEqual(flockPlace, { query: "Alpha", onlyNeeds: true, detail: "context", change: null });
    assert.deepEqual(queries, { history: "History query", audit: "Audit query", projects: "Project query" });
    // Minimize, deliver a pending card to the island, then restore without answering it.
    visibility(true); fsm.setNestHolds(false); fsm.setNestOpen(false); machine.openPinned();
    dock.layout(110, 0, 500, 600, false); assert.deepEqual(region, [110, 0, 500, 600]);
    nest.open("flock"); nest.shortcut("allow"); assert.deepEqual(actions, []);
    visibility(false); hiddenIsland();
    page.querySelector('[data-size="mini"]').fire("click"); await requested;
    assert.equal(page.isConnected, false); assert.equal(fsm.nestIsOpen(), false);
    machine.openPinned(); machine.fold(0);
    assert.equal(machine.mode, "open", "A pending card stays reachable even after a Mini request");
    await mount(true); hiddenIsland();
    nest.setZeca(false); nest.open("chat"); assert.equal(nest.current(), "flock");
    assert.ok(page.textContent.includes("Current permission"));
    await nest.flushState(); await close({ preventDefault() {} }); await mount(false);
    assert.equal(nest.current(), "flock"); assert.deepEqual(draft, expectedDraft);
    nest.open("connections"); await nest.flushState();
    assert.equal(connectionPlace.picked, row.item); assert.equal(connectionPlace.repo, "org/alpha");
    assert.deepEqual(view, originalView, "Layouts retain the same selected session/run/request owner");
    assert.deepEqual(actions, [], "Transitions never send, start/stop a run, answer, merge or change selection");
    page.remove(); fsm.setNestOpen(false); fsm.setNestHolds(false); machine.unpin(0);
  }
}
console.log("check-surface-continuity: ok (480/760/900/1180, Island/Panel/Quiet, Room/Full/Quick/Mini, failed save/retry, saved recreation, minimize/restore, Zeca off, source/draft/files/filters/connection/owner, no actions)");

// A retained search field filters the whole local list, including entries past the first page.
const requests = [], opened = [];
let recentBusy = false;
const recent = createRecent({ list: () => new Promise((resolve, reject) => requests.push({ resolve, reject })),
  open: id => opened.push(id), fresh: () => opened.push("fresh"), busy: () => recentBusy });
document.body.replaceChildren(recent.element);
const now = new Date();
const yesterday = new Date(now); yesterday.setDate(now.getDate() - 1);
const rows = Array.from({ length: 35 }, (_, i) => ({ id: i, title: i === 34 ? "Old login failure" : `Conversation ${i}`, label: i === 34 ? "Codex" : "API", folder: i === 34 ? "/work/legacy" : "/work/current", updatedMs: i === 34 ? yesterday.getTime() : now.getTime() - i * 1000, current: i === 0 }));
recent.refresh();
assert.ok(recent.element.textContent.includes("Loading"));
requests.at(-1).resolve(rows); await tick();
assert.equal(recent.element.querySelectorAll(".nest-recent-row").length, 30);
assert.ok(recent.element.querySelector('[aria-current="true"]'));
const search = recent.element.querySelector("input"); search.value = "legacy"; search.focus(); search.fire("input");
assert.equal(recent.element.querySelectorAll(".nest-recent-row").length, 1);
assert.ok(recent.element.textContent.includes("Yesterday"));
assert.equal(document.activeElement, search);
recent.element.querySelector(".nest-recent-row").fire("click"); assert.deepEqual(opened, [34]);
search.value = "Codex"; search.fire("input"); assert.equal(recent.element.querySelectorAll(".nest-recent-row").length, 1);
search.value = "missing"; search.fire("input"); assert.ok(recent.element.textContent.includes("No conversations match"));
search.value = "legacy"; search.fire("input");
recent.refresh(); requests.at(-1).reject(new Error("Offline")); await tick();
assert.ok(recent.element.textContent.includes("The last loaded list may be out of date"));
assert.ok(recent.element.textContent.includes("Old login failure"));
assert.equal(search.value, "legacy"); assert.equal(document.activeElement, search);
const recreatedRecent = createRecent({ list: async () => rows, open() {}, fresh() {}, busy: () => false });
assert.equal(recreatedRecent.element.querySelector("input").value, "legacy", "Closing and recreating Nest keeps the local history search");
recent.element.querySelector(".nest-recent-status").querySelector("button").fire("click");
assert.equal(recent.element.querySelector(".nest-recent-status").querySelector("button").disabled, true);
requests.at(-1).resolve(rows); await tick();
assert.ok(!recent.element.textContent.includes("The last loaded list"));
recentBusy = true; recent.refresh(); requests.at(-1).resolve(rows); await tick();
recent.element.querySelector(".nest-recent-row").fire("click");
assert.deepEqual(opened, [34], "Even an old row cannot switch conversations during a turn");
recentBusy = false; search.value = ""; search.fire("input");
recent.element.querySelectorAll("button").find(n => n.textContent === "Show more").fire("click");
assert.equal(recent.element.querySelectorAll(".nest-recent-row").length, 35);
recent.refresh(); const olderRequest = requests.at(-1); recent.refresh();
requests.at(-1).resolve(rows); await tick(); olderRequest.reject(new Error("Obsolete")); await tick();
assert.ok(!recent.element.textContent.includes("The last loaded list"));
console.log("check-surface-continuity: Recent local search, date groups, pagination, busy guard, retained query/focus, stale list/retry and response ordering");

// Only the explicit second click merges; updates and retry never perform this action.
window.innerWidth = 480;
const polls = [], merges = [];
const connections = createConnections({ status: async () => [], refresh: () => new Promise((resolve, reject) => polls.push({ resolve, reject })),
  onMerged() {}, merge: async (...args) => { merges.push(args); }, settings() {}, open() {}, zeca: () => null });
const mergeRow = { ...row, group: "yours", checks: "passing", merge: true,
  facts: { ...row.facts, runs: [{ name: "CI", state: "passed" }] } };
connections.update({ boards: [{ connector: "github", rows: [mergeRow] }] });
document.body.replaceChildren(connections.element); await tick();
connections.element.querySelector(".conn-row").fire("click");
const at = id => connections.element.querySelector(`[data-fid="act:${id}"]`);
assert.ok(connections.element.querySelector(".conn-check-summary").textContent.includes("1 of 1 passed"));
assert.equal(connections.element.querySelector(".conn-technical").contains(at("merge")), false);
at("merge").fire("click"); assert.equal(merges.length, 0);
at("merge-cancel").fire("click"); assert.equal(merges.length, 0);
at("merge").fire("click"); const confirmation = at("merge-yes"); confirmation.fire("click"); confirmation.fire("click");
await tick(); assert.equal(merges.length, 1, "A stale confirmation cannot submit a duplicate merge");
at("refresh").fire("click"); at("refresh").fire("click"); assert.equal(polls.length, 1);
assert.equal(at("refresh").getAttribute("aria-busy"), "true");
polls[0].reject(new Error("Offline")); await tick();
assert.ok(connections.element.textContent.includes("Connections could not update"));
at("refresh").fire("click"); polls[1].resolve(); await tick();
assert.ok(!connections.element.textContent.includes("Connections could not update"));
assert.equal(merges.length, 1);
console.log("check-surface-continuity: Connections check/action hierarchy, explicit merge/cancel, duplicate guards, refresh error/retry");

const memoryReads = [], memoryWrites = [], decisions = [];
const memory = createMemory({ load: () => new Promise((resolve, reject) => memoryReads.push({ resolve, reject })),
  decide: (...args) => new Promise((resolve, reject) => { memoryWrites.push(args); decisions.push({ resolve, reject }); }),
  undo: async id => { memoryWrites.push(["undo", id]); } });
const memoryHost = new Element("aside"); memoryHost.append(memory.element); document.body.replaceChildren(memoryHost);
let memoryView = { on: true, files: [{ place: "user", path: "/memory/user.md", facts: ["Prefers short answers"], used: 90, limit: 100 }],
  pending: [{ id: 7, op: "replace", place: "user", folder: "/work/alpha", old: "Short answers", new: "Concise answers with evidence", reason: "Your preference", source: "tool" }],
  history: [{ id: "ours", message: "Remember preference", by: "you", seconds: 1 }, { id: "external", message: "Manual edit", by: "other", seconds: 2 }],
  accepted: 1, rejected: 0, night: { day: "2026-10-15", debrief: "Yesterday's review" } };
const memoryButton = id => memory.element.querySelector(`[data-fid="${id}"]`);
memory.refresh(); assert.ok(memory.element.textContent.includes("Loading"));
memoryReads.at(-1).resolve(memoryView); await tick();
assert.ok(memory.element.textContent.indexOf("Waiting for you") < memory.element.textContent.indexOf("Yesterday's review"));
assert.ok(memory.element.textContent.includes("Current memoryShort answers"));
assert.ok(memory.element.textContent.includes("Proposed memoryConcise answers"));
assert.equal(memory.element.querySelectorAll(".memory-commit").filter(n => n.querySelector("button")).length, 1, "External edits never acquire Undo");
const fileDisclosure = memory.element.querySelector(".memory-file"); fileDisclosure.open = false;
memory.element.querySelector(".memory-history").open = true;
memoryHost.scrollTop = 140;
memory.refresh(); memoryReads.at(-1).resolve(memoryView); await tick();
assert.equal(memory.element.querySelector(".memory-file").open, false);
assert.equal(memory.element.querySelector(".memory-history").open, true);
assert.equal(memoryHost.scrollTop, 140);
assert.deepEqual(memoryWrites, [], "Loading and disclosing memory never accepts or reverts a change");
memoryButton("accept:7").focus(); memoryButton("accept:7").fire("click"); document.activeElement = document.body; await tick();
assert.deepEqual(memoryWrites, [[7, true]]);
assert.equal(memoryButton("reject:7").disabled, true);
memoryButton("reject:7").fire("click"); await tick(); assert.equal(memoryWrites.length, 1);
memory.refresh(); memoryReads.at(-1).resolve(memoryView); await tick();
assert.equal(memoryButton("accept:7").disabled, true, "An external refresh cannot unlock a pending decision");
decisions[0].reject("Cannot save this memory."); await tick(); memoryReads.at(-1).resolve(memoryView); await tick();
assert.ok(memory.element.textContent.includes("Cannot save this memory."));
assert.equal(memoryButton("accept:7").disabled, false);
assert.equal(document.activeElement, memoryButton("accept:7"), "The pending action regains focus after a failed write");
memory.refresh(); memoryReads.at(-1).reject(new Error("Offline")); await tick();
assert.ok(memory.element.textContent.includes("The last loaded view may be out of date"));
assert.ok(memory.element.textContent.includes("Concise answers"));
assert.ok(memory.element.textContent.includes("Cannot save this memory."));
memoryButton("retry").fire("click"); assert.equal(memoryButton("retry").disabled, true);
memoryReads.at(-1).resolve(memoryView); await tick();
assert.ok(!memory.element.textContent.includes("The last loaded view"));
memoryButton("accept:7").focus(); memoryButton("accept:7").fire("click"); document.activeElement = document.body; await tick();
memoryView = { ...memoryView, pending: [] }; decisions[1].resolve(); await tick(); memoryReads.at(-1).resolve(memoryView); await tick();
assert.equal(memoryButton("accept:7"), null); assert.equal(document.activeElement.className, "memory-heading");
memory.refresh(); const obsolete = memoryReads.at(-1); memory.refresh();
memoryReads.at(-1).resolve(memoryView); await tick(); obsolete.reject(new Error("Obsolete")); await tick();
assert.ok(!memory.element.textContent.includes("The last loaded view"));
console.log("check-surface-continuity: Memory proposal/fact/history hierarchy, explicit decisions, pending guard across refresh, stale/error/retry, retained disclosures/scroll/focus and response ordering");
