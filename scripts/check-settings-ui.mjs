// Exercise the real Settings entry with local native responses; no browser server or writes.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";
const nodes = node => [node, ...node.children.flatMap(nodes)];
class Element {
  children = []; listeners = {}; attributes = {}; dataset = {}; style = { setProperty() {} };
  className = ""; value = ""; open = false; scrollTop = 0; selectionStart = null; selectionEnd = null;
  constructor(tag) { this.tagName = tag.toUpperCase(); }
  classList = {
    contains: name => this.className.split(" ").includes(name),
    add: name => { if (!this.classList.contains(name)) this.className += ` ${name}`; },
    remove: name => { this.className = this.className.split(" ").filter(v => v !== name).join(" "); },
    toggle: (name, on = !this.classList.contains(name)) => this.classList[on ? "add" : "remove"](name),
  };
  set textContent(text) { this.replaceChildren(); this.text = text; }
  get textContent() { return (this.text ?? "") + this.children.map(n => n.textContent).join(""); }
  append(...children) { for (const node of children) { node.remove(); node.parentElement = this; this.children.push(node); } }
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
        const val = attr[1].startsWith("data-") ? this.dataset[attr[1].slice(5)] : this.attributes[attr[1]];
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
  getAttribute(name) { return this.attributes[name]; }
  addEventListener(name, fn) { (this.listeners[name] ??= []).push(fn); }
  fire(name, event = {}) { for (const fn of this.listeners[name] ?? []) fn({ target: this, ...event }); }
  focus() { document.activeElement = this; }
  setSelectionRange(start, end) { this.selectionStart = start; this.selectionEnd = end; }
  getContext() { return { clearRect() {}, fillRect() {} }; }
}
const root = new Element("div"), events = {}, writes = [], routes = [];
root.id = "settings";
globalThis.document = { body: new Element("body"), documentElement: new Element("html"),
  getElementById: () => root, createElement: tag => new Element(tag),
  createElementNS: (_ns, tag) => new Element(tag),
  createTextNode: text => { const n = new Element("text"); n.textContent = text; return n; },
  addEventListener() {}, hidden: false,
};
document.body.append(root);
globalThis.window = { setInterval: () => 0, clearInterval() {}, setTimeout: () => 0, clearTimeout() {}, addEventListener() {} };
globalThis.matchMedia = () => ({ matches: true });
globalThis.location = { hash: "#general" };
globalThis.Option = class extends Element {
  constructor(label, value, _default, selected) { super("option"); this.textContent = label; this.value = value; this.selected = selected; }
};
const settings = { sounds: true, volume: 50, foldAfter: 15, lang: "en", language: null, zeca: true,
  zecaSpecies: "atratus", zecaLook: "auto", flock: "brazil", nativeChat: true, chatAccess: "ask", memory: true,
  presence: "island", settingsPath: "/home/example/settings.json", dataPath: "/home/example/vults" };
globalThis.settingsTestBridge = new Proxy({}, { get: (_obj, name) => {
  if (name.startsWith("on")) return cb => { events[name] = cb; };
  return async (...args) => {
    switch (name) {
      case "appSettings": return settings;
      case "nestIsFull": return false;
      case "monitors": case "connectorsStatus": case "rulesList": case "policiesList": case "agentsDoctor": return [];
      case "projectPrefs": return {};
      case "installStatus": return { installed: false, hookReady: true, statusLine: "ours", configPath: "/home/example/agent.json" };
      case "apiProviders": return { selected: "example", providers: [{ id: "example", label: "Example", local: false, hasKey: false }] };
      case "webSettings": return { on: false, engine: "brave", hasKey: false };
      case "voiceStatus": return { ready: false, models: [], downloading: [], language: null };
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
    if (id.endsWith("/nest/head")) return "\0settings-head";
    if (id === "@tauri-apps/api/app") return "\0settings-version";
  }, load(id) {
    if (id === "\0settings-bridge") return "export const Bridge = globalThis.settingsTestBridge;";
    if (id === "\0settings-version") return 'export const getVersion = async () => "test";';
    if (id === "\0settings-head") return `
      const noop = () => {};
      export const createNestHead = () => ({ elements: [], set: noop, setZeca: noop, setSound: noop, setFull: noop, setCounts: noop, relabel: noop });
      export const createNestFoot = () => ({ element: document.createElement("footer"), setZeca: noop, setSpecies: noop, setTerminal: noop, relabel: noop });
      export const tabByKey = () => null;`;
  },
}], build: { write: false, minify: false, lib: { entry: `${ui}src/surfaces/settings/main.ts`, formats: ["es"] } } });
const chunk = bundle[0].output.find(p => p.type === "chunk");
await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
const tick = () => new Promise(resolve => setImmediate(resolve));
await tick();
const page = () => root.querySelector("main.page");
const nav = label => root.querySelectorAll("button.nav").find(n => n.textContent === label);
const go = async label => { nav(label).fire("click"); await tick(); };
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
await go("Activity");
assert.equal(page().querySelector(".week"), null);
assert.equal(page().querySelector(".activity-grid"), null, "Settings does not duplicate the weekly dashboard");
assert.ok(page().textContent.includes("Keep a history"));
button("Open Activity").fire("click"); await tick();
assert.deepEqual(routes, ["week"]);
button("Clear history…").fire("click");
assert.ok(page().textContent.includes("This can't be undone."));
button("Cancel").fire("click");
assert.deepEqual(writes, [], "Navigation, disclosures, draft restoration and cancel never write settings or clear history");
for (const label of ["Approvals", "Projects", "Connectors", "Phone", "Flock & Zeca", "About"]) {
  await go(label);
  assert.equal(page().querySelectorAll("h1").length, 1, `${label} keeps one page heading`);
  assert.ok(page().querySelector(".settings-heading"));
}
assert.deepEqual(writes, []);
console.log("check-settings-ui: all ten pages, single headings, disclosed diagnostics/shortcuts, retained scroll/focus/draft, Activity routing and explicit history confirmation");
