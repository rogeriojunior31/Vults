// Production readiness owner: inventory only, no microphone capture or desktop effects.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";
const all = node => [node, ...node.children.flatMap(all)];
class Element {
  children = []; listeners = {}; attributes = {}; dataset = {}; hidden = false; disabled = false;
  constructor(tag) { this.tag = tag; }
  set textContent(value) { this.text = value; this.children = []; }
  get textContent() { return (this.text ?? "") + this.children.map(node => node.textContent).join(""); }
  append(...nodes) { this.children.push(...nodes); }
  replaceChildren(...nodes) { this.text = ""; this.children = nodes; }
  addEventListener(name, fn) { this.listeners[name] = fn; }
  setAttribute(name, value) { this.attributes[name] = value; }
  focus() { document.activeElement = this; }
  click() { if (!this.disabled) this.listeners.click?.(); }
}
globalThis.document = { createElement: tag => new Element(tag), documentElement: {} };
const root = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({ configFile: false, root, logLevel: "silent", plugins: [{
  name: "readiness-language", transform(code, id) {
    if (id.endsWith("/voice-command-readiness.ts")) return code + '\nexport { setLang } from "./i18n";';
  },
}], build: { write: false, minify: false, lib: { entry: `${root}src/voice-command-readiness.ts`, formats: ["es"] } } });
const { createCommandReadiness, setLang } = await import(`data:text/javascript;base64,${Buffer.from(bundles[0].output.find(p => p.type === "chunk").code).toString("base64")}`);
const reads = [], destinations = [];
const view = createCommandReadiness(() => new Promise((resolve, reject) => reads.push({ resolve, reject })), destination => destinations.push(destination));
const text = () => view.element.textContent;
const button = label => all(view.element).find(node => node.tag === "button" && node.textContent === label);
const tick = () => new Promise(resolve => setImmediate(resolve));
const settings = { dictationCommands: true, dictationCommandMedia: true, dictationCommandApps: false, dictationCommandOutput: false, nowPlaying: false, dictationMicrophone: "Private input" };
const snapshot = { settings, microphones: ["Default input"], voice: { models: [{ id: "base", label: "Base", installed: true }], ready: true, selected: "base", language: "pt", system: "en" } };
assert.equal(reads.length, 0, "Opening the view does not enumerate devices or start recording");
view.visible(true);
view.setKeys({ talk: "Meta+V", "talk-command": "Meta+Z" });
assert.ok(text().includes("Meta+V") && text().includes("Meta+Z"));
const refresh = button("Check command setup");
refresh.focus(); refresh.click(); refresh.click();
assert.equal(reads.length, 1, "Repeated refresh cannot overlap inventory reads");
assert.equal(refresh.disabled, true);
reads[0].resolve(snapshot); await tick();
assert.ok(text().includes("Base") && text().includes("pt"));
assert.ok(text().includes("preferred microphone is missing") && text().includes("Media controls must also be on"));
assert.equal(document.activeElement, refresh, "Completion preserves the refresh action");
assert.equal(refresh.disabled, false);
button("Voice").click(); button("Keyboard shortcuts").click(); button("Diagnostics").click();
assert.deepEqual(destinations, ["dictationvoice", "dictationkeys", "dictationdiagnostics"]);
refresh.click(); reads[1].reject(Error("private native detail")); await tick();
assert.ok(text().includes("Could not read command setup") && !text().includes("private native detail"));
assert.ok(!text().includes("Base"), "Failed refresh must not report stale setup as current");
refresh.click(); view.visible(false); view.visible(true); refresh.click();
reads[2].resolve(snapshot); await tick();
assert.ok(!text().includes("Base"), "Late reply from a departed view is ignored");
reads[3].resolve({ ...snapshot, microphones: [], voice: { ...snapshot.voice, ready: false } }); await tick();
assert.ok(text().includes("No microphones found") && text().includes("Choose an installed speech model"));
refresh.click(); reads[4].resolve({ voice: null, settings: null, microphones: null }); await tick();
assert.ok(text().includes("Microphone availability is unknown"));
refresh.click(); reads[5].resolve({ ...snapshot, microphones: ["Private input"] }); await tick();
assert.ok(text().includes("Test capture in Diagnostics"), "Enumeration never claims verified capture");
const voiceLink = button("Voice"); voiceLink.focus(); view.relabel();
assert.equal(document.activeElement, button("Voice"), "Repaint preserves navigation focus");
refresh.click(); view.invalidate();
assert.ok(text().includes("Command setup changed"));
assert.equal(refresh.disabled, false);
reads[6].resolve(snapshot); await tick();
assert.ok(!text().includes("Base"), "A changed setup invalidates in-flight inventory");
assert.ok(text().includes("Command setup changed"));
view.invalidate(); assert.ok(text().includes("Command setup changed"), "Repeated notifications retain the recovery hint");
refresh.click(); reads[7].resolve(snapshot); await tick();
assert.ok(text().includes("Base") && !text().includes("Command setup changed"));
for (const lang of ["pt-BR", "es", "zh"]) {
  setLang(lang); view.relabel();
  assert.ok(!text().includes("Command setup") && !text().includes("Speech is transcribed first"));
}
console.log("check-voice-command-readiness: explicit inventory, truthful unknown/missing states, opt-in dependencies, separate keys, owner links, focus, retry, stale replies and translations");

// Test the real bridge aggregation without any native invocations.
const bridgeBundle = await build({ configFile: false, root, logLevel: "silent", plugins: [{
  name: "inventory-native-boundary", enforce: "pre",
  resolveId(id) { if (id.startsWith("@tauri-apps/api/")) return `\0inventory:${id}`; },
  load(id) {
    if (id.startsWith("\0inventory:")) return 'export const invoke = (...args) => globalThis.inventoryInvoke(...args); export const listen = () => {}; export const emitTo = () => {}; export const getCurrentWindow = () => {};';
  },
}], build: { write: false, minify: false, lib: { entry: `${root}src/bridge.ts`, formats: ["es"] } } });
const calls = [];
let denyInputs = false, denyVoice = false, denySettings = false;
globalThis.inventoryInvoke = async name => {
  calls.push(name);
  if (name === "voice_status") { if (denyVoice) throw Error("Status unavailable"); return snapshot.voice; }
  if (name === "app_settings") { if (denySettings) throw Error("Settings unavailable"); return snapshot.settings; }
  if (name === "dictation_microphones") {
    if (denyInputs) throw Error("Input enumeration failed");
    return snapshot.microphones;
  }
  throw Error(`Unexpected native operation: ${name}`);
};
const { Bridge } = await import(`data:text/javascript;base64,${Buffer.from(bridgeBundle[0].output.find(p => p.type === "chunk").code).toString("base64")}`);
assert.deepEqual(await Bridge.commandReadiness(), { voice: snapshot.voice, settings: snapshot.settings, microphones: null });
assert.deepEqual(calls, ["voice_status", "app_settings"], "Nest does not request privileged input enumeration");
calls.length = 0;
assert.deepEqual(await Bridge.commandReadiness(true), snapshot);
assert.deepEqual(calls, ["voice_status", "app_settings", "dictation_microphones"]);
denyInputs = true;
assert.deepEqual(await Bridge.commandReadiness(true), { voice: snapshot.voice, settings: snapshot.settings, microphones: null }, "Input failure retains independent model and setting readings");
denyVoice = denySettings = true;
await assert.rejects(Bridge.commandReadiness(), /Could not read command setup/, "A total read failure must reach the retry state");
await assert.rejects(Bridge.commandReadiness(true), /Could not read command setup/);
denyInputs = false;
assert.deepEqual(await Bridge.commandReadiness(true), { voice: null, settings: null, microphones: snapshot.microphones }, "A useful partial reading remains available");
denyVoice = denySettings = false;
assert.deepEqual(await Bridge.commandReadiness(), { voice: snapshot.voice, settings: snapshot.settings, microphones: null }, "Explicit retry recovers after native reads return");
console.log("check-voice-command-readiness: real bridge reads only status/settings; privileged input inventory is explicit and partial failures stay unknown");

setLang("en");
const cancellableReads = [];
const cancellable = createCommandReadiness(() => new Promise(resolve => cancellableReads.push(resolve)), () => {});
const cancellationRefresh = all(cancellable.element).find(node => node.dataset.setting === "command-readiness-refresh");
const cancellationAction = all(cancellable.element).find(node => node.dataset.setting === "command-readiness-cancel");
assert.equal(cancellationAction.hidden, true);
cancellable.visible(true); cancellationRefresh.click();
assert.equal(cancellationAction.hidden, false);
cancellationAction.focus(); cancellationAction.click();
assert.equal(cancellationAction.hidden, true);
assert.equal(cancellationRefresh.disabled, false);
assert.equal(document.activeElement, cancellationRefresh);
assert.ok(cancellable.element.textContent.includes("Check cancelled"));
assert.equal(cancellableReads.length, 1, "Cancel never starts another native read");
cancellationRefresh.click(); cancellationAction.focus();
cancellableReads[0](snapshot); await tick();
assert.equal(cancellationRefresh.disabled, true, "An old reply cannot end the replacement check");
assert.ok(!cancellable.element.textContent.includes("Base"));
cancellableReads[1](snapshot); await tick();
assert.equal(cancellationRefresh.disabled, false);
assert.equal(cancellationAction.hidden, true);
assert.equal(document.activeElement, cancellationRefresh, "Completion returns focus before hiding Cancel");
assert.ok(cancellable.element.textContent.includes("Base"));
console.log("check-voice-command-readiness: explicit cancellation, immediate retry, stale read isolation, hidden idle action and focus return");
