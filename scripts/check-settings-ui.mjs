// Exercise the real Settings entry with local native responses; no browser server or writes.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";
import { spawnSync } from "node:child_process";
const initialRoute = process.env.VULTS_SETTINGS_INITIAL_ROUTE ?? "";
if (initialRoute) globalThis.settingsTestWindowsBindings = { requested: {}, defaults: {}, active: {}, conflicts: [] };
const nodes = node => [node, ...node.children.flatMap(nodes)];
class Element {
  children = []; listeners = {}; attributes = {}; dataset = {}; style = { setProperty() {} };
  className = ""; value = ""; open = false; disabled = false; scrollTop = 0; selectionStart = null; selectionEnd = null;
  constructor(tag) { this.tagName = tag.toUpperCase(); }
  classList = {
    contains: name => this.className.split(" ").includes(name),
    add: name => { if (!this.classList.contains(name)) this.className += ` ${name}`; },
    remove: name => { this.className = this.className.split(" ").filter(v => v !== name).join(" "); },
    toggle: (name, on = !this.classList.contains(name)) => this.classList[on ? "add" : "remove"](name),
  };
  set textContent(text) { this.replaceChildren(); this.text = text; }
  get textContent() { return (this.text ?? "") + this.children.map(n => n.textContent).join(""); }
  append(...children) { for (const node of children) { node.remove(); node.parentElement = this; this.children.push(node); if (this.tagName === "SELECT" && node.tagName === "OPTION" && (node.selected || this.children.length === 1)) this.value = node.value; } }
  prepend(...children) { this.replaceChildren(...children, ...this.children); }
  replaceChildren(...children) { this.text = ""; for (const n of this.children) n.parentElement = null; this.children = []; this.append(...children); }
  remove() { if (this.parentElement) this.parentElement.children = this.parentElement.children.filter(n => n !== this); }
  before(...children) { this.parentElement?.append(...children); }
  after(...children) { this.parentElement?.append(...children); }
  contains(node) { return nodes(this).includes(node); }
  matches(selector) {
    return selector.split(",").some(part => {
      const s = part.trim();
      const attr = s.match(/\[([^=\]]+)(?:="([^"]*)")?\]/);
      if (attr) {
        const val = attr[1].startsWith("data-") ? this.dataset[attr[1].slice(5).replace(/-([a-z])/g, (_, letter) => letter.toUpperCase())] : this.attributes[attr[1]];
        if (attr[2] !== undefined ? val !== attr[2] : val === undefined) return false;
      }
      const plain = s.replace(/\[[^\]]*\]/g, "");
      const [tag, ...classes] = plain.split(".");
      return (!tag || this.tagName === tag.toUpperCase()) && classes.every(c => this.classList.contains(c));
    });
  }
  querySelectorAll(selector) { return nodes(this).slice(1).filter(n => n.matches(selector)); }
  querySelector(selector) { return this.querySelectorAll(selector)[0] ?? null; }
  setAttribute(name, value) { this.attributes[name] = value; }
  get isConnected() { return this === document.body || !!this.parentElement?.isConnected; }
  getAttribute(name) { return this.attributes[name] ?? null; }
  hasAttribute(name) { return name in this.attributes; }
  closest(selector) { return this.matches(selector) ? this : this.parentElement?.closest(selector) ?? null; }
  addEventListener(name, fn) { (this.listeners[name] ??= []).push(fn); }
  fire(name, event = {}) { for (const fn of this.listeners[name] ?? []) fn({ target: this, preventDefault() {}, stopPropagation() {}, ...event }); }
  focus() { document.activeElement = this; }
  setSelectionRange(start, end) { this.selectionStart = start; this.selectionEnd = end; }
  getContext() { return { clearRect() {}, fillRect() {} }; }
}
let diagnosticReads = 0, micStartReady;
const stoppedMics = [];
let initialSettingsReady;
const initialSettings = new Promise(resolve => { initialSettingsReady = resolve; });
const pendingErases = [];
const microphoneReads = [];
const pendingShortcutOpens = [];
const pendingSettings = [];
const pendingBudgets = [];
let budgetReads = 0;
let holdSettings = false;
const root = new Element("div"), events = {}, writes = [], routes = [];
root.id = "settings";
globalThis.document = { body: new Element("body"), documentElement: new Element("html"),
  getElementById: () => root, createElement: tag => new Element(tag),
  createElementNS: (_ns, tag) => new Element(tag),
  createTextNode: text => { const n = new Element("text"); n.textContent = text; return n; },
  addEventListener() {}, dispatchEvent() {}, hidden: false,
};
document.body.append(root);
const widthStorage = new Map(), windowEvents = {};
globalThis.localStorage = { getItem: key => widthStorage.get(key) ?? null, setItem: (key, value) => widthStorage.set(key, value) };
globalThis.window = { setInterval: () => 0, clearInterval() {}, setTimeout: () => 0, clearTimeout() {}, addEventListener(name, fn) { (windowEvents[name] ??= []).push(fn); } };
globalThis.matchMedia = () => ({ matches: true, addEventListener() {} });
globalThis.location = { hash: initialRoute ? `#${initialRoute}` : "#general" };
globalThis.Option = class extends Element {
  constructor(label, value, _default, selected) { super("option"); this.textContent = label; this.value = value; this.selected = selected; }
};
const settings = { dictationCommands: true, dictationCommandChat: false, dictationVocabulary: true, dictationProjectVocabulary: true, dictationMusic: "lower", dictationCancelByVoice: true, dictationSpokenCode: true, dictationCutRepeats: true, dictationRetryBase: true, dictationMicrophone: null, dictationInsert: false, dictationInsertAvailable: true, dictationWayland: true, dictationKeep: "30", dictationCaption: true, dictationSound: true, dictationDoubleTap: true, dictationSpaceLock: true, dictationEscapeCancel: true, sounds: true, volume: 50, foldAfter: 15, lang: "en", language: null, zeca: true,
  zecaSpecies: "atratus", zecaLook: "auto", flock: "brazil", nativeChat: true, chatAccess: "ask", memory: true,
  presence: "island", settingsPath: "/home/example/settings.json", dataPath: "/home/example/vults" };
globalThis.settingsTestBridge = new Proxy({}, { get: (_obj, name) => {
  if (name.startsWith("on")) return cb => { events[name] = cb; };
  return async (...args) => {
    if (globalThis.failReads?.has(name)) throw new Error("simulated failure");
    switch (name) {
      case "appSettings": return initialSettings;
      case "windowsShortcuts": return globalThis.settingsTestWindowsBindings ?? null;
      case "questionInstallStatus": return [];
      case "dictationMicrophones": return new Promise((resolve, reject) => microphoneReads.push({ resolve, reject }));
      case "dictationClear": writes.push([name]); return new Promise((resolve, reject) => pendingErases.push({ resolve, reject }));
      case "openShortcutSettings": writes.push([name]); return new Promise((resolve, reject) => pendingShortcutOpens.push({ resolve, reject }));
      case "setDictationInsert": writes.push([name, ...args]); return null;
      case "retryDictationPermission": writes.push([name]); return;
      case "dictationMicStart": return new Promise(resolve => { micStartReady=resolve; });
      case "dictationMicStop": stoppedMics.push(args[0]); return;
      case "dictationDoctor": diagnosticReads++; return { system: { host: null, device: null, rate: null, session: "unknown", shortcuts: "unavailable", keyboard: "off", kwin: "unverified", gpu: false, vad: false }, counters: { loops: 0, retries: 0, copied: 0 }, timings: null };
      case "dictationStatus": return null;
      case "dictationPermission": return "off";
      case "setDictationCommands": case "setDictationMusic": case "setDictationProcessing": case "setDictationMicrophone": case "setDictationGesture": case "setTheme": case "setWidget": case "setPresence": case "setSounds": case "setDictationKeep":
        writes.push([name, ...args]);
        if (holdSettings) return new Promise((resolve, reject) => pendingSettings.push({ resolve, reject }));
        return;
      case "spendToday": budgetReads++; return { usd: 1, tokens: 100, budget: 2, subscription: true };
      case "budgetSet": writes.push([name, ...args]); return new Promise((resolve, reject) => pendingBudgets.push({ resolve, reject }));
      case "nestIsFull": return false;
      case "monitors": case "rulesList": case "policiesList": case "agentsDoctor": return [];
      case "connectorsStatus": return [{ id: "github", enabled: false, lastOk: null, error: null, watching: 0, overBudget: false, account: "maintainer", host: "github.com" }, { id: "gitlab", enabled: false, lastOk: null, error: null, watching: 0, overBudget: false, account: "reviewer", host: "gitlab.com" }];
      case "projectPrefs": return {};
      case "installStatus": return { installed: false, hookReady: true, statusLine: "ours", configPath: "/home/example/agent.json" };
      case "apiProviders": return { selected: "example", providers: [{ id: "example", label: "Example", local: false, hasKey: false }] };
      case "webSettings": return { on: false, engine: "brave", hasKey: false };
      case "commandCheck": return { status: "parsed", actions: [{ action: "focus", target: "api", value: null, status: "recognized", candidates: [] }] };
      case "voiceStatus": return { ready: false, models: [
        {id:"base",label:"Base: fast, but weak outside English",size:60000000,installed:false},
        {id:"small",label:"Small: good in Portuguese and other languages",size:190000000,installed:false},
        {id:"turbo",label:"Large v3 Turbo: the most accurate; best with a GPU",size:574000000,installed:false}], downloading: [], language: null };
      case "phoneSettings": return { on: false, transport: "relay", relay: null, devices: [] };
      case "activity": return { history: true };
      case "openNest": routes.push(args[0]); return;
      default:
        if (/^(set|installApply|installPreview|apiKey|phonePair|clear|ruleRemove)/.test(name)) writes.push([name, ...args]);
        return {};
    }
  };
} });
const ui = fileURLToPath(new URL("../ui/", import.meta.url));
const bundle = await build({ configFile: false, root: ui, logLevel: "silent", plugins: [{
  name: "settings-native-mock", enforce: "pre", resolveId(id) {
    if (/\/bridge(?:\.ts)?$/.test(id)) return "\0settings-bridge";
    if (id.endsWith("/windows-shortcuts")) return "\0settings-windows-shortcuts";
    if (id.endsWith("/nest/head")) return "\0settings-head";
    if (id === "@tauri-apps/api/app") return "\0settings-version";
  }, load(id) {
    if (id === "\0settings-bridge") return "export const Bridge = globalThis.settingsTestBridge;";
    if (id === "\0settings-windows-shortcuts") return `
      import { createWindowsShortcuts as production } from ${JSON.stringify(`${ui}src/surfaces/settings/windows-shortcuts.ts`)};
      export const createWindowsShortcuts = (...args) => {
        const controller = production(...args);
        globalThis.settingsTestWindowsController = controller;
        return controller;
      };`;
    if (id === "\0settings-version") return 'export const getVersion = async () => "test";';
    if (id === "\0settings-head") return `
      const noop = () => {};
      export const createNestHead = actions => { globalThis.settingsHeadActions = actions; return ({ elements: [], set: id => { globalThis.settingsSelectedTab = id; }, setZeca: noop, setSound: noop, setFull: noop, setCounts: noop, relabel: noop }); };
      export const createNestFoot = () => ({ element: document.createElement("footer"), setZeca: noop, setSpecies: noop, setTerminal: noop, relabel: noop });
      export const tabByKey = () => null;`;
  },
}], build: { write: false, minify: false, lib: { entry: `${ui}src/surfaces/settings/main.ts`, formats: ["es"] } } });
const chunk = bundle[0].output.find(p => p.type === "chunk");
await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
const tick = () => new Promise(resolve => setImmediate(resolve));
events.onSettings({ dictationCommands: false, dictationCommandChat: true, dictationVocabulary: false, dictationProjectVocabulary: false, dictationMusic: "leave", dictationCancelByVoice: false, dictationSpokenCode: false, dictationCutRepeats: false, dictationRetryBase: false, dictationMicrophone: "Disconnected mic", dictationInsert: true, dictationDoubleTap: false, dictationSpaceLock: false, dictationEscapeCancel: false });
initialSettingsReady(settings);
await tick();
if (initialRoute) {
  assert.equal(root.querySelector('[data-dictation-section="commands"]')?.open, true,
    "A cold command-settings link opens its disclosure before any settings event");
  assert.equal(writes.length, 0, "Opening a linked preference never grants a capability");
  console.log("check-settings-ui: cold Windows command-settings link opens the requested disclosure without writes");
  process.exit(0);
}
const page = () => root.querySelector("main.page");
const nav = label => root.querySelectorAll("button.nav").find(n => n.textContent === label);
const go = async label => {
  if (label === "Dictation") events.onSettingsSection("dictation");
  else { if (!nav(label)) globalThis.settingsHeadActions.gear(); nav(label).fire("click"); }
  await tick();
};
const dictationPage = async id => {
  events.onSettingsSection(`dictation${id}`); await tick();
};
const detail = label => page().querySelectorAll("details").find(n => n.dataset.setting === label);
const button = label => page().querySelectorAll("button").find(n => n.textContent === label);
assert.equal(root.querySelector(".page-kicker"), null, "Settings is not repeated above the page title");
assert.equal(detail("Keyboard shortcuts").open, false);
detail("Keyboard shortcuts").open = true;
page().scrollTop = 125;
events.onSettings({ sounds: false });
assert.equal(detail("Keyboard shortcuts").open, true);
assert.equal(page().scrollTop, 125);
await go("Chat & voice");
const key = page().querySelectorAll("input").find(n => n.type === "password" && n.dataset.setting.startsWith("Example API key"));
assert.ok(key, "The existing key control remains available");
key.value = "unsaved-example"; key.focus(); key.setSelectionRange(3, 9);
detail("Your Claude Code and Codex setup").open = true;
events.onSettings({ sounds: true });
assert.equal(document.activeElement.value, "unsaved-example", "An unrelated refresh preserves the focused draft");
assert.equal(document.activeElement.selectionStart, 3);
assert.equal(document.activeElement.selectionEnd, 9);
assert.equal(detail("Your Claude Code and Codex setup").open, true);
await go("General");
assert.equal(detail("Keyboard shortcuts").open, true, "Disclosures stay open across navigation");
await go("Agents");
assert.equal(detail("What each agent can do").open, false, "The duplicate diagnostic list is disclosed on demand");
await go("History and budget");
assert.equal(page().querySelector(".week"), null);
assert.equal(page().querySelector(".activity-grid"), null, "Settings does not duplicate the weekly dashboard");
assert.ok(page().textContent.includes("Keep a history"));
button("Open Activity").fire("click"); await tick();
assert.deepEqual(routes, ["week"]);
button("Clear history…").fire("click");
assert.ok(page().textContent.includes("This can't be undone."));
button("Cancel").fire("click");
assert.deepEqual(writes, [], "Navigation, disclosures, draft restoration and cancel never write settings or clear history");
for (const label of ["Dictation", "Approvals", "Projects", "Connection setup", "Phone", "Flock & Zeca", "About"]) {
  await go(label);
  assert.equal(page().querySelectorAll("h1").length, 1, `${label} keeps one page heading`);
  assert.ok(page().querySelector(".settings-heading"));
}
await go("Connection setup");
assert.ok(page().textContent.includes("Last successful GitHub CLI account: maintainer on github.com."), "GitHub settings identify the last successful account and host from the connector poll");
const gitlabCard = page().querySelectorAll("section.card").find(card => card.querySelector(".card-title")?.textContent === "GitLab");
assert.ok(gitlabCard, "GitLab is an explicit registered connector");
assert.equal(gitlabCard.querySelector("input").checked, false);
assert.ok(gitlabCard.textContent.includes("glab auth login --hostname gitlab.com"));
assert.ok(gitlabCard.textContent.includes("Last successful GitLab CLI account: reviewer on gitlab.com."));
assert.deepEqual(writes, []);
console.log("check-settings-ui: all eleven pages, single headings, disclosed diagnostics/shortcuts, retained scroll/focus/draft, Activity routing and explicit history confirmation");

await go("General");
const soundsRow = () => page().querySelectorAll(".row").find(n => n.querySelector(".row-title")?.textContent === "Sounds");
const soundsControl = () => soundsRow().querySelector("input");
holdSettings = true;
const oldSound = soundsControl().checked;
soundsControl().checked = !oldSound; soundsControl().fire("change"); await tick();
assert.equal(pendingSettings.length, 1);
assert.equal(soundsControl().disabled, true);
assert.ok(soundsRow().textContent.includes("Saving"));
events.onSettings({ volume: 60 });
assert.equal(soundsControl().disabled, true, "A settings refresh cannot unlock a pending write");
assert.equal(soundsControl().checked, !oldSound, "A pending choice stays visible through repaint");
soundsControl().fire("change"); await tick(); assert.equal(pendingSettings.length, 1);
pendingSettings[0].reject(new Error("Cannot write settings")); await tick();
assert.equal(soundsControl().disabled, false);
assert.equal(soundsControl().checked, oldSound);
assert.ok(soundsRow().textContent.includes("Your previous choice is restored"));
soundsControl().checked = !oldSound; soundsControl().fire("change"); await tick();
pendingSettings[1].resolve(); await tick();
assert.equal(soundsControl().disabled, false);
assert.ok(soundsRow().textContent.includes("Saved."));
console.log("check-settings-ui: localized saving/saved/failure, rollback and pending guard across row repaint");

const rowNamed = name => page().querySelectorAll(".row").find(n => n.querySelector(".row-title")?.textContent === name);
const presenceButtons = () => rowNamed("Presence").querySelectorAll("button");
presenceButtons().find(n => n.textContent === "Panel").fire("click"); await tick();
assert.equal(presenceButtons().find(n => n.textContent === "Panel").getAttribute("aria-pressed"), "true");
assert.ok(presenceButtons().every(n => n.disabled));
events.onSettings({ volume: 65 });
assert.ok(presenceButtons().every(n => n.disabled));
pendingSettings.at(-1).reject(new Error("Cannot save presence")); await tick();
assert.equal(presenceButtons().find(n => n.textContent === "Island").getAttribute("aria-pressed"), "true");
assert.ok(presenceButtons().every(n => !n.disabled));
const widget = () => rowNamed("Corner widget").querySelector("select");
widget().value = "2"; widget().fire("change"); await tick();
events.onSettings({ volume: 70 });
assert.equal(widget().value, "2"); assert.equal(widget().disabled, true);
pendingSettings.at(-1).resolve(); await tick();
widget().value = "3"; widget().fire("change"); await tick();
pendingSettings.at(-1).reject(new Error("Cannot save corner")); await tick();
assert.equal(widget().value, "2", "Select rollback uses the last successful choice after a pending repaint");
assert.equal(widget().disabled, false);
console.log("check-settings-ui: segmented selection and dropdown rollback survive pending repaint");

// The real Settings entry owns budget writes; drafts and pending state survive repaint.
await go("History and budget");
const budgetInput = () => page().querySelector(".budget-input");
assert.equal(budgetInput().value, "2");
budgetInput().value = "5.25"; budgetInput().fire("input");
await go("General"); await go("History and budget");
assert.equal(budgetInput().value, "5.25", "Navigating back retains the unsaved budget");
button("Save").fire("click"); await tick();
assert.deepEqual(writes.at(-1), ["budgetSet", 525]);
assert.equal(budgetInput().disabled, true);
events.onSettings({ volume: 75 }); events.onSpend(); await tick();
assert.equal(budgetInput().value, "5.25");
assert.equal(budgetInput().disabled, true, "A spend or Settings event cannot unlock a pending save");
button("Saving…").fire("click"); await tick();
assert.equal(pendingBudgets.length, 1);
pendingBudgets[0].reject(new Error("Cannot save this budget.")); await tick();
assert.equal(budgetInput().disabled, false);
assert.equal(budgetInput().value, "5.25", "A failed budget save retains its draft");
assert.ok(page().textContent.includes("Cannot save this budget."));
button("Save").fire("click"); pendingBudgets[1].resolve(); await tick();
assert.equal(budgetInput().value, "5.25");
assert.ok(page().textContent.includes("Saved."));
const beforeInvalid = writes.length;
for (const invalid of ["0", "10000.01", "invalid"]) {
  budgetInput().value = invalid; budgetInput().fire("input"); button("Save").fire("click"); await tick();
  assert.equal(writes.length, beforeInvalid, "Invalid budgets must never reach native storage");
}
budgetInput().value = ""; budgetInput().fire("input"); button("Save").fire("click");
assert.deepEqual(writes.at(-1), ["budgetSet", null]);
pendingBudgets[2].resolve(); await tick();
assert.equal(budgetInput().value, "");
console.log("check-settings-ui: budget ownership, retained drafts, exact cents, save/retry, duplicate guard, validation and clearing");

await go("General");
const beforeExternalBudget = budgetReads;
events.onSettingsSection("activity"); await tick();
assert.equal(budgetReads, beforeExternalBudget + 1, "A native page request must load the budget just like sidebar navigation");
assert.equal(budgetInput().disabled, false);
console.log("check-settings-ui: external Activity navigation loads its budget editor");

// Initial read failures and older responses must never replace a saved budget.
const budgetBundle = await build({ configFile: false, root: ui, logLevel: "silent", build: {
  write: false, minify: false, lib: { entry: `${ui}src/surfaces/settings/budget.ts`, formats: ["es"] },
} });
const budgetChunk = budgetBundle[0].output.find(p => p.type === "chunk");
const { createBudget } = await import(`data:text/javascript;base64,${Buffer.from(budgetChunk.code).toString("base64")}`);
const reads = [], saves = [];
const editor = createBudget({
  today: () => new Promise((resolve, reject) => reads.push({ resolve, reject })),
  setBudget: cents => new Promise((resolve, reject) => saves.push({ cents, resolve, reject })),
});
editor.refresh();
const field = () => editor.element.querySelector("input");
assert.equal(field().disabled, true);
reads[0].reject("Budget unavailable."); await tick();
assert.ok(editor.element.textContent.includes("Budget unavailable."));
assert.equal(field().disabled, true, "An unreadable budget cannot appear as no limit");
editor.refresh(); editor.refresh();
reads[2].resolve({ usd: 1, tokens: 100, budget: 7, subscription: true }); await tick();
reads[1].reject("Obsolete error."); await tick();
assert.equal(field().value, "7");
assert.ok(!editor.element.textContent.includes("Obsolete error."));
editor.refresh();
field().value = "8.50"; field().fire("input");
editor.element.querySelectorAll("button").find(n => n.textContent === "Save").fire("click");
assert.equal(saves[0].cents, 850);
editor.refresh();
assert.equal(reads.length, 4, "A pending budget save cannot start another read");
saves[0].resolve(); await tick();
reads[3].resolve({ usd: 1, tokens: 100, budget: 7, subscription: true }); await tick();
assert.equal(field().value, "8.5", "A read from before the save must not restore the old limit");
console.log("check-settings-ui: initial budget read failure, stale responses and read/write ordering");

await go("General");
const themeControl = () => rowNamed("Theme").querySelector("select");
assert.equal(themeControl().dataset.savedValue, "0");
themeControl().value = "1"; themeControl().fire("change"); await tick();
assert.deepEqual(writes.at(-1), ["setTheme", "light"]); assert.equal(themeControl().disabled, true);
events.onSettings({ volume: 70 }); assert.equal(themeControl().value, "1"); assert.equal(themeControl().disabled, true);
pendingSettings.at(-1).reject(new Error("Cannot save theme")); await tick();
assert.equal(themeControl().value, "0"); assert.equal(themeControl().disabled, false);
themeControl().value = "1"; themeControl().fire("change"); await tick(); pendingSettings.at(-1).resolve(); await tick();
assert.equal(themeControl().value, "1");
assert.equal(page().querySelectorAll(".settings-group-title").length, 4);
console.log("check-settings-ui: theme pending/repaint/rollback/retry and grouped General settings");

await go("Dictation");
const musicControl = () => rowNamed("Music while you speak").querySelector("select");
assert.equal(musicControl().dataset.savedValue, "2", "Late bootstrap cannot undo the newer music choice");
assert.deepEqual(musicControl().querySelectorAll("option").map(o => o.textContent), ["Lower to 25%", "Pause it", "Leave it"]);
musicControl().value = "1"; musicControl().fire("change"); await tick();
assert.deepEqual(writes.at(-1), ["setDictationMusic", "pause"]);
const musicWrites = writes.length;
events.onSettings({ volume: 75 });
assert.equal(musicControl().disabled, true);
musicControl().fire("change"); await tick();
assert.equal(writes.length, musicWrites);
pendingSettings.at(-1).reject(new Error("Disk unavailable")); await tick();
assert.equal(musicControl().value, "2");
assert.equal(musicControl().disabled, false);
musicControl().value = "0"; musicControl().fire("change"); await tick();
assert.deepEqual(writes.at(-1), ["setDictationMusic", "lower"]);
events.onSettings({ dictationMusic: "lower" });
pendingSettings.at(-1).resolve(); await tick();
assert.equal(musicControl().value, "0");
console.log("check-settings-ui: music choice bootstrap, pending repaint, failure rollback and explicit retry");
for (const [label, key, field] of [["Help the model with these words", "vocabulary", "dictationVocabulary"], ["Project vocabulary", "projectVocabulary", "dictationProjectVocabulary"], ["Cancel by voice", "cancelByVoice", "dictationCancelByVoice"], ["Spoken code", "spokenCode", "dictationSpokenCode"], ["Remove repeated loops", "cutRepeats", "dictationCutRepeats"], ["Retry empty dictation with Base", "retryBase", "dictationRetryBase"]]) {
  await dictationPage(["vocabulary", "projectVocabulary"].includes(key) ? "words" : key === "cancelByVoice" ? "keys" : key === "spokenCode" ? "paste" : "voice");
  const control = () => rowNamed(label).querySelector("input");
  assert.equal(control().checked, false, "Late bootstrap cannot undo a newer processing choice");
  control().checked = true; control().fire("change"); await tick();
  assert.deepEqual(writes.at(-1), ["setDictationProcessing", key, true]);
  const count = writes.length;
  events.onSettings({ volume: 75 });
  assert.equal(control().disabled, true);
  control().fire("change"); await tick();
  assert.equal(writes.length, count, "A repaint cannot duplicate a processing save");
  pendingSettings.at(-1).reject(new Error("Disk unavailable")); await tick();
  assert.equal(control().checked, false);
  assert.equal(control().disabled, false);
  control().checked = true; control().fire("change"); await tick();
  events.onSettings({ [field]: true });
  pendingSettings.at(-1).resolve(); await tick();
  assert.equal(control().checked, true);
}
console.log("check-settings-ui: independent processing choices, late-bootstrap race, pending redraw, failure rollback and explicit retry");
await dictationPage("keys");
const modeControl = () => rowNamed("Recording mode").querySelector("select");
assert.equal(modeControl().dataset.savedValue, "1", "Late bootstrap cannot undo newer gesture choices");
for (const name of ["Space locks recording", "Esc cancels dictation"]) {
  const control = rowNamed(name).querySelector("input");
  assert.equal(control.checked, false);
  assert.equal(control.hasAttribute("disabled"), true, "Raw chords are unavailable on Wayland");
}
modeControl().value = "0"; modeControl().fire("change"); await tick();
assert.deepEqual(writes.at(-1), ["setDictationGesture", "doubleTap", true]);
events.onSettings({ volume: 70 });
assert.equal(modeControl().disabled, true);
modeControl().fire("change");
assert.deepEqual(writes.at(-1), ["setDictationGesture", "doubleTap", true]);
pendingSettings.at(-1).reject(new Error("Disk unavailable")); await tick();
assert.equal(modeControl().value, "1", "Failed save restores the previous mode");
modeControl().value = "0"; modeControl().fire("change"); await tick();
events.onSettings({ dictationDoubleTap: true });
pendingSettings.at(-1).resolve(); await tick();
assert.equal(modeControl().value, "0");
console.log("check-settings-ui: gesture choices, late-bootstrap guard, unavailable Wayland chords and mode save/repaint/rollback/retry");
await dictationPage("voice");
const microphoneControl = () => rowNamed("Dictation microphone").querySelector("select");
assert.equal(microphoneReads.length, 1, "Only Dictation navigation loads the input list; reentry reuses the pending read");
assert.equal(microphoneControl().disabled, true);
assert.ok(page().textContent.includes("Loading microphones"));
assert.ok(microphoneControl().textContent.includes("Disconnected mic"), "A late bootstrap preserves the newer microphone choice");
await go("General");
microphoneReads[0].reject(new Error("Audio server unavailable")); await tick();
await go("Dictation");
assert.equal(microphoneReads.length, 1, "Read failures need explicit retry");
assert.ok(page().textContent.includes("Could not list microphones"));
button("Refresh microphones").fire("click"); await tick();
button("Refresh microphones").fire("click");
assert.equal(microphoneReads.length, 2, "Pending reads cannot be duplicated");
microphoneReads[1].resolve(["USB mic"]); await tick();
assert.ok(microphoneControl().textContent.includes("Disconnected mic (unavailable)"));
assert.equal(microphoneControl().dataset.savedValue, "2", "A disconnected saved device never changes preference implicitly");
const microphoneWrites = () => writes.filter(write => write[0] === "setDictationMicrophone");
assert.equal(microphoneWrites().length, 0);
microphoneControl().value = "1"; microphoneControl().fire("change"); await tick();
assert.deepEqual(writes.at(-1), ["setDictationMicrophone", "USB mic"]);
events.onSettings({ volume: 70 });
assert.equal(microphoneControl().disabled, true);
assert.equal(button("Refresh microphones").disabled, true);
button("Refresh microphones").fire("click");
assert.equal(microphoneReads.length, 2, "Inventory cannot reorder a pending selection");
microphoneControl().fire("change"); await tick();
assert.equal(microphoneWrites().length, 1);
pendingSettings.at(-1).reject(new Error("Disk full")); await tick();
assert.equal(microphoneControl().value, "2", "A failed save restores the disconnected preference");
microphoneControl().value = "1"; microphoneControl().fire("change"); await tick();
events.onSettings({ dictationMicrophone: "USB mic" });
pendingSettings.at(-1).resolve(); await tick();
assert.equal(microphoneControl().value, "1");
assert.equal(button("Refresh microphones").disabled, false);
button("Refresh microphones").fire("click"); await tick();
microphoneReads[2].reject(new Error("Audio server disconnected")); await tick();
assert.ok(microphoneControl().textContent.includes("USB mic"), "A failed refresh retains its last list");
assert.ok(page().textContent.includes("The last list may be out of date"));
button("Refresh microphones").fire("click"); await tick();
microphoneReads[3].resolve([]); await tick();
assert.ok(page().textContent.includes("No microphones found"));
assert.ok(microphoneControl().textContent.includes("USB mic (unavailable)"));
microphoneControl().value = "0"; microphoneControl().fire("change"); await tick();
assert.deepEqual(writes.at(-1), ["setDictationMicrophone", null]);
events.onSettings({ dictationMicrophone: null }); pendingSettings.at(-1).resolve(); await tick();
assert.equal(microphoneControl().value, "0", "System default clears the saved device");
console.log("check-settings-ui: lazy microphone inventory, loading/error/stale/empty, explicit retry, disconnected preference, save rollback and default restoration");

await dictationPage("privacy");
const retentionControl = () => rowNamed("Keep dictation history").querySelector("select");
assert.equal(retentionControl().dataset.savedValue, "2");
retentionControl().value = "0"; retentionControl().fire("change"); await tick();
assert.deepEqual(writes.at(-1), ["setDictationKeep", "0"]);
assert.equal(retentionControl().disabled, true);
events.onSettings({ volume: 80 });
assert.equal(retentionControl().disabled, true, "A repaint cannot start a duplicate retention write");
pendingSettings.at(-1).reject(new Error("Erasure failed")); await tick();
assert.equal(retentionControl().value, "2");
assert.equal(retentionControl().disabled, false);
retentionControl().value = "1"; retentionControl().fire("change"); await tick();
events.onSettings({ dictationKeep: "7" });
pendingSettings.at(-1).resolve(); await tick();
assert.equal(retentionControl().value, "1");
assert.deepEqual(writes.at(-1), ["setDictationKeep", "7"]);
console.log("check-settings-ui: dictation retention, explicit choice, shared pending guard, failure rollback and native settings acknowledgement");

await dictationPage("paste");
assert.equal(rowNamed("Insert dictation in supported editors and tmux panes").querySelector("input").checked, true,
  "A delayed bootstrap cannot undo a newer insertion setting");
const beforePermission = writes.length;
events.onDictationPermission("unavailable");
assert.ok(page().textContent.includes("Keyboard access is unavailable"));
assert.equal(button("Request keyboard access").hasAttribute("disabled"), false);
assert.equal(writes.length, beforePermission, "Navigating to the page never requests desktop permission");
button("Request keyboard access").fire("click"); await tick();
assert.deepEqual(writes.at(-1), ["retryDictationPermission"]);
events.onDictationPermission("ready");
assert.equal(button("Request keyboard access").hasAttribute("disabled"), true);
const deliveryMode = () => page().querySelector(".dictation-delivery-state").dataset.deliveryMode;
assert.equal(deliveryMode(), "insertion", "Ready keyboard access permits the configured insertion mode");
const modeWrites = writes.length;
events.onDictationPermission("temporary"); assert.equal(deliveryMode(), "insertion");
events.onDictationPermission("requesting"); assert.equal(deliveryMode(), "waiting");
assert.equal(button("Request keyboard access").hasAttribute("disabled"), true);
events.onDictationPermission("unavailable"); assert.equal(deliveryMode(), "waiting");
events.onSettings({ dictationInsert: false }); assert.equal(deliveryMode(), "clipboard");
events.onSettings({ dictationInsert: true }); events.onDictationPermission("ready");
assert.equal(writes.length, modeWrites, "Mode presentation never writes settings or requests keyboard access");
events.onShortcutKeys({ talk: "Meta+V", "talk-again": "Meta+B", "talk-cancel": "Meta+X", "talk-command": "Meta+Z" });
await dictationPage("keys");
assert.ok(page().textContent.includes("Meta+V") && page().textContent.includes("Meta+B") && page().textContent.includes("Meta+X"));
assert.ok(rowNamed("Voice command").textContent.includes("Meta+Z"), "The shortcut page separates dictation and local commands");
const x11Keys = () => page().querySelector('[data-setting="dictation-x11-keys"]');
assert.equal(x11Keys().open, false, "Platform-specific gestures start disclosed");
x11Keys().open = true; events.onSettings({ volume: 80 });
assert.equal(x11Keys().open, true, "A settings repaint keeps the keyboard disclosure choice");
await dictationPage("commands");
assert.ok(rowNamed("Command key").textContent.includes("Meta+Z"), "Dictation shows the command key's actual desktop binding");
assert.equal(page().querySelector(".voice-command-check"), null, "Command testing belongs to the work page, not preferences");
await dictationPage("keys");
assert.equal(pendingShortcutOpens.length, 0, "Entering Settings never launches a desktop editor");
button("Change in KDE…").fire("click");
assert.equal(pendingShortcutOpens.length, 1);
assert.ok(page().querySelectorAll('[role="status"]').some(n => n.textContent.includes("Opening KDE Shortcuts")));
events.onSettings({ volume: 60 });
assert.equal(button("Change in KDE…").disabled, true);
button("Change in KDE…").fire("click");
assert.equal(pendingShortcutOpens.length, 1, "Repainting cannot duplicate a pending launch");
await go("General");
pendingShortcutOpens[0].reject(new Error("Desktop entry missing")); await tick();
await go("Dictation"); await dictationPage("keys");
assert.ok(page().querySelector('[role="alert"]').textContent.includes("Could not open KDE Shortcuts"));
button("Change in KDE…").fire("click");
assert.ok(!page().querySelectorAll('[role="alert"]').some(n => n.textContent.includes("Could not open KDE Shortcuts")), "Retry clears the previous shortcut failure");
pendingShortcutOpens[1].resolve(); await tick();
assert.equal(button("Change in KDE…").disabled, false);
assert.ok(!page().textContent.includes("Opening KDE Shortcuts"));
assert.ok(page().textContent.includes("Meta+V"), "Launching the editor never guesses a new binding");
console.log("check-settings-ui: explicit KDE shortcut launch, retained pending state, failure/retry and no duplicate launch");
await dictationPage("privacy");
button("Erase dictation history…").fire("click");
const confirmation = () => page().querySelectorAll('[role="group"]').find(n => n.getAttribute("aria-label") === "Erase dictation history");
assert.ok(confirmation());
assert.equal(document.activeElement.dataset.setting, "Cancel dictation erasure", "Privacy deletion starts on its safe alternative");
confirmation().fire("keydown", { key: "Escape", preventDefault() {} });
assert.equal(pendingErases.length, 0);
assert.equal(confirmation(), undefined, "Escape cancels the confirmation without erasing");
assert.equal(document.activeElement.dataset.dictationEraseRequest, "", "Escape returns to the erasure request");
button("Erase dictation history…").fire("click"); button("Erase").fire("click");
assert.equal(pendingErases.length, 1);
events.onSettings({ volume: 65 });
assert.equal(button("Erase").disabled, true);
button("Erase").fire("click"); assert.equal(pendingErases.length, 1, "A repaint cannot duplicate erasure");
pendingErases[0].reject(new Error("Disk unavailable")); await tick();
assert.ok(page().textContent.includes("Could not erase dictation history"));
assert.ok(!page().textContent.includes("Dictation history erased"));
assert.equal(document.activeElement.dataset.setting, "Cancel dictation erasure", "Failed deletion restores safe focus while preserving retry");
button("Erase").fire("click");
rowNamed("Keep dictation history").querySelector("select").focus();
pendingErases[1].resolve(); await tick();
assert.equal(document.activeElement.dataset.setting, "Keep dictation history", "A completed deletion does not steal a newer focus");
assert.ok(page().textContent.includes("Dictation history erased. Usage totals stay."));
assert.equal(confirmation(), undefined);
globalThis.settingsHeadActions.tab("dictation"); await tick();
assert.equal(routes.at(-1), "dictation", "The Dictation tab returns to its work webview");
await go("Chat & voice");
assert.equal(rowNamed("Language you speak"),undefined,"Chat links to the shared voice editor instead of duplicating it");
button("Dictation settings").fire("click"); assert.equal(page().querySelector("h1").textContent, "Dictation");
assert.equal(globalThis.settingsSelectedTab,"settings");
assert.ok(root.querySelector(".settings-page-picker"), "Global Settings navigation stays available");
await go("General");
assert.equal(rowNamed("Keep dictation history"), undefined, "General links to the single settings owner");
assert.ok(root.querySelectorAll("button.nav").some(n=>n.textContent==="Dictation"),"Dictation settings are discoverable in the global Settings sidebar");
button("Dictation settings").fire("click"); assert.equal(globalThis.settingsSelectedTab,"settings");
await dictationPage("commands");
assert.equal(writes.filter(write => write[0] === "dictationClear").length, 2);
console.log("check-settings-ui: dedicated Dictation navigation/keys, shared model link, late-settings race, explicit portal retry and erase confirmation/failure/retry without duplicate writes");

const commandSwitch = () => rowNamed("Enable voice commands").querySelector("input");
const fallback = () => rowNamed("When it is not a command").querySelector("select");
assert.equal(commandSwitch().checked, false, "A late bootstrap cannot undo a newer command choice");
assert.ok(page().textContent.includes("Voice commands are off. Dictation keeps its own shortcut."));
assert.equal(fallback().disabled, true);
assert.equal(fallback().value, "1", "Disabled commands preserve the separately chosen fallback");
commandSwitch().checked = true; commandSwitch().fire("change"); await tick();
assert.deepEqual(writes.at(-1), ["setDictationCommands", "enabled", true]);
events.onSettings({ volume: 60 });
assert.equal(commandSwitch().disabled, true);
pendingSettings.at(-1).reject(new Error("Disk unavailable")); await tick();
assert.equal(commandSwitch().checked, false);
assert.equal(fallback().disabled, true);
commandSwitch().checked = true; commandSwitch().fire("change"); await tick();
events.onSettings({ dictationCommands: true });
pendingSettings.at(-1).resolve(); await tick();
assert.equal(fallback().disabled, false);
assert.ok(page().textContent.includes("Unrecognized phrases may be sent to the current Chat."));
fallback().value = "0"; fallback().fire("change"); await tick();
assert.deepEqual(writes.at(-1), ["setDictationCommands", "chat", false]);
events.onSettings({ volume: 65 });
assert.equal(fallback().disabled, true);
pendingSettings.at(-1).reject(new Error("Disk unavailable")); await tick();
assert.equal(fallback().value, "1", "A failed fallback change restores the explicit saved grant");
fallback().value = "0"; fallback().fire("change"); await tick();
events.onSettings({ dictationCommandChat: false });
pendingSettings.at(-1).resolve(); await tick();
assert.equal(fallback().value, "0");
assert.ok(page().textContent.includes("Unrecognized phrases stay local."));
assert.ok(page().textContent.includes("Refused commands and ambiguous names stay local"));
assert.ok(!page().textContent.includes("What you can say"), "Examples remain in the command work page");
const outputSwitch = () => rowNamed("System audio voice commands").querySelector("input");
assert.equal(outputSwitch().checked, false, "System audio has its own default-off grant");
outputSwitch().checked = true; outputSwitch().fire("change"); await tick();
assert.deepEqual(writes.at(-1), ["setDictationCommands", "output", true]);
events.onSettings({ volume: 70 });
assert.equal(outputSwitch().disabled, true, "A repaint cannot unlock a pending system-audio grant");
pendingSettings.at(-1).reject(new Error("Disk unavailable")); await tick();
assert.equal(outputSwitch().checked, false, "A failed save cannot grant output control");
outputSwitch().checked = true; outputSwitch().fire("change"); await tick();
events.onSettings({ dictationCommandOutput: true });
pendingSettings.at(-1).resolve(); await tick();
assert.equal(outputSwitch().checked, true);
assert.equal(rowNamed("Media voice commands").querySelector("input").checked, false);
assert.equal(rowNamed("Application voice commands").querySelector("input").checked, false);
console.log("check-settings-ui: voice command opt-in, late bootstrap, pending repaints, failed saves, retry, binding and shared phrase guide");

events.onSettingsSection("dictationstyles"); await tick();
assert.equal(page().querySelector("h1").textContent, "Styles");
assert.equal(globalThis.settingsSelectedTab, "dictation", "Styles belongs to the Dictation work area");
const navigation = root.querySelector(".dictation-nav");
const navItems = () => navigation.querySelectorAll("[data-dictation-page]");
assert.deepEqual(navItems().map(n => n.dataset.dictationPage), ["history", "commands", "conversation", "meetings", "dictionary", "snippets", "dictationstyles", "dictationdiagnostics"]);
assert.equal(root.querySelector(".sidebar"), null, "A tool does not gain a second navigation");
assert.ok(!navItems().some(n => n.dataset.dictationPage === "statistics"));
const navGroup = navigation.querySelector(".dictation-groups");
navItems()[0].focus(); navGroup.fire("keydown", { key: "End", preventDefault() {} });
assert.equal(document.activeElement, navItems().at(-1));
navGroup.fire("keydown", { key: "ArrowDown", preventDefault() {} });
assert.equal(document.activeElement, navItems()[0]);
const picker = root.querySelector(".dictation-page-picker");
picker.value = "dictationdiagnostics"; picker.onchange(); await tick();
assert.equal(page().querySelector("h1").textContent, "Diagnostics", "The compact picker opens Diagnostics in Dictation");
assert.equal(diagnosticReads, 1);
button("Test microphone").fire("click"); await tick();
await dictationPage("privacy");
micStartReady(42); await tick();
assert.deepEqual(stoppedMics, [42], "Leaving Diagnostics cancels a late microphone start");
assert.equal(page().querySelector("h1").textContent, "Dictation");
assert.equal(globalThis.settingsSelectedTab, "settings");
assert.equal(root.querySelector(".dictation-nav"), null, "Preferences have no nested page navigation");
assert.equal(root.querySelector(".settings-section-picker"), null);
assert.ok(root.querySelector(".sidebar"));
for (const id of ["voice", "keys", "paste", "privacy", "words", "commands", "conversation", "meetings", "agents"])
  assert.ok(page().querySelectorAll("[data-dictation-section]").some(n => n.dataset.dictationSection === id), `${id} exists on the same preference page`);
assert.ok(!page().textContent.includes("Microphone test"));
page().scrollTop = 245;
const retentionFocus = rowNamed("Keep dictation history").querySelector("select"); retentionFocus.focus();
events.onSettings({ volume: 72 }); await tick();
assert.equal(page().scrollTop, 245, "A preference refresh retains reading position");
assert.equal(document.activeElement.dataset.setting, "Keep dictation history");
await go("General"); nav("Dictation").fire("click"); await tick();
assert.equal(page().querySelector("h1").textContent, "Dictation", "The global Settings destination always opens preferences");
console.log("check-settings-ui: one preference page, eight work destinations, Styles/Diagnostics ownership, compact picker, keyboard movement, retained focus/scroll and late microphone cancellation");
events.onSettingsSection("dictationvoice"); await tick();
for (const lang of ["pt-BR", "es", "zh"]) {
  events.onSettings({ lang });
  for (const english of ["Base: fast, but weak outside English", "Small: good in Portuguese and other languages", "Large v3 Turbo: the most accurate; best with a GPU"])
    assert.ok(!page().textContent.includes(english), "Known model descriptions use the active UI language");
}
events.onSettings({lang:"en"});assert.ok(page().textContent.includes("Base: fast, but weak outside English"));
console.log("check-settings-ui: retained Dictation configuration and localized voice-model descriptions");

await go("General");
const widthPick = () => rowNamed("Mini and Quick width").querySelector("select");
assert.equal(widthPick().value, "0");
const priorWrites = writes.length;
widthPick().value = "2"; widthPick().fire("change"); await tick();
assert.equal(widthStorage.get("island-width"), "680");
assert.equal(writes.length, priorWrites, "Appearance width uses shared presentation state, not privileged config writes");
widthStorage.set("island-width", "760");
for (const fn of windowEvents.storage ?? []) fn({ key: "island-width" });
assert.equal(widthPick().value, "3", "Compact changes appear in General");
const saveWidth = localStorage.setItem;
localStorage.setItem = () => { throw new Error("full"); };
widthPick().value = "1"; widthPick().fire("change"); await tick();
assert.equal(widthPick().value, "3", "A failed save rolls back to the shared saved width");
assert.equal(widthStorage.get("island-width"), "760");
localStorage.setItem = saveWidth;
widthStorage.delete("island-width");
for (const fn of windowEvents.storage ?? []) fn({ key: "island-width" });
assert.equal(widthPick().value, "0");
console.log("check-settings-ui: shared General/Compact width, cross-view refresh, Default and failed-save rollback");

globalThis.settingsTestWindowsBindings = { requested: {}, defaults: {}, active: {}, conflicts: [] };
await globalThis.settingsTestWindowsController.refresh();
await go("Dictation");
events.onSettings({ dictationMusic: "lower" }); await tick();
const beforeWindowsMusic = writes.length;
assert.deepEqual(musicControl().querySelectorAll("option").map(o => o.textContent), ["Pause it", "Leave it"]);
assert.equal(musicControl().value, "1", "A saved Linux Lower preference is inactive on Windows");
assert.equal(writes.length, beforeWindowsMusic, "Showing the effective default must not rewrite Linux preferences");
musicControl().value = "0"; musicControl().fire("change"); await tick();
assert.deepEqual(writes.at(-1), ["setDictationMusic", "pause"]);
pendingSettings.at(-1).reject(new Error("Disk unavailable")); await tick();
assert.equal(musicControl().value, "1", "An unsuccessful save must not enable Windows media pauses");
musicControl().value = "0"; musicControl().fire("change"); await tick();
pendingSettings.at(-1).resolve(); await tick();
assert.equal(musicControl().value, "0");
assert.equal(rowNamed("Media voice commands").querySelector("input").checked, false);
console.log("check-settings-ui: Windows pause opt-in, preserved Linux default, failed save and explicit retry");
await dictationPage("commands");
events.onSettings({ dictationCommandOutput: false }); await tick();
assert.ok(rowNamed("System audio voice commands"), "Windows exposes the independent output grant");
const appSwitch = () => rowNamed("Application voice commands").querySelector("input");
assert.ok(rowNamed("Application voice commands").textContent.includes("Microsoft Store"), "Windows explains both registered app sources");
events.onSettings({ dictationCommandApps: false }); await tick();
assert.equal(appSwitch().checked, false, "Windows app launch has a separate default-off grant");
appSwitch().checked = true; appSwitch().fire("change"); await tick();
assert.deepEqual(writes.at(-1), ["setDictationCommands", "applications", true]);
pendingSettings.at(-1).reject(new Error("Disk unavailable")); await tick();
assert.equal(appSwitch().checked, false, "Failed saving cannot grant app launches");
appSwitch().checked = true; appSwitch().fire("change"); await tick();
pendingSettings.at(-1).resolve(); await tick();
assert.equal(appSwitch().checked, true);
assert.equal(outputSwitch().checked, false, "App permission does not grant computer audio");
assert.equal(rowNamed("Media voice commands").querySelector("input").checked, false);
assert.equal(outputSwitch().checked, false);
outputSwitch().checked = true; outputSwitch().fire("change"); await tick();
assert.deepEqual(writes.at(-1), ["setDictationCommands", "output", true]);
pendingSettings.at(-1).reject(new Error("Disk unavailable")); await tick();
assert.equal(outputSwitch().checked, false, "A failed Windows save cannot enable output commands");
outputSwitch().checked = true; outputSwitch().fire("change"); await tick();
pendingSettings.at(-1).resolve(); await tick();
assert.equal(outputSwitch().checked, true);
assert.equal(rowNamed("Media voice commands").querySelector("input").checked, false);
console.log("check-settings-ui: Windows output opt-in is visible, independent, persisted and failure-safe");
const cold = spawnSync(process.execPath, [fileURLToPath(import.meta.url)], {
  env: { ...process.env, VULTS_SETTINGS_INITIAL_ROUTE: "dictationcommands" }, windowsHide: true, encoding: "utf8",
});
assert.equal(cold.status, 0, cold.stderr || cold.stdout);
process.stdout.write(cold.stdout);
// The Windows mixer picker never enumerates until the user asks, and drops late replies.
const audioBundle = await build({ configFile: false, root: ui, logLevel: "silent", build: {
  write: false, minify: false, lib: { entry: `${ui}src/surfaces/settings/windows-player-audio.ts`, formats: ["es"] },
} });
const audioChunk = audioBundle[0].output.find(p => p.type === "chunk");
const { createWindowsPlayerAudio } = await import(`data:text/javascript;base64,${Buffer.from(audioChunk.code).toString("base64")}`);
const audioReads = [], audioWrites = [];
let audioView;
const audioPicker = createWindowsPlayerAudio({
  status: async () => null,
  list: () => new Promise((resolve, reject) => audioReads.push({ resolve, reject })),
  select: token => new Promise((resolve, reject) => audioWrites.push({ token, resolve, reject })),
}, () => { audioView = audioPicker.render(); });
audioPicker.context(true, false); audioView = audioPicker.render();
assert.equal(audioView.querySelector("button").disabled, true);
assert.equal(audioReads.length, 0);
audioPicker.context(true, true); audioView = audioPicker.render();
audioView.querySelector("button").fire("click"); await tick();
assert.equal(audioReads.length, 1);
assert.equal(audioView.querySelector("button").disabled, true);
const audioCandidate = { token: "1:0", app: "test.exe", output: "Speakers", pid: 42, percent: 50 };
audioReads[0].resolve([audioCandidate]); await tick();
assert.equal(audioView.querySelector("select").disabled, false);
audioPicker.refreshStatus(); await tick();
assert.equal(audioView.querySelector("select").children.length, 2, "Focus must preserve explicit inventory awaiting a choice");
let audioSelect = audioView.querySelector("select"); audioSelect.value = "1:0"; audioSelect.fire("change"); await tick();
assert.equal(audioWrites.at(-1).token, "1:0");
audioWrites.at(-1).reject("Session changed"); await tick();
assert.equal(audioView.querySelector("select").value, "");
assert.ok(audioView.textContent.includes("Refresh and choose again"));
audioView.querySelector("button").fire("click"); await tick();
audioPicker.context(false, true);
assert.equal(audioWrites.at(-1).token, null, "Leaving cancels native association work");
audioWrites.at(-1).resolve();
audioReads[1].resolve([audioCandidate]); await tick();
audioPicker.context(true, true); audioView = audioPicker.render();
assert.equal(audioView.querySelector("select").disabled, true, "Late inventory must be discarded");
audioView.querySelector("button").fire("click"); await tick();
audioPicker.context(true, false); audioReads[2].resolve([audioCandidate]); await tick();
audioView = audioPicker.render();
assert.equal(audioView.querySelector("select").disabled, true, "Revocation discards pending inventory");
assert.equal(audioWrites.filter(write => write.token !== null).length, 1, "No volume or target action is implicit");
console.log("check-settings-ui: Windows mixer selection is explicit, gated and rejects stale replies");
const statusReads = [];
let restoredView;
const restoredAudio = createWindowsPlayerAudio({
  status: () => new Promise(resolve => statusReads.push(resolve)),
  list: async () => { throw Error('No implicit inventory'); },
  select: async () => { throw Error('No implicit association'); },
}, () => { restoredView = restoredAudio.render(); });
restoredAudio.context(true, true);
statusReads[0](audioCandidate); await tick();
assert.equal(restoredView.querySelector('select').value, audioCandidate.token);
restoredAudio.context(false, true); restoredAudio.context(true, true);
statusReads[1](audioCandidate); await tick();
assert.equal(restoredView.querySelector('select').value, audioCandidate.token, 'A completed association survives navigation');
restoredAudio.context(false, true); restoredAudio.context(true, true);
restoredAudio.changed(); statusReads[2](audioCandidate); await tick();
assert.equal(restoredView.querySelector('select').value, '', 'Invalidated status cannot restore an old association');
console.log('check-settings-ui: completed mixer associations restore without inventory and reject obsolete status');
// Empty/error status is authoritative, and older focus reads cannot restore a stale link.
restoredAudio.context(false, true); restoredAudio.context(true, true);
statusReads[3](audioCandidate); await tick();
restoredAudio.refreshStatus(); restoredAudio.refreshStatus();
statusReads[5](null); await tick();
assert.equal(restoredView.querySelector('select').value, '');
assert.ok(restoredView.textContent.includes('Refresh and choose again'));
statusReads[4](audioCandidate); await tick();
assert.equal(restoredView.querySelector('select').value, '', 'Older focus read cannot restore an expired link');
restoredAudio.context(false, true); restoredAudio.context(true, true);
restoredAudio.context(false, true); restoredAudio.context(true, true);
statusReads[7](null); await tick(); statusReads[6](audioCandidate); await tick();
assert.equal(restoredView.querySelector('select').value, '', 'Older visibility read cannot replace newer empty status');
let failedAudioView;
const failedAudio = createWindowsPlayerAudio({ status: async () => { throw Error('Native service failed'); }, list: async () => [], select: async () => {} }, () => { failedAudioView = failedAudio.render(); });
failedAudio.context(true, true); await tick();
assert.equal(failedAudioView.querySelector('select').value, '');
assert.ok(failedAudioView.textContent.includes('Could not verify the audio link'));
console.log('check-settings-ui: expired/failed mixer status clears stale choices and focus/visibility reads have distinct ownership');

const giteaBundle = await build({ configFile: false, root: ui, logLevel: "silent", build: {
  write: false, minify: false, lib: { entry: `${ui}src/surfaces/settings/gitea.ts`, formats: ["es"] },
} });
const giteaChunk = giteaBundle[0].output.find(p => p.type === "chunk");
const { createGitea } = await import(`data:text/javascript;base64,${Buffer.from(giteaChunk.code).toString("base64")}`);
let finishGitea, failGitea;
const giteaWrites = [];
const gitea = createGitea({ read: async () => ({baseUrl:'https://git.example.com/team/',generation:'first'}), save: (...args) => { giteaWrites.push(args); return new Promise((resolve,reject) => { finishGitea=resolve; failGitea=reject; }); } });
gitea.context(true,false); await tick();
const giteaFields=nodes(gitea.element).filter(n => n.tagName==='INPUT');
const [giteaUrl,giteaToken]=giteaFields;
const giteaButtons=nodes(gitea.element).filter(n => n.tagName==='BUTTON');
const [giteaSave,giteaClear]=giteaButtons;
assert.equal(giteaToken.type,'password');
assert.equal(giteaUrl.value,'https://git.example.com/team/');
giteaToken.value='synthetic-token'; gitea.context(true,false);
assert.equal(giteaToken.value,'synthetic-token','Routine Settings refresh retains the draft');
giteaSave.fire('click'); giteaSave.fire('click'); await tick();
assert.equal(giteaWrites.length,1,'One save at a time');
assert.deepEqual(giteaWrites[0],['https://git.example.com/team/','synthetic-token','first',false]);
assert.equal(giteaToken.value,'','Token is cleared as soon as native saving begins');
assert.ok(giteaSave.disabled);
finishGitea({baseUrl:'https://git.example.com/team/',generation:'second'}); await tick();
assert.ok(gitea.element.textContent.includes('Turn on Gitea separately'));
giteaToken.value='another-token'; gitea.context(false,false);
assert.equal(giteaToken.value,'','Leaving the page clears credentials');
gitea.context(true,true);
assert.ok(giteaUrl.disabled && giteaToken.disabled && giteaSave.disabled && giteaClear.disabled);
giteaClear.fire('click'); await tick(); assert.equal(giteaWrites.length,1,'Enabled connections cannot be edited');
gitea.context(true,false); giteaToken.value='retry-token'; giteaSave.fire('click'); await tick();
failGitea('Could not save the Gitea connection.'); await tick();
assert.ok(gitea.element.textContent.includes('Could not save the Gitea connection.'));
assert.equal(giteaToken.value,'');
giteaClear.fire('click'); await tick(); assert.equal(giteaWrites.length,2,'Remove connection asks before removing');
assert.equal(document.activeElement.textContent,'Cancel','Cancel takes the focus while removal is asked');
document.activeElement.fire('click'); await tick(); assert.equal(giteaWrites.length,2,'Cancel removes nothing');
giteaClear.fire('click'); giteaClear.fire('click'); await tick(); assert.deepEqual(giteaWrites[2],['https://git.example.com/team/','','second',true]);
finishGitea({baseUrl:'',generation:''}); await tick();
assert.equal(giteaUrl.value,''); assert.ok(giteaClear.disabled);
console.log('check-settings-ui: Gitea keeps drafts, clears tokens, serializes saves and requires a separate activation');

// A list that cannot be read says so, with a retry, instead of looking empty or off.
for (const [label, call, failed] of [
  ["Projects", "projectsList", "Could not read your project choices."],
  ["Phone", "phoneSettings", "Could not read the phone settings."],
  ["Connection setup", "connectorsStatus", "Could not read which connections are on."],
]) {
  globalThis.failReads = new Set([call]);
  await go(label); await tick(); await tick();
  assert.ok(page().textContent.includes(failed), `${label} says its read failed`);
  const retryButton = page().querySelectorAll("button").find(b => b.textContent === "Try again");
  assert.ok(retryButton, `${label} offers Try again`);
  globalThis.failReads = new Set();
  retryButton.fire("click"); await tick(); await tick();
  assert.ok(!page().textContent.includes(failed), `${label} recovers on Try again`);
}
console.log('check-settings-ui: Projects, Phone and Connections show a failed read with a retry and recover');

// A removal asks in place: the first click runs nothing, Cancel and Escape back out, the second click runs once.
const piecesBundle = await build({ configFile: false, root: ui, logLevel: "silent", build: {
  write: false, minify: false, lib: { entry: `${ui}src/surfaces/settings/pieces.ts`, formats: ["es"] },
} });
const piecesChunk = piecesBundle[0].output.find(part => part.type === "chunk");
const { confirmed } = await import(`data:text/javascript;base64,${Buffer.from(piecesChunk.code).toString("base64")}`);
let removals = 0, finishRemoval;
const removal = confirmed("Remove", "This permission asks again on its next request.", "Remove permission", () => { removals++; return new Promise(resolve => { finishRemoval = resolve; }); });
document.body.append(removal);
const removalButtons = () => removal.querySelectorAll("button");
assert.deepEqual(removalButtons().map(b => b.textContent), ["Remove"]);
removalButtons()[0].fire("click");
assert.equal(removals, 0, "The first click only asks");
assert.deepEqual(removalButtons().map(b => b.textContent), ["Cancel", "Remove permission"]);
assert.equal(document.activeElement.textContent, "Cancel", "Cancel takes the focus, so a held Enter cannot confirm");
assert.ok(removal.textContent.includes("This permission asks again on its next request."));
assert.ok(removalButtons()[1].getAttribute("aria-describedby"), "The removal is described by its effect");
removalButtons()[0].fire("click");
assert.equal(removals, 0); assert.deepEqual(removalButtons().map(b => b.textContent), ["Remove"]);
assert.equal(document.activeElement.textContent, "Remove", "Cancel returns the focus to the action");
removalButtons()[0].fire("click"); removal.fire("keydown", { key: "Escape" });
assert.equal(removals, 0); assert.deepEqual(removalButtons().map(b => b.textContent), ["Remove"]);
removal.fire("keydown", { key: "Escape" });
removalButtons()[0].fire("click"); const armed = removalButtons()[1];
armed.fire("click"); armed.fire("click"); await tick();
assert.equal(removals, 1, "A repeated click cannot run the removal twice");
assert.ok(removalButtons().every(b => b.disabled), "Both buttons wait for the removal");
finishRemoval(); await tick(); await tick();
assert.deepEqual(removalButtons().map(b => b.textContent), ["Remove"]);
console.log('check-settings-ui: a removal asks first, cancels by button or Escape and runs once');
