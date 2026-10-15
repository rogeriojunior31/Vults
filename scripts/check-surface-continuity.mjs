// Exercise production Nest, Chat, island state and input regions together; no browser or desktop server.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";

const nodes = node => [node, ...node.children.flatMap(nodes)];
class Element {
  children = []; listeners = {}; attributes = {}; dataset = {}; style = { setProperty() {} };
  className = ""; value = ""; hidden = false; scrollTop = 0;
  constructor(tag) { this.tag = tag; }
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
  matches(selector) {
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
  getBoundingClientRect() { return { left: 0, top: 0, width: 480, height: 200 }; }
  getContext() { return { clearRect() {}, fillRect() {} }; }
  attachShadow() { const shadow = new Element("shadow"); shadow.parentElement = this; return shadow; }
  toDataURL() { return "data:image/png;base64,"; }
}
const windowEvents = {}, documentEvents = {}, storage = new Map();
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
  matchMedia: query => ({ matches: query.includes("min-width") ? window.innerWidth >= 900 : window.innerWidth <= 600, addEventListener() {} }),
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
      export { createNest } from ${source("surfaces/nest/render.ts")};
      export { ChatPanel } from ${source("island/chat.ts")};
      export { savedClose, savedTransition } from ${source("surfaces/nest/close.ts")};
      export * as fsm from ${source("island/fsm.ts")};
      export * as dock from ${source("island/dock.ts")};` : null }],
  build: { write: false, minify: false, lib: { entry: "continuity-entry", formats: ["es"] } },
});
const chunk = bundles[0].output.find(part => part.type === "chunk");
const { createNest, ChatPanel, savedClose, savedTransition, fsm, dock } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
const tick = () => new Promise(resolve => setImmediate(resolve));
const visibility = hidden => { document.hidden = hidden; for (const fn of documentEvents.visibilitychange ?? []) fn(); };
const row = { item: "org/alpha#7", group: "to-review", name: "org/alpha#7", title: "Review alpha", checks: null, review: null, link: true, merge: false,
  facts: { repo: "org/alpha", branch: "feature/alpha", diff: null, author: null, reviewers: [], updated: null, runs: [], release: null, command: null } };

for (const width of [480, 1180]) {
  window.innerWidth = width;
  for (const presence of ["island", "panel", "quiet"]) {
    fsm.setPresence(presence); fsm.setNestOpen(false); fsm.setNestHolds(false);
    const machine = new fsm.IslandMachine();
    const session = { id: "owned-run", agent: "codex", project: "Alpha", cwd: "/work/alpha", status: "approval", attention: "needs-you", card: true,
      activity: null, step: null, steps: [], step_count: 0, subagents: 0, note: null, editor: null, species: "black", bird_chosen: false };
    const view = { sessions: [session], alerts: [], focus: { agent: "codex", id: "owned-run" }, front: { agent: "codex", id: "owned-run" },
      approval: { agent: "codex", session: "owned-run", request: "pending-request", tool: "Bash", target: "git status", description: "Current permission", questions: [], queue: 1, always: false },
      boards: [{ connector: "github", rows: [row] }] };
    const originalView = structuredClone(view), actions = [];
    let draft = { text: "Unsent", files: ["/inbox/note.txt"], folder: "/work/alpha", provider: "codex" };
    let place = "audit", flockPlace = { query: "Alpha", onlyNeeds: true, detail: "context", change: null };
    let connectionPlace = { chosen: "github", picked: row.item, repo: "org/alpha", tab: "prs", detailOpen: width < 900 };
    const queries = { history: "History query", audit: "Audit query", projects: "Project query" };
    const current = { provider: "codex", label: "Codex", folder: "/work/alpha", started: true, model: "gpt", effort: "high", lines: [], busy: true, partial: "", waiting: [] };
    let nest, page, quick, requested, saveFails = false, transitionErrors = 0;
    const chat = from => ({ current: async () => current, draft: async () => structuredClone(draft),
      saveDraft: async value => { if (saveFails) throw new Error("Draft save unavailable."); draft = structuredClone(value); (from === "nest" ? quick : nest?.chat)?.sync({ kind: "draft", from, draft }); },
      models: async () => [], pick: async () => ({}), conversations: async () => [], keyboard() {},
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
    nest.open("chat"); nest.chat.draft("More context");
    const expectedDraft = { ...draft, text: "Unsent\n\nMore context" };
    for (const full of [true, false, true, false]) {
      page.querySelector(full ? ".nest-fullscreen" : '[data-size="room"]').fire("click"); await requested;
      assert.equal(page.classList.contains("full"), full); assert.deepEqual(draft, expectedDraft);
    }
    page.querySelector(".nest-fullscreen").fire("click"); await requested;
    const escape = { key: "Escape", target: page, preventDefault() { this.defaultPrevented = true; } };
    for (const fn of windowEvents.keydown ?? []) fn(escape);
    await requested; assert.equal(page.classList.contains("full"), false); assert.equal(nest.current(), "chat");
    nest.open("connections"); await nest.flushState();
    assert.deepEqual(connectionPlace, { chosen: "github", picked: row.item, repo: "org/alpha", tab: "prs", detailOpen: width < 900 });
    if (width >= 900) assert.equal(page.querySelector('[data-fid="row:org/alpha#7"]').getAttribute("aria-pressed"), "true");
    else assert.ok(page.textContent.includes("Review alpha"), "Narrow recreation restores the selected connection detail");
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
console.log("check-surface-continuity: ok (480/1180, Island/Panel/Quiet, Room/Full/Quick/Mini, failed save/retry, saved recreation, minimize/restore, Zeca off, source/draft/files/filters/connection/owner, no actions)");
