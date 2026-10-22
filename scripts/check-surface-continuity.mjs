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
  after(...children) {
    const parent = this.parentElement;
    if (!parent) return;
    for (const node of children) node.remove();
    parent.children.splice(parent.children.indexOf(this) + 1, 0, ...children);
    for (const node of children) node.parentElement = parent;
  }
  prepend(...children) { this.append(...children); this.children = [...children, ...this.children.filter(node => !children.includes(node))]; }
  replaceChildren(...children) { this.text = ""; for (const node of this.children) node.parentElement = null; this.children = []; this.append(...children); }
  remove() { if (this.parentElement) this.parentElement.children = this.parentElement.children.filter(node => node !== this); this.parentElement = null; }
  contains(node) { return nodes(this).includes(node); }
  get options() { return this.children.filter(node => node.tagName === "OPTION"); }
  get firstElementChild() { return this.children[0] ?? null; }
  get lastElementChild() { return this.children.at(-1) ?? null; }
  get isConnected() { return this === document.body || !!this.parentElement?.isConnected; }
  get offsetParent() { return this.isConnected && !this.hidden && this.parentElement?.offsetParent !== null ? this.parentElement ?? this : null; }
  removeEventListener(event, fn) { this.listeners[event] = (this.listeners[event] ?? []).filter(f => f !== fn); }
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
  attachShadow() { const shadow = new Element("shadow"); shadow.parentElement = this; this.shadowRoot = shadow; return shadow; }
  toDataURL() { return "data:image/png;base64,"; }
}
const windowEvents = {}, documentEvents = {}, storage = new Map(), mediaChanges = [];
globalThis.document = { hidden: false, body: new Element("body"),
  createElement: tag => new Element(tag), createElementNS: (_ns, tag) => new Element(tag),
  createTextNode: text => { const node = new Element("text"); node.textContent = text; return node; },
  createDocumentFragment: () => new Element("fragment"),
  querySelector: selector => document.body.querySelector(selector),
  addEventListener: (event, fn) => (documentEvents[event] ??= []).push(fn),
  dispatchEvent: event => { for (const fn of documentEvents[event.type] ?? []) fn(event); },
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
globalThis.devicePixelRatio = 1;
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
      export { createMeetings } from ${source("surfaces/nest/dictation/meetings.ts")};
      export { createMemory } from ${source("surfaces/nest/memory.ts")};
      export { createConnections } from ${source("surfaces/nest/connections.ts")};
      export { createRecent } from ${source("surfaces/nest/recent.ts")};
      export { setDayPart } from ${source("island/daypart.ts")};
      export { setMusic, emoteFor } from ${source("island/behavior.ts")};
      export { badgeOf } from ${source("island/views.ts")};
      export { createNow } from ${source("island/now.ts")};
      export { createProfiles } from ${source("surfaces/nest/profiles.ts")};
      export { settingsQuick } from ${source("island/quick.ts")};
      export { focusCard, questionBody } from ${source("island/views.ts")};
      export { applyTheme } from ${source("theme.ts")};
      export { sidePanel } from ${source("surfaces/nest/panels.ts")};
      export { createWeek } from ${source("surfaces/nest/week.ts")};
      export { paletteItems } from ${source("palette.ts")};
      export { NEST_TABS, tabByKey } from ${source("surfaces/nest/head.ts")};
      export { createNest } from ${source("surfaces/nest/render.ts")};
      export { ChatPanel } from ${source("island/chat.ts")};
      export { Sky } from ${source("island/sky.ts")};
      export { Scene } from ${source("island/scene.ts")};
      export { Clock } from ${source("clock.ts")};
      export { savedClose, savedTransition } from ${source("surfaces/nest/close.ts")};
      export * as fsm from ${source("island/fsm.ts")};
      export * as dock from ${source("island/dock.ts")};` : null }],
  build: { write: false, minify: false, lib: { entry: "continuity-entry", formats: ["es"] } },
});
const chunk = bundles[0].output.find(part => part.type === "chunk");
const { createProfiles, settingsQuick, createMeetings, setDayPart, setMusic, emoteFor, badgeOf, createNow, focusCard, questionBody, applyTheme, sidePanel, paletteItems, NEST_TABS, tabByKey, createWeek, createMemory, createConnections, createRecent, createNest, ChatPanel, Scene, Sky, Clock, savedClose, savedTransition, fsm, dock } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
const scenes = new WeakMap(), active = new WeakMap();
const updateScene = Scene.prototype.update, activateScene = Scene.prototype.setActive;
Scene.prototype.update = function (...args) { scenes.set(this.canvas, this); return updateScene.apply(this, args); };
Scene.prototype.setActive = function (on) { active.set(this.canvas, on); return activateScene.call(this, on); };
const tick = () => new Promise(resolve => setImmediate(resolve));
const visibility = hidden => { document.hidden = hidden; for (const fn of documentEvents.visibilitychange ?? []) fn(); };
const row = { item: "org/alpha#7", group: "to-review", name: "org/alpha#7", title: "Review alpha", checks: null, review: null, link: true, merge: false,
  facts: { repo: "org/alpha", branch: "feature/alpha", diff: null, author: null, reviewers: [], updated: 1_770_000_000, runs: [], release: null, command: null } };

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
    const originalView = structuredClone(view), actions = [], voiceCalls = [], preparations = [];
    let draft = { text: "Unsent", files: ["/inbox/note.txt"], folder: "/work/alpha", provider: "codex" };
    let place = "audit", flockPlace = { query: "Alpha", onlyNeeds: true, status: null, detail: "context", change: null };
    let connectionPlace = { chosen: "github", picked: row.item, repo: "org/alpha", tab: "prs", detailOpen: width < 900 };
    const queries = { history: "History query", audit: "Audit query", projects: "Project query" };
    const current = { provider: "codex", label: "Codex", folder: "/work/alpha", started: true, model: "gpt", effort: "high", lines: [], busy: true, partial: "", waiting: [] };
    let nest, page, quick, requested, saveFails = false, transitionErrors = 0;
    const chat = from => ({ current: async () => current, draft: async () => structuredClone(draft),
      prepareZeca: async generation => { preparations.push([from, generation]); },
      saveDraft: async value => { if (saveFails) throw new Error("Draft save unavailable."); draft = structuredClone(value); (from === "nest" ? quick : nest?.chat)?.sync({ kind: "draft", from, draft }); },
      models: async () => [], pick: async () => ({}), conversations: async () => [], keyboard() {},
      voice: { start: async tap => { voiceCalls.push(["start", from, tap]); },
        stop: async () => { voiceCalls.push(["stop", from]); return ""; }, cancel: () => voiceCalls.push(["cancel", from]) },
      ...Object.fromEntries(["send", "reset", "open", "decide", "stop"].map(name => [name, () => { actions.push(`chat:${name}`); throw new Error(`Unexpected ${name}`); }])),
    });
    quick = new ChatPanel(chat("island"), () => {});
    const quickRoot = new Element("main"); quickRoot.append(quick.element); document.body.replaceChildren(quickRoot);
    await quick.restore();
    let readSpend = async () => ({ usd: 0, tokens: 0, budget: null, subscription: false });
    const settingsRoutes = [];
    let dictationReturns = 0;
    let dictationReads = 0;
    let realtimePhase = "recording", realtimeStarts = 0;
    const realtimeControls = [];
    const backend = {
      dictation: { conversation: {
        start: async () => { realtimeStarts++; return { id: "voice-fixture", phase: realtimePhase, response: "reply-token", capture: "capture-token", reason: null, draft: null, reply: null }; },
        status: async () => { throw Error("Unexpected heartbeat in synchronous scene checks"); },
        control: async (...args) => { realtimeControls.push(args); }, copy: async () => {},
      }, statistics: async period => ({ period, usage: { count: 0, words: 0, secs: 0, saved_minutes: null, wpm: null }, days: [], apps: [], apps_complete: true }), words: async () => ({ words: [], replacements: [], snippets: [] }), change: async () => {}, trySentence: async () => ({ text: "", cancelled: false, segments: [] }), load: async () => { dictationReads++; return { retention: "30", rows: [], today: null }; },
        remove: async () => actions.push("dictation:remove"), clear: async () => actions.push("dictation:clear"), again: async () => actions.push("dictation:again") },
      chat: chat("nest"), savePlace: async value => { place = value; },
      flock: { loadPlace: async () => structuredClone(flockPlace), savePlace: async value => { flockPlace = structuredClone(value); },
        diff: async () => null, usage: async () => [], ...Object.fromEntries(["focus", "jump", "review", "decide", "decideAlways", "answer", "release", "hush", "openFolder", "projectPref"].map(name => [name, () => actions.push(name)])) },
      connections: { loadPlace: async () => structuredClone(connectionPlace), savePlace: async value => { connectionPlace = structuredClone(value); },
        status: async () => [], refresh: async () => {}, onMerged() {}, open: () => actions.push("connection:open"), merge: () => actions.push("merge"), settings() {} },
      records: { query: async (tab, value) => { if (value !== undefined) queries[tab] = value; return queries[tab]; }, history: async () => ({ on: true, turns: [] }), audit: async () => [], usage: async () => ({ windows: [], days: [], roosts: [], budget: null, subscription: false }), traces: async () => [] },
      roosts: { query: async value => { if (value !== undefined) queries.projects = value; return queries.projects; }, load: async () => [], week: async () => null },
      memory: { load: async () => ({ on: false, facts: [], proposals: [] }) }, spend: () => readSpend(),
      week: { activity: async () => ({ history: false, today: "2026-10-14", weeks: ["2026-10-12"], week: { monday: "2026-10-12", sunday: "2026-10-18", turns: 0, headline: "No completed turns.", projects: [], days: [0, 0, 0, 0, 0, 0, 0] }, grid: [] }) },
      openDictation: () => { dictationReturns++; },
      settings: section => settingsRoutes.push(section), setZeca() {}, island: () => backend.size("quick"), sound() {}, media() {}, endDnd() {}, look() {},
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
    if (width === 480 && presence === "island") {
      const reads = [];
      const originalSpend = readSpend;
      readSpend = () => new Promise((resolve, reject) => reads.push({ resolve, reject }));
      nest.open("cost");
      const today = page.querySelector(".cost-today");
      assert.ok(today.textContent.includes("Loading"));
      assert.ok(!today.textContent.includes("$0.00"), "An unreadable spend is not zero");
      reads[0].reject("Cannot read spend."); await tick();
      assert.ok(today.textContent.includes("Cannot read spend."));
      nodes(today).find(n => n.tag === "button" && n.textContent === "Try again").click();
      reads[1].resolve({ usd: 1, tokens: 100, budget: 2, subscription: true }); await tick();
      assert.ok(today.textContent.includes("$1.00 of your $2.00"));
      assert.equal(today.querySelector("input"), null, "Cost never edits the budget");
      const originalSettings = backend.settings;
      backend.settings = section => actions.push(["settings", section]);
      nodes(page).find(n => n.tag === "button" && n.textContent === "Budget settings").click();
      assert.deepEqual(actions.pop(), ["settings", "activity"]);
      backend.settings = originalSettings;
      nest.spendChanged(); nest.spendChanged();
      reads[3].reject("Latest spend read failed."); await tick();
      reads[2].resolve({ usd: 99, tokens: 100, budget: 2, subscription: true }); await tick();
      assert.ok(today.textContent.includes("$1.00 of your $2.00"));
      assert.ok(today.textContent.includes("Latest spend read failed."));
      assert.ok(!today.textContent.includes("$99"), "An obsolete spend cannot replace the current result");
      readSpend = originalSpend;
      nest.open("audit");
    }
    nest.open("chat"); nest.chat.draft("More context");
    const character = nest.chat.perchSlot.querySelector(".chat-character");
    const canvas = character.querySelector("canvas");
    assert.equal(canvas.isConnected, true, "The chat's companion remains mounted outside the scrolling messages");
    const portrait = scenes.get(canvas);
    assert.ok(portrait, "Room uses the production animated scene in the chat's own perch");
    assert.equal(active.get(canvas), true);
    Clock.freeze(Clock.now() + 1000);
    assert.equal(portrait.playing("zeca"), "think", "A restored running conversation animates its own companion");
    character.fire("click");
    assert.equal(portrait.playing("zeca"), "think", "Greeting never replaces an active reply");
    nest.chat.load({ ...current, busy: false });
    assert.equal(canvas.isConnected, true, "Replacing restored messages keeps the same companion attached");
    Clock.freeze(Clock.now() + 1000);
    document.body.classList.add("still");
    character.fire("click");
    assert.equal(portrait.playing("zeca"), "idle", "Paused motion does not play a greeting");
    document.body.classList.remove("still");
    character.fire("click");
    assert.equal(portrait.playing("zeca"), "hello");
    nest.chat.element.querySelector("textarea").fire("input");
    assert.equal(portrait.playing("zeca"), "listen", "Typing cuts a cosmetic greeting at once");
    nest.chat.receive({ kind: "permission", id: "portrait-card", tool: "Bash", target: "git status", description: null, full: null, added: 0, removed: 0, always: false, outside: null });
    character.fire("click");
    assert.equal(portrait.playing("zeca"), "question", "A waiting card outranks a cosmetic reaction");
    assert.deepEqual(actions, [], "The companion's reaction never sends, starts or answers anything");
    nest.chat.load(current);
    Clock.freeze(null);
    nest.chat.setVoiceReady(true);
    nest.open("flock");
    await nest.chat.flushDraft();
    const questionSnapshot = structuredClone(draft);
    const questionDraft = nest.chat.element.querySelector("textarea").value;
    const ask = page.querySelector('[data-fid="act:ask"]');
    assert.ok(ask, "The selected session retains its Ask Zeca action");
    ask.click(); await tick(); await tick();
    assert.equal(nest.current(), "chat");
    assert.deepEqual(preparations.at(-1), ["nest", undefined]);
    assert.ok(nest.chat.element.querySelector("textarea").value.startsWith(questionDraft));
    assert.ok(nest.chat.element.querySelector("textarea").value.includes("What is Alpha doing now?"));
    assert.deepEqual(actions, [], "A session question never resets, sends or answers its permission");
    nest.chat.restoreDraft(questionSnapshot); await nest.chat.flushDraft();
    nest.open("flock");
    const beforeDictation = voiceCalls.length;
    nest.shortcut("talk"); nest.shortcut("talk-up"); await tick();
    assert.equal(nest.current(), "flock", "The dictation shortcut never opens Chat");
    assert.equal(voiceCalls.length, beforeDictation, "Dictation events never start Chat voice");
    nest.open("chat");
    nest.shortcut("talk"); nest.shortcut("talk-up"); await tick();
    assert.equal(voiceCalls.length, beforeDictation, "A focused Chat cannot steal dictation");
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
    const startRealtime = async phase => {
      realtimePhase = phase; nest.open("dictation-conversation");
      const work = page.querySelector(".voice-conversation");
      await nodes(work).find(n => n.tag === "button" && n.textContent === "Start recording").onclick();
      return work;
    };
    const stopRealtime = work => nodes(work).find(n => n.tag === "button" && ["Cancel", "End conversation"].includes(n.textContent)).onclick();
    for (const [phase, clip] of [["preparing", "think"], ["recording", "listen"], ["responding", "think"], ["waiting", "idle"], ["speaking", "speak"]]) {
      const work = await startRealtime(phase);
      assert.equal(work.querySelector("canvas"), canvas, "Voice reuses the Chat canvas instead of creating a second character");
      assert.equal(portrait.playing("zeca"), clip, phase);
      assert.equal(active.get(canvas), true);
      const starts = realtimeStarts, since = portrait.zeca.bird.since;
      Clock.freeze(Clock.now() + 123);
      nest.setFull(true); nest.setFull(false);
      assert.equal(portrait.zeca.bird.since, since, "Redraws preserve the current sprite phase");
      Clock.freeze(null);
      assert.equal(work.querySelector("canvas"), canvas);
      assert.equal(realtimeStarts, starts, "Layout changes never start another capture");
      if (phase === "speaking") {
        const now = Clock.now(); Clock.freeze(now);
        await nodes(work).find(n => n.tag === "button" && n.textContent === "Stop response").onclick();
        assert.equal(portrait.playing("zeca"), "idle", "Interruption cuts speaking immediately without waiting for the cosmetic minimum");
        assert.deepEqual(realtimeControls.at(-1), ["voice-fixture", "interrupt", "reply-token", undefined]);
        Clock.freeze(null);
      }
      stopRealtime(work);
      assert.equal(active.get(canvas), false);
      assert.equal(nest.chat.perchSlot.querySelector("canvas"), canvas);
    }
    let voiceWork = await startRealtime("speaking");
    nest.setZeca(false);
    assert.equal(active.get(canvas), false); assert.equal(voiceWork.querySelector(".voice-conversation-character").hidden, true);
    nest.setZeca(true); assert.equal(active.get(canvas), true);
    window.innerHeight = 360;
    for (const media of mediaChanges.filter(media => media.query.includes("height"))) media.fn();
    assert.equal(active.get(canvas), false);
    window.innerHeight = 640;
    for (const media of mediaChanges.filter(media => media.query.includes("height"))) media.fn();
    assert.equal(active.get(canvas), true);
    visibility(true); assert.equal(active.get(canvas), false);
    assert.equal(realtimeControls.at(-1)[1], "cancel", "Hiding revokes the conversation and its animated feedback");
    visibility(false); assert.equal(active.get(canvas), false, "Showing again cannot replay stale speaking");
    voiceWork = await startRealtime("recording");
    nest.view({ ...view, locked: true }); assert.equal(active.get(canvas), false);
    nest.view(view); assert.equal(active.get(canvas), false, "Unlocking cannot replay cancelled recording");
    voiceWork = await startRealtime("speaking");
    nest.open("chat");
    assert.equal(nest.chat.perchSlot.querySelector("canvas"), canvas);
    assert.equal(voiceWork.querySelector(".voice-conversation-character").hidden, true);
    assert.equal(realtimeControls.at(-1)[1], "cancel");
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
    page.querySelectorAll("button").find(b=>b.dataset.tab==="dictation").click();
    assert.equal(dictationReturns,1,"The header returns through the native Dictation owner rather than opening a hidden work page");
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
    assert.deepEqual(flockPlace, { query: "Alpha", onlyNeeds: true, status: null, detail: "context", change: null });
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
    const readsBefore = dictationReads;
    for (const tab of ["flock", "chat", "activity", "connections", "dictation", "chat"]) {
      nest.open(tab); await tick();
      if (tab === "dictation") {
        page.querySelector(".dictation-history").querySelectorAll("button").find(button => button.textContent === "Dictation settings").fire("click");
        assert.deepEqual(settingsRoutes, ["dictation"], "The work area links to the central Dictation settings owner");
      }
      const panes = page.children.filter(node => node.getAttribute("role") === "tabpanel");
      assert.equal(panes.length, 5);
      assert.deepEqual(panes.filter(node => !node.hidden).map(node => node.id), [`nest-pane-${tab === "chat" ? "flock" : tab}`],
        "Only the selected pane is shown, with Zeca-off Chat returning to Flock");
    }
    assert.equal(dictationReads, readsBefore + 1, "History loads when its own tab opens, not while Chat is selected");
    assert.ok(!page.querySelector(".dictation-history").textContent.includes("Reading dictation history"), "Opening Dictation settles its loading state");
    nest.open("dictation"); await tick();
    page.querySelector(".dictation-groups").querySelectorAll("button").find(button => button.textContent === "Styles").click(); await tick();
    assert.equal(settingsRoutes.at(-1), "dictationstyles", "Styles opens the single privileged editor inside the Dictation workspace");
    assert.equal(page.querySelector(".dictation-styles"),null,"No second read-only settings page remains");
    nest.open("dictation-dictionary");await tick();
    assert.equal(page.querySelector(".your-words").hidden,false,"Settings can return to an exact Dictation work page");
    page.querySelector(".dictation-groups").querySelectorAll("button").find(button => button.textContent === "History").click(); await tick();
    const commandNav = () => page.querySelector(".dictation-groups").querySelectorAll("button").find(button => button.textContent === "Voice commands");
    assert.ok(!commandNav().disabled);
    commandNav().click(); await tick();
    const workPage = () => page.querySelectorAll(".dictation-history").find(element => !element.hidden);
    assert.equal(workPage().querySelector("h2").textContent, "Voice commands");
    workPage().querySelectorAll("button").find(button=>button.textContent==="Command settings").click();
    assert.equal(settingsRoutes.at(-1),"dictationcommands","Command settings opens its own configuration directly");
    const commandSearch = workPage().querySelector("input");
    commandSearch.value = "Keep this command search"; commandSearch.fire("input");
    workPage().querySelector("details").open = true;
    page.querySelector(".dictation-groups").querySelectorAll("button").find(button => button.textContent === "Dictionary").click();
    commandNav().click(); await tick();
    nest.open("chat"); nest.open("dictation"); await tick();
    assert.equal(workPage().querySelector("h2").textContent, "Voice commands", "Tab handoffs retain the command work page");
    assert.equal(workPage().querySelector("input"), commandSearch);
    assert.equal(commandSearch.value, "Keep this command search");
    assert.equal(workPage().querySelector("details").open, true);
    assert.ok(!actions.includes("dictation:again") && !actions.includes("dictation:clear"));
    page.querySelector(".dictation-groups").querySelectorAll("button").find(button => button.textContent === "Statistics").fire("click"); await tick();
    assert.equal(nest.current(), "dictation-statistics", "The area links to Activity's single statistics owner");
    await nest.flushState();
    assert.equal(place, "dictation-statistics", "Statistics navigation survives the Room/Full handoff");
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

// Connections searches the loaded snapshot and retains both reading panes through a poll.
storage.delete("vults.connections-query");
window.innerWidth = 1180;
const connectionReads = [], connectionEffects = [];
const connectionUI = createConnections({ status: () => new Promise((resolve, reject) => connectionReads.push({ resolve, reject })),
  refresh: async () => { connectionEffects.push("refresh"); }, onMerged() {}, merge: async () => { connectionEffects.push("merge"); },
  open: () => connectionEffects.push("open"), settings: () => connectionEffects.push("settings"), zeca: () => null });
const second = { ...row, item: "org/beta#9", name: "org/beta#9", title: "An old useful result", facts: { ...row.facts, repo: "org/beta", branch: "fix/needle", author: "river" } };
let connectionView = { boards: [{ connector: "github", rows: [mergeRow, second] }] };
const healthy = [{ id: "github", enabled: true, lastOk: Date.now() / 1000, error: null, watching: 2, overBudget: false }];
const cHost = new Element("main"); cHost.append(connectionUI.element); document.body.replaceChildren(cHost);
connectionUI.update(connectionView); connectionReads.at(-1).resolve(healthy); await tick();
const cAt = id => connectionUI.element.querySelector(`[data-fid="${id}"]`);
assert.equal(connectionUI.element.querySelectorAll(".conn-chip").length, 1, "The connector is named once below the Connections page heading");
assert.equal(connectionUI.element.querySelectorAll('[data-fid="act:manage"]').length, 1);
const cSearch = connectionUI.element.querySelector(".conn-search");
assert.equal(cSearch.getAttribute("aria-label"), "Search connections");
cSearch.focus(); cSearch.value = "needle"; cSearch.fire("input");
assert.equal(connectionUI.element.querySelectorAll(".conn-row").length, 1);
assert.equal(connectionUI.element.querySelector(".conn-row").getAttribute("aria-pressed"), "true");
assert.equal(document.activeElement, cSearch);
assert.ok(connectionUI.element.textContent.includes("Results: 1 of 2"));
cSearch.value = "@missing"; cSearch.fire("input");
assert.ok(connectionUI.element.textContent.includes("No items match your search"));
assert.equal(connectionUI.element.querySelector(".conn-detail"), null);
connectionUI.element.querySelectorAll("button").find(b => b.textContent === "Clear filters").fire("click");
assert.equal(cSearch.value, ""); assert.equal(document.activeElement, cSearch);
cAt("act:repo").value = "org/beta"; cAt("act:repo").fire("change");
assert.equal(connectionUI.element.querySelectorAll(".conn-row").length, 1);
cAt("act:repo").value = ""; cAt("act:repo").fire("change");
cAt("row:org/alpha#7").fire("click");
assert.equal(connectionUI.element.querySelector(".conn-check-list").open, true, "Checks are visible on the initial triage view");
const detailBefore = connectionUI.element.querySelector(".conn-detail");
assert.ok(nodes(detailBefore).indexOf(cAt("act:merge")) < nodes(detailBefore).indexOf(cAt("act:checks")), "Actions precede an arbitrarily long check list");
connectionUI.element.querySelector(".conn-list").scrollTop = 120;
detailBefore.scrollTop = 75; cAt("act:technical").parentElement.open = true;
connectionUI.redraw();
assert.equal(connectionUI.element.querySelector(".conn-list").scrollTop, 120);
assert.equal(connectionUI.element.querySelector(".conn-detail").scrollTop, 75);
assert.equal(cAt("act:technical").parentElement.open, true);
connectionView = { boards: [{ connector: "github", rows: [{ ...mergeRow, checks: "failing", review: "approved", merge: false,
  facts: { ...mergeRow.facts, runs: [{ name: "Fine", state: "passed" }, { name: "Very long failed check name", state: "failed" }] } }, second] }] };
connectionUI.update(connectionView); connectionReads.at(-1).resolve(healthy); await tick();
assert.equal(connectionUI.element.querySelector(".conn-check-list").open, true, "Newly failing checks become visible even after success was collapsed");
assert.equal(cAt("row:org/alpha#7").querySelector(".conn-tag").textContent, "Checks failing", "A review approval cannot conceal failing checks");
assert.ok(connectionUI.element.querySelector(".conn-check").classList.contains("failed"));
assert.equal(cAt("act:merge"), null, "Only the real row offers merge");
connectionUI.element.querySelector(".conn-check-list").open = false; connectionUI.redraw();
assert.equal(connectionUI.element.querySelector(".conn-check-list").open, false, "Manual disclosure choice survives an unchanged status");
const outsideControl = new Element("button"); outsideControl.dataset.fid = "act:open"; document.body.append(outsideControl); outsideControl.focus();
connectionUI.redraw(); assert.equal(document.activeElement, outsideControl, "A poll cannot steal focus from another region");
cSearch.value = "river"; cSearch.fire("input"); assert.equal(connectionUI.element.querySelectorAll(".conn-row").length, 1);
assert.equal(connectionUI.element.querySelector(".conn-list").scrollTop, 0);
connectionUI.refresh(); connectionReads.at(-1).reject(new Error("Status offline")); await tick();
assert.ok(connectionUI.element.textContent.includes("Connection status could not load"));
assert.ok(connectionUI.element.textContent.includes("An old useful result"));
assert.equal(connectionUI.element.querySelector(".conn-search"), cSearch);
cAt("act:status-retry").fire("click"); const obsoleteStatus = connectionReads.at(-1);
cAt("act:status-retry").fire("click"); connectionReads.at(-1).resolve(healthy); await tick();
obsoleteStatus.reject(new Error("Obsolete")); await tick();
assert.equal(connectionUI.element.querySelector(".view-state.error"), null);
assert.deepEqual(connectionEffects, [], "Search, filtering, disclosing and local status retry perform no external action");
window.innerWidth = 480; connectionUI.redraw(); cAt("row:org/beta#9").fire("click");
assert.ok(cAt("act:back-detail"));
connectionUI.element.fire("keydown", { key: "Escape", preventDefault() {}, stopPropagation() {} });
assert.equal(connectionUI.element.querySelector(".conn-detail"), null);
assert.equal(document.activeElement, cAt("row:org/beta#9"));
cSearch.focus(); connectionUI.element.fire("keydown", { key: "Escape", target: cSearch, isComposing: true, preventDefault() {}, stopPropagation() {} });
assert.equal(cSearch.value, "river");
connectionUI.element.fire("keydown", { key: "Escape", target: cSearch, preventDefault() {}, stopPropagation() {} });
assert.equal(cSearch.value, ""); assert.equal(document.activeElement, cSearch);
storage.delete("vults.connections-query");
const unknownReads = [];
const unknownUI = createConnections({ status: () => new Promise((resolve, reject) => unknownReads.push({ resolve, reject })), refresh: async () => {},
  onMerged() {}, merge: async () => {}, open() {}, settings() {}, zeca: () => null });
unknownUI.update({ boards: [] }); unknownReads.at(-1).reject(new Error("Unavailable")); await tick();
assert.equal(unknownUI.element.querySelector(".nest-add"), null, "An unreadable status never claims the connector is off");
unknownUI.element.querySelector('[data-fid="act:status-retry"]').fire("click"); unknownReads.at(-1).resolve(healthy); await tick();
assert.ok(unknownUI.element.querySelector(".conn-waiting"));
assert.equal(unknownUI.element.querySelector(".nest-add"), null);
console.log("check-surface-continuity: Connections search/filter/empty states, independent scroll, explicit actions, check priority/disclosures, narrow Escape and stale status ordering without side effects");


// Week failures stop loading, keep a known recap and cannot replace a newer read.
const weekReads = [], weekEffects = [];
const weekUI = createWeek({ activity: () => new Promise((resolve, reject) => weekReads.push({ resolve, reject })),
  github: async () => ({ state: "off" }), saveImage: async () => { weekEffects.push("save"); return null; },
  settings: () => weekEffects.push("history-settings"), connectors: () => weekEffects.push("settings") });
const weekPage = weekUI.element.shadowRoot.querySelector(".page");
weekUI.refresh(); assert.ok(weekPage.textContent.includes("Reading the history"));
weekReads[0].reject(new Error("History unavailable")); await tick();
assert.ok(weekPage.textContent.includes("Could not load This week"));
assert.ok(!weekPage.textContent.includes("Reading the history"));
const weekRetry = weekPage.querySelectorAll("button").find(b => b.textContent === "Try again");
weekRetry.focus(); weekRetry.fire("click"); weekRetry.fire("click"); assert.equal(weekReads.length, 2);
const recap = { dictatedWords: 135, history: true, today: "2026-10-15", weeks: ["2026-10-12"], grid: [],
  week: { monday: "2026-10-12", sunday: "2026-10-18", turns: 0, headline: "Known recap", projects: [], days: [0, 0, 0, 0, 0, 0, 0] } };
weekReads[1].resolve(recap); await tick(); assert.ok(weekPage.textContent.includes("Known recap"));
assert.ok(weekPage.textContent.includes("135") && weekPage.textContent.includes("Dictated words"), "Dictation totals remain visible without agent turns");
weekUI.refresh(); weekReads[2].reject(new Error("Stale recap")); await tick();
assert.ok(weekPage.textContent.includes("Known recap")); assert.ok(weekPage.textContent.includes("last loaded view is kept"));
weekUI.refresh(); const oldWeek = weekReads.at(-1); weekUI.refresh(); weekReads.at(-1).resolve(recap); await tick();
oldWeek.reject(new Error("Obsolete")); await tick(); assert.equal(weekPage.querySelector(".view-state.error"), null);
assert.deepEqual(weekEffects, [], "Retrying the recap never clears, exports or changes history settings");
const populatedWeek = { ...recap.week, turns: 4, active_secs: 120, steps: 12, added: 8, removed: 2, files: 2,
  allowed: 0, denied: 0, answered: 0, longest_secs: 60, agents: [["codex", 4]], days: [120, 0, 0, 0, 0, 0, 0] };
weekUI.refresh(); weekReads.at(-1).resolve({ ...recap, week: populatedWeek, previous: { ...populatedWeek, turns: 2, active_secs: 60 } }); await tick();
assert.ok(weekPage.textContent.includes("+100% compared with the previous week"));
assert.ok(weekPage.textContent.includes("The previous week's value was zero."));
assert.ok(weekPage.textContent.includes("Turns by agent"));
assert.ok(!weekPage.textContent.includes("Keep a history"), "Activity does not edit retention settings");
weekUI.refresh(); weekReads.at(-1).resolve({ ...recap, week: populatedWeek, previous: { ...populatedWeek, turns: 0 } }); await tick();
assert.ok(weekPage.textContent.includes("No recorded baseline for comparison."));

console.log("check-surface-continuity: Week initial error/retry, retained recap and obsolete read rejection without setting/export actions");

assert.deepEqual(NEST_TABS, ["flock", "chat", "activity", "connections", "dictation"]);
for (const zeca of [true, false]) {
  const tabs = NEST_TABS.filter(id => id !== "chat" || zeca);
  for (let n = 1; n <= 5; n++) {
    assert.equal(tabByKey({ code: `Digit${n}`, altKey: true }, zeca), tabs[n - 1]);
  }
  const commands = [], routes = [];
  const items = paletteItems({ sessions: [], focus() {}, review() {},
    command: zeca ? text => commands.push(text) : null, tab: id => routes.push(id) });
  assert.ok(!items.some(item => /task|delegate/i.test(item.label)), "No retired entry points remain in the palette");
  for (const item of items) item.run();
  assert.deepEqual(routes, tabs);
  assert.deepEqual(commands, zeca ? ["/continue ", "/remember "] : []);
}
console.log("check-surface-continuity: five tabs, keyboard order with Zeca on/off, palette removal and retained continuation/memory");

const panelHost = new Element("div"), panel = new Element("aside"), retainedInput = new Element("input");
panelHost.clientWidth = 1180; panelHost.append(panel); panel.append(retainedInput); retainedInput.value = "unsent draft";
sidePanel(panel, "check-panel", "Memory", 300, "left", 180, 400);
const handle = panel.querySelector(".panel-handle"), foldPanel = panel.querySelector(".panel-fold");
handle.fire("keydown", { key: "ArrowRight", preventDefault() {} }); assert.equal(handle.getAttribute("aria-valuenow"), "320");
foldPanel.click(); assert.equal(foldPanel.getAttribute("aria-expanded"), "false");
assert.equal(retainedInput.value, "unsent draft"); assert.ok(panel.contains(retainedInput));
const recreatedPanel = new Element("aside"); sidePanel(recreatedPanel, "check-panel", "Memory", 300, "left", 180, 400);
assert.equal(recreatedPanel.querySelector(".panel-fold").getAttribute("aria-expanded"), "false");
handle.fire("dblclick"); assert.equal(handle.getAttribute("aria-valuenow"), "300"); assert.equal(foldPanel.getAttribute("aria-expanded"), "true");
handle.fire("pointerdown", { button: 0, clientX: 100, pointerId: 1, preventDefault() {} });
handle.fire("pointermove", { clientX: 5000 }); handle.fire("pointerup"); assert.equal(handle.getAttribute("aria-valuenow"), "400");
handle.fire("keydown", { key: "Home", preventDefault() {} }); assert.equal(handle.getAttribute("aria-valuenow"), "180");
weekPage.querySelector(".week-settings").click(); assert.deepEqual(weekEffects, ["history-settings"]);
console.log("check-surface-continuity: resized/folded panel state, retained draft, reset, pointer/keyboard bounds and Settings-owned retention");

// Both canvas owners must refresh live colors even when animation is paused.
const scenePaint = Scene.prototype.draw, skyPaint = Sky.prototype.draw;
const paintedThemes = [];
Scene.prototype.draw = function () { paintedThemes.push(this.canvas); };
Sky.prototype.draw = function () { paintedThemes.push(this.canvas); };
const themedScene = new Scene({ width: 100, height: 40, wire: 30, zeca: null, vults: { max: 0, scale: 1, at: () => ({ x: 0, wire: 30 }) } });
const themedSky = new Sky(() => {});
themedScene.colors = themedSky.colors = { wire: "old-theme" };
document.documentElement = new Element("html");
applyTheme("black", true); applyTheme("light"); applyTheme("black", true);
assert.equal(document.documentElement.getAttribute("data-theme"), "light", "A stale startup read cannot undo a newer theme event");
applyTheme("black"); assert.equal(document.documentElement.getAttribute("data-theme"), "black");
assert.equal(themedScene.colors, null); assert.equal(themedSky.colors, null);
assert.ok(paintedThemes.includes(themedScene.canvas) && paintedThemes.includes(themedSky.canvas));
themedScene.setActive(false); themedSky.update([], false); paintedThemes.length = 0;
themedScene.colors = themedSky.colors = { wire: "old-theme" }; applyTheme("light");
assert.equal(themedScene.colors, null); assert.equal(themedSky.colors, null);
assert.ok(!paintedThemes.includes(themedScene.canvas) && !paintedThemes.includes(themedSky.canvas), "Hidden canvases invalidate without starting animation");
Scene.prototype.draw = scenePaint; Sky.prototype.draw = skyPaint;
console.log("check-surface-continuity: shared theme events win over late startup reads");

const cardEvents = [];
const cardActions = { keys: {}, skip: id => cardEvents.push(["skip", id]), review: s => cardEvents.push(["review", s.id]),
  decide: (...args) => cardEvents.push(["decide", ...args]), decideAlways: id => cardEvents.push(["always", id]),
  answer: (...args) => cardEvents.push(["answer", ...args]), release: id => cardEvents.push(["release", id]),
  keyboard: on => cardEvents.push(["keyboard", on]), relayout() {}, jump() {}, dismiss() {}, hush() {}, openChat() {} };
const skipSession = { id: "skip-test", agent: "codex", project: "Alpha", cwd: "/work/alpha", status: "approval", attention: "needs-you", card: true,
  activity: null, step: null, steps: [], step_count: 0, subagents: 0, waiting: true };
const skipApproval = { agent: "codex", session: "skip-test", request: "skip-permission", tool: "Bash", target: "git status", questions: [], queue: 1 };
const permissionCard = focusCard(skipSession, skipApproval, { sync() {} }, new Element("aside"), cardActions, false);
const oldAllow = permissionCard.querySelectorAll("button").find(b => b.textContent === "Allow");
permissionCard.querySelector(".card-skip").click(); oldAllow.click();
assert.deepEqual(cardEvents, [["skip", "skip-permission"]], "Quick Skip cannot fall through to an old Allow control");
const skippedCard = focusCard({ ...skipSession, skipped: true }, null, { sync() {} }, new Element("aside"), cardActions, false);
skippedCard.querySelectorAll("button").find(b => b.textContent === "Show the card").click();
assert.deepEqual(cardEvents.at(-1), ["review", "skip-test"]);
const questionApproval = { ...skipApproval, request: "skip-question", questions: [{ question: "Choose a label", options: [{ label: "A", description: "" }], multi: false }] };
const questionHost = new Element("main"); questionHost.append(...questionBody({ ...skipSession, status: "question" }, questionApproval, cardActions));
questionHost.querySelectorAll("button").find(b => b.textContent === "Other…").click();
const answerDraft = questionHost.querySelector(".other-input"); answerDraft.value = "unfinished answer"; answerDraft.oninput();
questionHost.querySelector(".card-skip").click();
assert.deepEqual(cardEvents.at(-1), ["skip", "skip-question"]);
assert.ok(!cardEvents.some(([name]) => ["answer", "release", "decide", "always"].includes(name)));
questionHost.replaceChildren(...questionBody({ ...skipSession, status: "question" }, questionApproval, cardActions));
assert.equal(questionHost.querySelector(".other-input").value, "unfinished answer", "Skipping preserves the question draft");
questionHost.querySelectorAll("button").find(b => b.textContent === "Send").click();
assert.deepEqual(cardEvents.at(-1), ["answer", "skip-question", ["unfinished answer"]]);
console.log("check-surface-continuity: Quick Skip blocks stale Allow, retains question drafts and resumes only through an explicit action");

const nowEvents = [];
const nowUI = createNow({ request: (...args) => nowEvents.push(['request', args[0]]), session: s => nowEvents.push(['session', s.id]),
  relayout() {},
  connection: (...args) => nowEvents.push(['connection', ...args]), alert: key => nowEvents.push(['alert', key]),
  chat: () => nowEvents.push(['chat']), memory: () => nowEvents.push(['memory']), room: tab => nowEvents.push(['room', tab]) });
const attentionSession = { ...skipSession, id: 'now-session', skipped: true };
const reviewRow = { ...row, item: 'now-review', group: 'to-review', checks: null, review: null, merge: false };
const failedRow = { ...row, item: 'now-failed', group: 'yours', checks: 'failing', review: null, merge: false };
const nowView = { sessions: [attentionSession], approval: null, alerts: [], boards: [{ connector: 'github', rows: [reviewRow, failedRow] }], inbox: [
  ...['offer-one', 'offer-two'].map(request => ({ level: 'needs-you', source: { kind: 'request', request, session: { agent: attentionSession.agent, id: attentionSession.id }, tool: 'Bash', target: request, description: null, question: false, skipped: true } })),
  { level: 'failed', source: { kind: 'connection', connector: 'github', item: failedRow.item } },
  { level: 'info', source: { kind: 'connection', connector: 'github', item: reviewRow.item } },
  { level: 'needs-you', source: { kind: 'chat', count: 2 } }, { level: 'info', source: { kind: 'memory', count: 1 } },
] };
document.body.replaceChildren(nowUI.element);
nowUI.update(nowView, true);
assert.equal(nowUI.element.querySelectorAll('.now-row').length, 6);
assert.deepEqual(nowEvents, [], 'Drawing the inbox has no effects');
const nowAt = id => nowUI.element.querySelector(`[data-now="${id}"]`);
nowAt('request:offer-two').click();
assert.deepEqual(nowEvents, [['request', 'offer-two']], 'Each offer opens by its own id, including deferred offers of one session');
const focusNow = nowAt('request:offer-one'); focusNow.focus();
nowUI.element.querySelector('.now-feed').scrollTop = 91;
nowUI.update({ ...nowView, sessions: [{ ...attentionSession, step: 'updated metadata' }] }, true);
assert.equal(document.activeElement.dataset.now, 'request:offer-one');
assert.equal(nowUI.element.querySelector('.now-feed').scrollTop, 91);
nowAt('connection:github:now-failed').click(); nowAt('chat').click(); nowAt('memory').click();
assert.deepEqual(nowEvents.slice(1), [['connection', 'github', 'now-failed'], ['chat'], ['memory']]);
nowAt('seen-all').focus(); nowAt('seen-all').click();
assert.equal(document.activeElement.dataset.now, 'request:offer-one', 'Seen hands keyboard focus to a remaining item');
assert.equal(nowUI.element.querySelectorAll('.now-row').length, 5, 'Seen only removes quiet connection news; failures and cards stay');
assert.ok(nowAt('connection:github:now-failed')); assert.ok(nowAt('request:offer-one'));
assert.equal(nowEvents.length, 4, 'Marking seen sends no backend action');
nowUI.update(nowView, false);
assert.equal(nowUI.element.querySelectorAll('.now-row').length, 3, 'Zeca off removes only his Chat and Memory items');
nowUI.update({ ...nowView, inbox: [{ level: 'info', source: { kind: 'memory', count: null } }] }, true);
assert.ok(nowUI.element.textContent.includes('Could not read memory proposals.'));
nowUI.update({ ...nowView, inbox: [] }, true);
assert.ok(nowUI.element.textContent.includes('Nothing needs your attention'));
assert.equal(nowUI.element.querySelectorAll('.now-row').length, 0, 'The renderer never reconstructs a competing inbox from sessions or boards');
assert.equal(nowEvents.length, 4);
connectionUI.update(connectionView);
connectionUI.select('github', mergeRow.item);
assert.equal(cSearch.value, '', 'A Now link reveals its item despite an old search');
assert.equal(storage.get('vults.connections-query'), '', 'The revealed connection stays visible when reopened');
assert.ok(connectionUI.element.querySelector('.conn-detail').textContent.includes(mergeRow.title), 'A Now link opens the exact selected detail');
assert.ok(!connectionEffects.includes('merge'), 'Opening connection detail never merges');
console.log('check-surface-continuity: Now uses the shared inbox, opens exact deferred offers, retains focus/scroll, separates Seen from decisions and routes connection detail');

// The renderer consumes the core's rest stage, including on a late mount.
const restSession = { ...skipSession, id: 'rest-clock', status: 'idle', attention: 'quiet', waiting: false, card: false, rested: false, species: 'atratus' };
assert.equal(badgeOf(restSession), 'finished');
assert.equal(badgeOf({ ...restSession, rested: true }), undefined);
assert.equal(badgeOf({ ...restSession, status: 'approval', rested: null }), 'approval');
assert.equal(badgeOf({ ...restSession, rested: null }), undefined, 'Dismissing a failure cannot invent a successful rest');
assert.equal(emoteFor({ ...restSession, status: 'finished', rested: false }), 'done');
assert.equal(emoteFor(restSession), 'done');
assert.equal(emoteFor({ ...restSession, status: 'finished', rested: true }), null);
const skyContext = new Proxy({}, { get: () => () => {} });
function restSky() {
  const sky = new Sky(() => {});
  sky.ctx = skyContext; sky.colors = {};
  sky.cache = () => ({ drawShot() {} });
  sky.setVisitors(false);
  sky.place(new Map([['codex:rest-clock', { x: 120, y: 30, scale: 1 }]]), { left: 0, top: 0, width: 340, height: 38, radius: 14 });
  return sky;
}
setDayPart('day'); setMusic(false); Clock.freeze(0);
const restingSky = restSky();
restingSky.update([restSession], true);
Clock.freeze(60_000); restingSky.draw();
assert.equal(restingSky.owns('codex:rest-clock'), false, 'A local clock cannot advance a recent core state');
const readyRest = { ...restSession, rested: true };
restingSky.update([readyRest], true);
assert.equal(restingSky.owns('codex:rest-clock'), true);
const reopenedSky = restSky(); reopenedSky.update([readyRest], true);
assert.equal(reopenedSky.owns('codex:rest-clock'), true, 'Mounting a new renderer cannot restart the core rest');
restingSky.call('codex:rest-clock');
Clock.freeze(65_000); restingSky.draw();
assert.equal(restingSky.owns('codex:rest-clock'), false);
Clock.freeze(89_999); restingSky.draw();
assert.equal(restingSky.owns('codex:rest-clock'), false, 'Calling a bird down keeps a local visual rest');
Clock.freeze(90_000); restingSky.draw();
assert.equal(restingSky.owns('codex:rest-clock'), true);
restingSky.update([{ ...readyRest, status: 'approval', attention: 'needs-you', rested: null }], true);
Clock.freeze(92_000); restingSky.draw();
assert.equal(restingSky.owns('codex:rest-clock'), false, 'A permission lands the bird without answering it');
setDayPart('night'); restingSky.update([readyRest], true);
assert.equal(restingSky.owns('codex:rest-clock'), false);
setDayPart('day'); setMusic(true); restingSky.update([readyRest], true);
assert.equal(restingSky.owns('codex:rest-clock'), false, 'Music keeps idle courtship on the wire');
setMusic(false); restingSky.update([readyRest], true);
assert.equal(restingSky.owns('codex:rest-clock'), true);
restingSky.motion = { matches: true }; restingSky.draw();
assert.equal(restingSky.owns('codex:rest-clock'), false, 'Reduced motion keeps the bird perched');
restingSky.motion = { matches: false }; document.body.classList.add('still'); restingSky.draw();
assert.equal(restingSky.owns('codex:rest-clock'), false);
document.body.classList.remove('still'); restingSky.update([readyRest], true);
assert.equal(restingSky.owns('codex:rest-clock'), true);
let restFrames = 0; const restTimeout = window.setTimeout; window.setTimeout = () => { restFrames++; return 0; };
restingSky.update([readyRest], false);
assert.equal(restFrames, 0, 'A hidden sky schedules no animation');
window.setTimeout = restTimeout; setDayPart('day'); setMusic(false); Clock.freeze(null);
console.log('check-surface-continuity: core-owned rest, recent green mark, immediate late mount, call-down, urgent landing, music/night, reduced motion and hidden sky');

// Reduced motion still draws each state, without a repeating frame timer or advancing emotes.
{
  const media=window.matchMedia, timeout=window.setTimeout;
  let reduce=true, scheduled=0;const listeners=[];
  window.matchMedia=query=>query.includes('prefers-reduced-motion')?{get matches(){return reduce;},addEventListener:(_event,fn)=>listeners.push(fn)}:media(query);
  window.setTimeout=()=>{scheduled++;return 0;};
  const scene=new Scene({width:114,height:108,wire:100,zeca:{x:21,wire:100,scale:3},vults:{max:0,scale:1,at:()=>({x:0,wire:100})},skyTop:0,flights:false,reflow:false,emotes:false});
  scene.colors={wire:'#000',claude:'#000'};
  const frames=[];scene.cache=()=>({drawShot:(_ctx,shot)=>frames.push(JSON.stringify(shot.frame))});
  scene.update([],null,{clip:'speak',agent:'claude'});scheduled=0;scene.draw();assert.equal(scheduled,0);
  Clock.freeze(Clock.now()+1000);scene.draw();assert.equal(frames[0],frames[1],'Repeated paused redraws keep the same pose');
  scene.update([],null,{clip:'question',agent:'claude'});scheduled=0;scene.draw();assert.equal(scene.playing('zeca'),'question');assert.equal(scheduled,0,'A waiting card updates its static pose without a frame loop');
  reduce=false;listeners.forEach(fn=>fn());assert.ok(scheduled>0,'Restoring motion refreshes the active scene');scheduled=0;scene.draw();assert.ok(scheduled>0);
  document.body.classList.add('still');scheduled=0;scene.draw();assert.equal(scheduled,0,'Manual still mode also stops the frame loop');document.body.classList.remove('still');
  scene.setActive(false);scheduled=0;listeners.forEach(fn=>fn());assert.equal(scheduled,0,'A media change never wakes a hidden scene');
  window.matchMedia=media;window.setTimeout=timeout;Clock.freeze(null);
}
console.log('check-surface-continuity: static state poses under reduced motion/manual pause, no repeating timer, live preference resume and no hidden wake');

// Recording is an explicit, page-owned action; stale completions cannot restart a hidden check.
{
  let snapshot = { id: 0, phase: "idle", elapsedMs: 0, maxSeconds: 60, microphone: null, monitor: null, fallback: false, error: null, modelId: "base", language: "en", hasTranscript: false };
  const calls = [];
  let start = null;
  const backend = {
    transcript: async id => { calls.push(["transcript", id]); return { id, microphone: { durationMs: 1200, segments: [{ startMs: 100, endMs: 900, text: "Keep <script> as literal transcript text." }] }, system: { durationMs: 1200, segments: [] } }; },
    status: async id => { calls.push(["status", id]); return { ...snapshot }; },
    prepare: async () => { calls.push(["prepare"]); snapshot = { ...snapshot, id: snapshot.id+1, phase: "ready", microphone: "mic <literal>", monitor: "speakers.monitor", fallback: true }; return { ...snapshot }; },
    begin: async (id, consent) => { calls.push(["begin", id, consent]); if (start) return start.promise; snapshot = { ...snapshot, phase: "recording", elapsedMs: 1_250 }; return { ...snapshot }; },
    end: async (id, cancel) => { calls.push(["end", id, cancel]); snapshot = { ...snapshot, phase: cancel ? "cancelled" : "finished", hasTranscript: !cancel }; return { ...snapshot }; },
    recover: async () => { calls.push(["recover"]); snapshot = { ...snapshot, phase: "idle", error: null }; return { ...snapshot }; },
  };
  const meeting = createMeetings(backend); document.body.append(meeting.element);
  const button = label => meeting.element.querySelectorAll("button").find(button => button.textContent === label);
  const consent = meeting.element.querySelector("input");
  assert.equal(calls.length, 0, "Mounting a hidden page never reads sources or records");
  meeting.setActive(true); await tick();
  assert.deepEqual(calls.map(call => call[0]), ["status"]);
  button("Review audio sources").click(); await tick();
  assert.ok(meeting.element.textContent.includes("mic <literal>"));
  assert.equal(button("Record and transcribe").disabled, true);
  button("Record and transcribe").click(); assert.equal(calls.filter(call => call[0]==="begin").length, 0);
  consent.checked = true; consent.fire("change");
  button("Record and transcribe").click(); await tick();
  assert.deepEqual(calls.find(call => call[0]==="begin"), ["begin", 1, true]);
  assert.equal(consent.checked, false);
  assert.equal(meeting.element.querySelector("output").textContent, "00:01");
  assert.equal(button("Stop recording").hidden, false);
  button("Stop recording").click(); await tick();
  assert.ok(meeting.element.textContent.includes("Transcript ready. Temporary audio was deleted."));
  assert.equal(button("Stop recording").hidden, true);
  await tick();
  assert.ok(meeting.element.textContent.includes("Keep <script> as literal transcript text."));
  assert.equal(meeting.element.querySelectorAll("script").length, 0);
  const rows = meeting.element.querySelector(".meeting-transcript-rows");
  const row = rows.firstElementChild;
  const disclosure = meeting.element.querySelector(".meeting-transcript").querySelector("details");
  disclosure.open = true;
  meeting.relabel(); assert.equal(rows.firstElementChild, row, "Polling/relabel retains transcript nodes and selection");
  assert.equal(disclosure.open, true, "Polling retains transcript details");
  button("System audio").click(); assert.ok(rows.textContent.includes("No speech was detected in this source."));
  assert.equal(button("System audio").getAttribute("aria-pressed"), "true");
  button("Microphone").click();
  button("Discard transcript").click(); await tick();
  assert.equal(meeting.element.querySelector(".meeting-transcript").hidden, true);
  button("Review audio sources").click(); await tick();
  assert.equal(consent.checked, false, "Each review needs a new explicit confirmation");
  consent.checked = true; consent.fire("change"); button("Record and transcribe").click(); await tick();
  meeting.setActive(false); await tick();
  assert.deepEqual(calls.at(-1), ["end", 2, true], "Leaving the work page cancels the active check");
  meeting.setActive(true); await tick();
  button("Review audio sources").click(); await tick();
  let resolve; start = { promise: new Promise(done => { resolve = done; }), resolve: value => resolve(value) };
  consent.checked = true; consent.fire("change"); button("Record and transcribe").click();
  meeting.setActive(false);
  start.resolve({ ...snapshot, phase: "starting" }); await tick();
  assert.deepEqual(calls.at(-1), ["end", 3, true], "A late start response is cancelled rather than drawn on a hidden page");
  assert.equal(consent.checked, false);
  meeting.element.remove();
}
console.log("check-surface-continuity: explicit meeting consent, source review, stop/cancel, hidden ownership and late-start cleanup");

// The developer preview requires a separate per-recording choice and never shares source IDs.
{
  let snapshot = { id: 1, phase: "idle", elapsedMs: 0, maxSeconds: 60, microphone: "mic", monitor: "out.monitor", fallback: false, error: null, modelId: "base", language: "en", hasTranscript: false, identificationAvailable: true };
  const calls = [];
  const data = { id: 1, microphone: { durationMs: 4000, attribution: "analyzed", segments: [
    { startMs: 100, endMs: 500, text: "Hello", speaker: 0 }, { startMs: 500, endMs: 900, text: "world.", speaker: 0 },
    { startMs: 1000, endMs: 1500, text: "Uncertain.", speaker: null }, { startMs: 1800, endMs: 2000, text: "Other voice.", speaker: 1 },
  ] }, system: { durationMs: 4000, attribution: "unavailable", segments: [{ startMs: 0, endMs: 1500, text: "Kept without labels.", speaker: null }] } };
  const view = createMeetings({ status: async () => ({ ...snapshot }), prepare: async () => ({ ...snapshot, phase: "ready" }),
    begin: async (id, consent, identify) => { calls.push([id, consent, identify]); snapshot = { ...snapshot, phase: "finished", hasTranscript: true }; return { ...snapshot }; },
    transcript: async () => structuredClone(data), end: async () => ({ ...snapshot, phase: "cancelled", hasTranscript: false }), recover: async () => snapshot });
  document.body.append(view.element); view.setActive(true); await tick();
  const button = text => view.element.querySelectorAll("button").find(b => b.textContent === text);
  button("Review audio sources").click(); await tick();
  const [consent, identify] = view.element.querySelectorAll("input");
  assert.equal(!!identify.checked, false);
  identify.checked = true; consent.checked = true; consent.fire("change");
  button("Record and transcribe").click(); await tick(); await tick();
  assert.deepEqual(calls, [[1, true, true]]);
  assert.equal(identify.checked, false, "A speaker choice is not reused for another recording");
  const rows = view.element.querySelector(".meeting-transcript-rows");
  assert.equal(rows.children.length, 3, "Adjacent words of the same voice remain readable");
  assert.ok(rows.textContent.includes("Voice 1 Hello world."));
  assert.ok(rows.textContent.includes("Unknown voice Uncertain."));
  assert.ok(rows.textContent.includes("00:01.00–00:01.50"), "Subsecond words never appear to have zero duration");
  assert.ok(rows.textContent.includes("Voice 2 Other voice."));
  assert.equal(data.microphone.segments.length, 4, "Rendering does not alter source word timing");
  button("System audio").click();
  assert.ok(rows.textContent.includes("Kept without labels."));
  assert.ok(!rows.textContent.includes("Unknown voice"), "Unavailable analysis must keep plain text without speaker labels");
  assert.ok(view.element.textContent.includes("Voice identification was unavailable. The transcript is kept without labels."));
  view.setActive(false); await tick(); view.element.remove();
}
console.log("check-surface-continuity: speaker preview opt-in, readable model words, unknown/failure fallback and separate sources");

// Quick's inline picks must consume only navigation, never apply a choice on an arrow or Escape.
{
  const changes = [];
  const settings = { presence: "island", foldAfter: 15, history: true, zeca: true, flock: "world" };
  const view = settingsQuick(settings, {}, () => {}, false, "general", { value: 680, failed: false,
    change: value => changes.push(value),
  });
  document.body.append(view);
  const pick = view.querySelectorAll(".quick-pick").find(node => node.dataset.quickPick === "width");
  const trigger = pick.querySelector(".quick-pick-btn"), list = pick.querySelector(".quick-pick-list");
  const choices = list.querySelectorAll("button");
  const key = (name, extra = {}) => {
    const result = { prevented: false, stopped: false };
    pick.fire("keydown", { key: name, target: document.activeElement,
      preventDefault: () => { result.prevented = true; }, stopPropagation: () => { result.stopped = true; }, ...extra });
    return result;
  };
  assert.equal(trigger.getAttribute("aria-controls"), list.id);
  assert.equal(trigger.getAttribute("aria-expanded"), "false");
  assert.equal(list.hidden, true);
  trigger.focus();
  key("ArrowDown", { isComposing: true });
  assert.equal(list.hidden, true, "Composition must not open a selector");
  key("ArrowDown");
  assert.equal(document.activeElement, choices[2], "Opening starts at the saved choice");
  assert.equal(trigger.getAttribute("aria-expanded"), "true");
  assert.deepEqual(choices[2].scrolledIntoView, { block: "nearest" });
  key("End"); assert.equal(document.activeElement, choices[3]);
  key("Home"); assert.equal(document.activeElement, choices[0]);
  key("ArrowUp"); assert.equal(document.activeElement, choices[3]);
  key("ArrowDown"); assert.equal(document.activeElement, choices[0]);
  assert.deepEqual(changes, []);
  assert.deepEqual(key("Escape"), { prevented: true, stopped: true }, "Escape closes the list before the island can fold");
  assert.equal(document.activeElement, trigger);
  assert.equal(list.hidden, true);
  trigger.click(); choices[3].click();
  assert.deepEqual(changes, [760], "Only explicit activation applies a choice");
  assert.equal(document.activeElement, trigger, "Focus returns before a callback can redraw the picker");
  trigger.click();
  assert.deepEqual(key("Tab"), { prevented: false, stopped: false }, "Tab closes without trapping native navigation");
  assert.equal(list.hidden, true);
  trigger.click();
  const outside = new Element("button"); document.body.append(outside); outside.focus();
  pick.fire("focusout", { relatedTarget: outside });
  assert.equal(list.hidden, true);
  assert.equal(document.activeElement, outside, "Leaving the selector must not steal focus");
  outside.remove(); view.remove();
  const failed = settingsQuick(settings, {}, () => {}, false, "general", { value: 680, failed: true, change() {} });
  assert.ok(failed.querySelector('[role="alert"]'), "A failed width save must be announced");
}
console.log("check-surface-continuity: Quick picker arrows/Home/End, IME, explicit choice, Escape, Tab, blur and focus return");

{
  const reads = [];
  let returned = false;
  const profiles = createProfiles(() => new Promise((resolve, reject) => reads.push({ resolve, reject })), () => { returned = true; });
  document.body.append(profiles.element);
  assert.equal(reads.length, 0, "Hidden profiles do not read a catalog");
  profiles.visible(true); assert.equal(reads.length, 1);
  profiles.visible(false);
  const fixture = [{ id: "reviewer", built_in: true, revision: 1, title: "Reviewer", about: "Review fixture", species: "gypaetus", look: "none", instructions: "Inspect changes.", skills: [], skill_catalog: [] }];
  reads[0].resolve(fixture); await tick();
  assert.equal(profiles.element.querySelectorAll(".agent-profile-choice").length, 0, "A retired read cannot repaint hidden profiles");
  profiles.visible(true); reads[1].reject(new Error("offline")); await tick();
  assert.ok(profiles.element.querySelector(".view-state"));
  profiles.element.querySelector(".view-state-body").querySelector("button").click();
  reads[2].resolve(fixture); await tick();
  const choice = profiles.element.querySelector(".agent-profile-choice");
  choice.click();
  assert.ok(profiles.element.querySelector(".agent-profile-detail").textContent.includes("Inspect changes."));
  assert.ok(profiles.element.querySelector(".agent-profile-detail").textContent.includes("1"));
  assert.equal(profiles.element.querySelector(".agent-profile-choice").getAttribute("aria-pressed"), "true");
  profiles.relabel();
  assert.equal(profiles.element.querySelector(".agent-profile-choice").getAttribute("aria-pressed"), "true");
  profiles.element.querySelector(".activity-heading").querySelector("button").click();
  assert.equal(returned, true);
  profiles.visible(false); profiles.element.remove();
}
console.log("check-surface-continuity: profile catalog lazy read, late cancellation, explicit retry, selection/revision and return");

{
  const preset = { id: "developer", built_in: true, revision: 1, title: "Developer", about: "Description", species: "atratus", look: "none", instructions: "Original instructions", skills: [], skill_catalog: [{ id: "verify-change", revision: 1, title: "Verify", content: "Run focused checks" }] };
  const writes = [];
  const profiles = createProfiles(async () => [preset], () => {}, edit => new Promise((resolve, reject) => writes.push({ edit, resolve, reject })));
  document.body.append(profiles.element); profiles.visible(true); await tick();
  profiles.element.querySelector(".agent-profile-choice").click();
  profiles.element.querySelector(".agent-profile-edit").click();
  const input = profiles.element.querySelector("input"); input.value = "Literal custom title"; input.fire("input");
  const text = profiles.element.querySelector("textarea"); text.value = "Keep this draft\nexactly."; text.fire("input");
  const skill = profiles.element.querySelector(".agent-profile-skills").querySelector("input");
  assert.ok(profiles.element.querySelector(".agent-profile-skill").textContent.includes("Run focused checks"));
  skill.checked = true; skill.fire("change");
  const species = profiles.element.querySelector('[data-field="species"]'); species.value = "vultur"; species.fire("change");
  const look = profiles.element.querySelector('[data-field="look"]'); look.value = "crown"; look.fire("change");
  assert.equal(profiles.element.querySelector("input").value, "Literal custom title", "Appearance updates retain text and controls");
  profiles.visible(false); profiles.relabel(); profiles.visible(true);
  assert.equal(profiles.element.querySelector('[data-field="species"]').value, "vultur");
  assert.equal(profiles.element.querySelector('[data-field="look"]').value, "crown");
  assert.equal(profiles.element.querySelector("textarea").value, "Keep this draft\nexactly.", "Navigation and relabel retain unsaved instructions");
  const submit = () => profiles.element.querySelector("form").fire("submit", { preventDefault() {} });
  submit(); submit(); assert.equal(writes.length, 1, "A pending save cannot create duplicate copies");
  writes[0].reject("storage"); await tick();
  assert.ok(profiles.element.querySelector('[role="alert"]'));
  assert.equal(profiles.element.querySelector("input").value, "Literal custom title");
  assert.equal(profiles.element.querySelector("textarea").value, "Keep this draft\nexactly.");
  submit();
  assert.deepEqual(writes[1].edit, { action: "copy", preset: "developer", title: "Literal custom title", instructions: "Keep this draft\nexactly.", species: "vultur", look: "crown", skill_ids: ["verify-change"] });
  const custom = { ...preset, built_in: false, id: "custom-7", title: writes[1].edit.title, instructions: writes[1].edit.instructions, species: writes[1].edit.species, look: writes[1].edit.look, skills: preset.skill_catalog };
  writes[1].resolve(custom); await tick();
  assert.equal(profiles.element.querySelectorAll(".agent-profile-choice").length, 2, "The preset survives creating a copy");
  assert.ok(profiles.element.querySelector(".agent-profile-detail").textContent.includes("Literal custom title"));
  profiles.element.querySelector(".agent-profile-edit").click(); submit();
  assert.equal(writes[2].edit.action, "update"); assert.equal(writes[2].edit.id, 7); assert.equal(writes[2].edit.revision, 1);
  assert.deepEqual(writes[2].edit.skill_ids, ["verify-change"]);
  writes[2].reject("conflict"); await tick();
  assert.ok(profiles.element.querySelector('[role="alert"]').textContent.includes("Cancel"));
  assert.equal(profiles.element.querySelector("textarea").value, custom.instructions);
  profiles.visible(false); profiles.element.remove();
}
console.log("check-surface-continuity: profile copies, retained drafts, single pending save, failure retry and expected revision");

{
  const original = Element.prototype.getContext;
  let pixels = 0;
  Element.prototype.getContext = function () {
    const canvas = this;
    return { fillRect(x, y, w, h) {
      assert.ok(x >= 0 && y >= 0 && x + w <= canvas.width && y + h <= canvas.height, "An accessory or species must never be clipped in a profile preview");
      pixels++;
    } };
  };
  try {
    const preset = {id:"developer",built_in:true,revision:1,title:"Developer",about:"Description",species:"atratus",look:"none",instructions:"Instructions",skills:[],skill_catalog:[]};
    const profiles = createProfiles(async () => [preset], () => {}, async () => preset);
    document.body.append(profiles.element);profiles.visible(true);await tick();
    profiles.element.querySelector(".agent-profile-choice").click();profiles.element.querySelector(".agent-profile-edit").click();
    const species = profiles.element.querySelector('[data-field="species"]'), look = profiles.element.querySelector('[data-field="look"]');
    assert.equal(species.options.length, 23);
    assert.ok(!look.options.some(option => option.value === "auto"), "Profile appearance is pinned, never calendar-dependent");
    for (const bird of species.options) for (const accessory of look.options) {
      species.value = bird.value;look.value = accessory.value;look.fire("change");
    }
    assert.ok(pixels > 0);profiles.visible(false);profiles.element.remove();
  } finally { Element.prototype.getContext = original; }
}
console.log("check-surface-continuity: every species/accessory preview fits with integer pixels and no calendar option");
