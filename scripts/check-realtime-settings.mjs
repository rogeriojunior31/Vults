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

// Exercise the work page with native owners replaced, keeping all lifecycle races deterministic.
const conversationBundles = await build({ configFile: false, root, logLevel: "silent",
  build: { write: false, minify: false, lib: { entry: `${root}src/surfaces/nest/dictation/conversation.ts`, formats: ["es"] } } });
const conversationChunk = conversationBundles[0].output.find(part => part.type === "chunk");
const { createVoiceConversation } = await import(`data:text/javascript;base64,${Buffer.from(conversationChunk.code).toString("base64")}`);
const originalTimeout = globalThis.setTimeout, originalClear = globalThis.clearTimeout;
const timers = new Map(); let nextTimer = 0;
globalThis.setTimeout = callback => { const id = ++nextTimer; timers.set(id, callback); return id; };
globalThis.clearTimeout = id => timers.delete(id);
try {
  const starts = [], polls = [], controls = [], copies = [], automaticChoices = [];
  let configured = 0;
  const work = createVoiceConversation({
    start: automatic => { automaticChoices.push(automatic); const d = pending(); starts.push(d); return d.promise; },
    status: id => { const d = pending(); polls.push({ ...d, id }); return d.promise; },
    control: async (id, action, response) => { controls.push({ id, action, ...(response ? { response } : {}) }); },
    copy: async text => { copies.push(text); },
  }, () => configured++);
  document.body.append(work.element);
  const button = name => nodes(work.element).find(n => n.tagName === "BUTTON" && n.textContent === name);
  const click = async name => { const b = button(name); assert.ok(b, name); if (!b.disabled) { b.fire("click"); await b.onclick?.(); } };
  const state = (id, phase, extra = {}) => ({ id, phase, reason: null, draft: null, reply: null, ...extra });
  const refresh = async () => { const entry = timers.entries().next().value; assert.ok(entry, "An active session needs a heartbeat"); timers.delete(entry[0]); entry[1](); await tick(); };
  assert.equal(starts.length, 0);
  work.setActive(true); await click("Voice conversation settings"); assert.equal(configured, 1);
  const automatic = nodes(work.element).find(n => n.tagName === "INPUT" && "voiceAutomatic" in n.dataset);
  assert.ok(!automatic.checked, "Automatic completion must start off");
  automatic.checked = true; automatic.fire("change");
  const first = click("Start recording"); await tick();
  assert.deepEqual(automaticChoices, [true]);
  assert.equal(automatic.disabled, true, "Freeze the mode while Start is pending");
  assert.equal(starts.length, 1); await click("Start recording"); assert.equal(starts.length, 1);
  work.setActive(false); starts[0].resolve(state("1", "starting")); await first;
  assert.deepEqual(controls.pop(), { id: "1", action: "cancel" }, "A departed start acknowledgement must cancel its new native owner");
  assert.equal(timers.size, 0);
  work.setActive(true);
  const second = click("Start recording"); starts[1].resolve(state("2", "recording")); await second;
  assert.equal(button("Start recording").disabled, true);
  assert.equal(button("Finish recording").disabled, false);
  assert.equal(automatic.disabled, true);
  assert.ok(work.element.textContent.includes("Pause to finish automatically"));
  await click("Finish recording"); assert.deepEqual(controls.pop(), { id: "2", action: "finish" });
  await refresh(); polls[0].resolve(state("2", "recording", { reason: "auto-unavailable" })); await tick();
  assert.equal(button("Finish recording").disabled, false);
  assert.ok(work.element.textContent.includes("Automatic finish is unavailable"));
  await refresh(); polls[1].resolve(state("2", "recovery", { draft: '<script>kept as text</script>', reason: 'key' })); await tick();
  const draft = nodes(work.element).find(n => n.tagName === "TEXTAREA");
  assert.equal(draft.value, '<script>kept as text</script>'); assert.equal(timers.size, 0);
  draft.value = "My reviewed words"; draft.fire("input"); await click("Copy text"); assert.deepEqual(copies, ["My reviewed words"]);
  work.setActive(false); work.setActive(true); assert.equal(draft.value, "My reviewed words", "Navigation preserves the unsent reviewed draft");
  const third = click("Start recording"); starts[2].resolve(state("3", "recording")); await third;
  await refresh(); await click("Cancel"); polls[2].resolve(state("3", "recovery", { draft: "stale secret" })); await tick();
  assert.equal(draft.value, ""); assert.ok(!work.element.textContent.includes("stale secret")); assert.equal(timers.size, 0);
  const fourth = click("Start recording"); starts[3].resolve(state("4", "recording")); await fourth;
  await refresh(); polls[3].reject(Error("private native detail")); await tick();
  assert.deepEqual(controls.pop(), { id: "4", action: "cancel" });
  assert.ok(!work.element.textContent.includes("private native detail")); assert.ok(work.element.textContent.includes("Could not confirm voice status"));
  const fifth = click("Start recording"); starts[4].reject(Error("private preflight detail")); await fifth;
  assert.ok(work.element.textContent.includes("Could not start recording"));
  assert.ok(!work.element.textContent.includes("private preflight detail"));
  const sixth = click("Start recording"); starts[5].resolve(state("6", "recording")); await sixth;
  await refresh(); polls.at(-1).resolve(state("6", "waiting", { reply: "The first heard reply" })); await tick();
  assert.equal(button("Record another turn").disabled, false);
  assert.equal(button("Finish recording").disabled, true);
  assert.equal(button("End conversation").disabled, false);
  assert.ok(work.element.textContent.includes("Microphone off."));
  assert.ok(timers.size > 0, "Waiting retains the page lease without reopening capture");
  const following = click("Record another turn"); await tick();
  assert.deepEqual(controls.at(-1), { id: "6", action: "next" });
  await click("Record another turn");
  assert.equal(controls.filter(c => c.action === "next").length, 1, "A duplicate click cannot enqueue another turn");
  assert.equal(starts.length, 6, "Continuation must reuse the native session");
  polls.at(-1).resolve(state("6", "starting", { reply: "The first heard reply" })); await following;
  assert.ok(work.element.textContent.includes("The first heard reply"));
  await refresh(); polls.at(-1).resolve(state("6", "waiting", { reply: "The second heard reply" })); await tick();
  await refresh(); polls.at(-1).resolve(state("6", "speaking", { response: "reply-1" })); await tick();
  assert.equal(button("Stop response").disabled, false);
  await click("Stop response");
  assert.deepEqual(controls.at(-1), { id: "6", action: "interrupt", response: "reply-1" });
  assert.equal(button("Stop response").disabled, true);
  assert.ok(work.element.textContent.includes("Stopping the response"));
  await refresh(); polls.at(-1).resolve(state("6", "waiting", { reply: "Only the heard prefix [interrupted]" })); await tick();
  assert.equal(button("Record another turn").disabled, false);
  assert.equal(starts.length, 6, "Stopping output cannot open another microphone/session");
  await click("End conversation");
  assert.deepEqual(controls.at(-1), { id: "6", action: "cancel" });
  assert.equal(timers.size, 0);
  work.setActive(false);
  assert.equal(timers.size, 0);
} finally {
  globalThis.setTimeout = originalTimeout; globalThis.clearTimeout = originalClear;
}
console.log("check-realtime-settings: explicit work-page controls, duplicate start, late acknowledgement, heartbeat, recovery draft, inert text, copy, cancellation and departure pass");

{
  const controls = [], starts = []; let lists = 0, listing = pending();
  const inventory = { token: "reviewed", sources: [{ name: "mic", label: "Microphone" }], sinks: [{ name: "out", label: "Headphones" }], pairs: [{ source: "clean", sink: "reference" }] };
  const work = createVoiceConversation({
    routes: () => { lists++; return listing.promise; },
    start: async (automatic, route) => { starts.push({ automatic, route }); return { id: "r", phase: "recording", capture: "capture-1", routed: true, reason: null, draft: null, reply: null }; },
    status: async () => ({ id: "r", phase: "waiting", capture: null, routed: true, reason: null, draft: null, reply: null }),
    control: async (id, action, response, capture) => { controls.push({ id, action, response, capture }); },
    copy: async () => {},
  }, () => {});
  document.body.append(work.element); work.setActive(true);
  const button = text => nodes(work.element).find(n => n.tagName === "BUTTON" && n.textContent === text);
  const choose = (field, value) => { const e = nodes(work.element).find(n => n.dataset.voiceRoute === field); if (e.type === "checkbox") e.checked = value; else e.value = value; e.fire("change"); return e; };
  const click = async text => { const b = button(text); assert.ok(b, text); if (!b.disabled) await b.onclick?.(); };
  choose("mode", "headphones");
  assert.equal(lists, 0, "Changing mode cannot inspect devices or open audio");
  assert.equal(button("Start recording").disabled, true);
  const loading = click("Refresh audio devices"); listing.resolve(inventory); await loading;
  choose("consent", true); assert.equal(button("Start recording").disabled, true, "Device selection is explicit too");
  choose("source", "mic"); choose("sink", "out");
  assert.equal(button("Start recording").disabled, true, "Changing devices revokes previous consent");
  choose("consent", true); assert.equal(button("Start recording").disabled, false);
  await click("Start recording");
  assert.deepEqual(starts, [{ automatic: false, route: { token: "reviewed", mode: "headphones", source: "mic", sink: "out", confirmed: true } }]);
  assert.equal(nodes(work.element).find(n => n.dataset.voiceRoute === "mode").disabled, true);
  await click("Finish recording");
  assert.equal(controls.at(-1).capture, "capture-1", "Finish is bound to the exact native recording");
  assert.equal(button("Finish recording").disabled, true, "Finish cannot be double-clicked while decoding");
  await click("Cancel");
  assert.equal(button("Start recording").disabled, true, "A new conversation needs fresh confirmation");
  choose("mode", "echo"); choose("pair", "0"); choose("consent", true);
  assert.equal(button("Start recording").disabled, false);
  const refresh = click("Refresh audio devices"); // Replaced inventory invalidates an otherwise valid draft.
  await refresh;
  assert.equal(button("Start recording").disabled, true);
  listing = pending();
  const failing = click("Refresh audio devices"); listing.reject(Error("private server details")); await failing;
  assert.equal(button("Start recording").disabled, true);
  assert.ok(work.element.textContent.includes("Could not list audio devices"));
  assert.ok(!work.element.textContent.includes("private server details"));
  listing = pending();
  const late = click("Refresh audio devices"); work.setActive(false); listing.resolve(inventory); await late;
  assert.equal(nodes(work.element).find(n => n.dataset.voiceRoute === "consent").checked, false);
  assert.equal(starts.length, 1, "Late inventory or departure cannot start recording");
  work.element.remove();
}
console.log("check-realtime-settings: explicit route/device/consent, frozen start, recording-bound Finish, refresh failure and late inventory pass");

{
  const capability = pending(), starts = [], localTimers = new Map(); let timerId = 0;
  const state = { id: "semantic", phase: "recording", capture: "capture-1", routed: false, semanticFallback: false, reason: null, draft: null, reply: null };
  globalThis.setTimeout = cb => { localTimers.set(++timerId, cb); return timerId; };
  globalThis.clearTimeout = id => localTimers.delete(id);
  try {
    const work = createVoiceConversation({
      options: () => capability.promise,
      start: async (automatic, route, semantic) => { starts.push({ automatic, route, semantic }); return { ...state }; },
      status: async () => ({ ...state }), control: async () => {}, copy: async () => {},
    }, () => {});
    document.body.append(work.element); work.setActive(true);
    const semantic = nodes(work.element).find(n => "voiceSemantic" in n.dataset);
    const automatic = nodes(work.element).find(n => "voiceAutomatic" in n.dataset);
    assert.equal(semantic.parentElement.hidden, true, "Unsupported/unread capability cannot offer semantic inference");
    automatic.checked = true; automatic.fire("change"); capability.resolve({ semantic: true }); await tick();
    assert.equal(semantic.parentElement.hidden, false); assert.equal(semantic.checked, false);
    semantic.checked = true; semantic.fire("change");
    const button = text => nodes(work.element).find(n => n.tagName === "BUTTON" && n.textContent === text);
    await button("Start recording").onclick();
    assert.deepEqual(starts, [{ automatic: true, route: undefined, semantic: true }]);
    assert.equal(semantic.disabled, true, "Freeze semantic selection for the retained conversation");
    state.semanticFallback = true;
    const poll = async () => { const [id, cb] = localTimers.entries().next().value; localTimers.delete(id); cb(); await tick(); };
    await poll(); assert.ok(work.element.textContent.includes("This conversation uses local silence detection."));
    state.phase = "waiting"; await poll(); assert.equal(semantic.disabled, true);
    button("End conversation").onclick();
    assert.equal(semantic.checked, false, "Another conversation needs a fresh semantic choice");
    work.setActive(false); work.element.remove();
    const late = pending();
    const departed = createVoiceConversation({ options: () => late.promise, start: async () => { throw Error("Unexpected capture"); }, status: async () => ({ ...state }), control: async () => {}, copy: async () => {} }, () => {});
    document.body.append(departed.element); departed.setActive(true); departed.setActive(false); late.resolve({ semantic: true }); await tick();
    assert.equal(nodes(departed.element).find(n => "voiceSemantic" in n.dataset).parentElement.hidden, true);
    departed.element.remove(); assert.equal(localTimers.size, 0);
  } finally { globalThis.setTimeout = originalTimeout; globalThis.clearTimeout = originalClear; }
}
console.log("check-realtime-settings: semantic preview capability, explicit default-off choice, frozen session, visible failure fallback and late/departed capability pass");
