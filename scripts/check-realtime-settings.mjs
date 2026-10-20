// Exercise the production bridge without a desktop, microphone, keyring or provider.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build, normalizePath } from "vite";

const root = normalizePath(fileURLToPath(new URL("../ui/", import.meta.url)));
const calls = [];
const saved = { on: false, model: "gpt-realtime-2.1-mini", limitMicrousd: "9223372036854775807" };
globalThis.realtimeSettingsInvoke = async (command, args) => {
  calls.push({ command, args });
  if (command === "voice_realtime_settings") return saved;
};
const bundles = await build({ configFile: false, root, logLevel: "silent", plugins: [{
  name: "realtime-settings-boundary",
  enforce: "pre",
  resolveId(id) { if (id.startsWith("@tauri-apps/api/")) return `\0${id}`; },
  load(id) {
    if (id === "\0@tauri-apps/api/core") return "export const invoke = globalThis.realtimeSettingsInvoke;";
    if (id === "\0@tauri-apps/api/event") return "export const emitTo = () => {}; export const listen = () => {};";
    if (id === "\0@tauri-apps/api/window") return "export const getCurrentWindow = () => ({});";
  },
}], build: { write: false, minify: false, lib: { entry: `${root}src/bridge.ts`, formats: ["es"] } } });
try {
  const chunk = bundles[0].output.find(part => part.type === "chunk");
  const { Bridge } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
  assert.equal(calls.length, 0, "Importing the bridge cannot read settings or start voice");
  assert.deepEqual(await Bridge.voiceRealtimeSettings(), saved);
  assert.deepEqual(calls.pop(), { command: "voice_realtime_settings", args: undefined });
  for (const limit of [NaN, Infinity, -1, 0.5, 1000000, null, "00", "+1", "-1", "0.5", "9223372036854775808"]) {
    await assert.rejects(Bridge.setVoiceRealtime(true, limit, saved.model));
    assert.equal(calls.length, 0, "Invalid or rounded budgets cannot reach native consent");
  }
  await Bridge.setVoiceRealtime(true, "1000000", saved.model);
  assert.deepEqual(calls.pop(), { command: "set_voice_realtime", args: { on: true, limitMicrousd: "1000000", model: saved.model } });
  await Bridge.setVoiceRealtime(false, "9223372036854775807", saved.model);
  assert.deepEqual(calls.pop(), { command: "set_voice_realtime", args: { on: false, limitMicrousd: "9223372036854775807", model: saved.model } });
  assert.equal(calls.length, 0);
  console.log("check-realtime-settings: exact readback, explicit writes and lossless decimal budgets without capture");
} finally {
  delete globalThis.realtimeSettingsInvoke;
}

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
  fire(name, event = {}) { for (const fn of this.listeners[name] ?? []) fn({ target: this, ...event }); }
  focus() { document.activeElement = this; }
  setSelectionRange(start, end) { this.selectionStart = start; this.selectionEnd = end; }
  getContext() { return { clearRect() {}, fillRect() {} }; }
  get options() { return this.children.filter(n => n.tagName === 'OPTION'); }
}

globalThis.document = { body: new Element("body"), documentElement: new Element("html"),
  createElement: tag => new Element(tag), createTextNode: text => { const n = new Element("text"); n.textContent = text; return n; },
  dispatchEvent() {}, addEventListener() {} };
globalThis.window = { addEventListener() {} };
const pageBundles = await build({ configFile: false, root, logLevel: "silent", plugins: [{
  name: "realtime-settings-language", enforce: "pre",
  transform(code, id) { if (id.endsWith("/settings/realtime.ts")) return `${code}\nexport { setLang } from "../../i18n";`; },
}], build: { write: false, minify: false, lib: { entry: `${root}src/surfaces/settings/realtime.ts`, formats: ["es"] } } });
const pageChunk = pageBundles[0].output.find(part => part.type === "chunk");
const { createRealtimeSettings, setLang } = await import(`data:text/javascript;base64,${Buffer.from(pageChunk.code).toString("base64")}`);
const reads = [], writes = [];
const pending = () => { let resolve, reject; const promise = new Promise((yes, no) => { resolve = yes; reject = no; }); return { promise, resolve, reject }; };
const tick = () => new Promise(resolve => setImmediate(resolve));
let keyOpens = 0;
const page = createRealtimeSettings({ load: () => { const d = pending(); reads.push(d); return d.promise; },
  save: (...args) => { const d = pending(); writes.push({ ...d, args }); return d.promise; } }, () => keyOpens++);
document.body.append(page.element);
const field = name => nodes(page.element).find(n => n.dataset.realtimeField === name);
const press = text => { const n = nodes(page.element).find(n => n.tagName === "BUTTON" && n.textContent === text); assert.ok(n, text); n.fire("click"); return n; };
const saveButton = () => nodes(page.element).find(n => n.tagName === "BUTTON" && n.textContent === "Save");
const snapshot = (on = false, amount = "1000000") => ({ on, model: saved.model, limitMicrousd: amount });
const edit = (name, value) => { const n = field(name); if (name === "on") n.checked = value; else n.value = value; n.fire(name === "budget" ? "input" : "change"); };
assert.equal(reads.length, 0); assert.equal(writes.length, 0);
page.visible(true); page.visible(false); reads[0].resolve(snapshot(true)); await tick();
assert.equal(field("on").disabled, true, "Departed bootstrap cannot populate the form");
page.visible(true); reads[1].resolve(snapshot()); await tick();
assert.equal(field("on").checked, false); assert.equal(field("budget").value, "1.000000");
edit("on", true); edit("budget", "1,000001"); page.relabel();
page.visible(false); page.visible(true); reads[2].resolve(snapshot()); await tick();
assert.equal(field("budget").value, "1,000001", "Navigation preserves the exact unsent draft");
assert.equal(saveButton().disabled, false);
press("Save"); press("Save"); page.relabel();
assert.deepEqual(writes[0].args, [true, "1000001", saved.model]); assert.equal(writes.length, 1);
assert.equal(field("budget").disabled, true);
writes[0].reject(Error("private native detail")); await tick();
assert.equal(field("on").checked, true); assert.equal(field("budget").value, "1,000001");
assert.ok(page.element.textContent.includes("Could not confirm")); assert.ok(!page.element.textContent.includes("private native detail"));
press("Save"); writes[1].resolve(); await tick(); reads[3].resolve(snapshot(true, "1000001")); await tick();
assert.ok(page.element.textContent.includes("Saved.")); assert.equal(saveButton().disabled, true);
page.changed(); reads[4].resolve(snapshot(false, "2000000")); await tick();
edit("budget", "9223372036854.775807"); press("Save");
assert.deepEqual(writes[2].args, [false, "9223372036854775807", saved.model]);
writes[2].resolve(); await tick(); reads[5].resolve(snapshot(false, "9223372036854775807")); await tick();
assert.equal(field("budget").value, "9223372036854.775807", "Large persisted limits round-trip without floating point");
edit("on", true); edit("budget", "0"); assert.equal(saveButton().disabled, true);
for (const value of ["1e2", "1,000.00", "-1", "NaN", "1.0000001", "9223372036854.775808"]) {
  edit("budget", value); assert.equal(saveButton().disabled, true, value);
}
edit("budget", "1.50"); page.changed(); reads[6].resolve(snapshot(false, "3000000")); await tick();
assert.equal(field("budget").value, "1.50"); assert.equal(saveButton().disabled, true);
assert.ok(page.element.textContent.includes("changed elsewhere"));
press("Discard edits and reload"); reads[7].resolve(snapshot(false, "3000000")); await tick();
assert.equal(field("budget").value, "3.000000"); assert.equal(field("on").checked, false);
edit("budget", "2"); press("Discard edits and reload"); reads[8].reject(Error("private load detail")); await tick();
assert.equal(field("budget").value, "2"); assert.equal(saveButton().disabled, true);
press("Discard edits and reload"); reads[9].resolve(snapshot()); await tick();
edit("on", true); press("Save"); writes[3].resolve(); await tick(); reads[10].resolve(snapshot(false)); await tick();
assert.ok(page.element.textContent.includes("changed elsewhere"), "An acknowledgement alone cannot claim saved consent");
press("Configure OpenAI key"); assert.equal(keyOpens, 1);
for (const lang of ["pt-BR", "es", "zh"]) { setLang(lang); page.relabel(); assert.ok(!page.element.textContent.includes("Enable voice conversation")); }
setLang("en"); page.visible(false);
console.log("check-realtime-settings: drafts, navigation, exact USD, single save, failures, external changes, readback, translations and explicit key navigation");
