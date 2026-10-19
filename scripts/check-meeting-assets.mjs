// Production installer controls with a deferred local backend; no desktop, disk or network.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";
const all = node => [node, ...node.children.flatMap(all)];
class Element {
  children = []; attributes = {}; listeners = {}; dataset = {}; className = ""; disabled = false; open = false;
  constructor(tag) { this.tag = tag; }
  set textContent(text) { this.text = text; this.children = []; }
  get textContent() { return (this.text ?? "") + this.children.map(n => n.textContent).join(""); }
  append(...nodes) { for (const node of nodes) { node.parentElement = this; this.children.push(node); } }
  addEventListener(name, fn) { this.listeners[name] = fn; }
  setAttribute(name, value) { this.attributes[name] = value; }
  focus() { document.activeElement = this; }
  click() { if (!this.disabled && !this.hidden) this.listeners.click?.(); }
}
globalThis.document = { createElement: tag => new Element(tag), documentElement: {}, hidden: false };
const scheduled = new Map(); let timer = 0;
globalThis.window = { setInterval: fn => { scheduled.set(++timer, fn); return timer; }, clearInterval: id => scheduled.delete(id) };
const ui = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({ configFile: false, root: ui, logLevel: "silent", plugins: [{ name: "installer-language", transform(code, id) {
  if (id.endsWith("/meeting-assets.ts")) return code + '\nexport { setLang } from "../../i18n";';
} }], build: { write: false, minify: false, lib: { entry: `${ui}src/surfaces/settings/meeting-assets.ts`, formats: ["es"] } } });
const { createMeetingAssets, setLang } = await import(`data:text/javascript;base64,${Buffer.from(bundles[0].output.find(p => p.type === "chunk").code).toString("base64")}`);
const statuses = [], reviews = [], starts = [], stops = [];
const pending = list => new Promise((resolve, reject) => list.push({ resolve, reject }));
const view = createMeetingAssets({ status: () => pending(statuses), review: () => pending(reviews),
  begin: (id, consent) => { const promise = pending(starts); starts.at(-1).id = id; starts.at(-1).consent = consent; return promise; },
  cancel: async id => { stops.push(id); return snapshot("cancelled", id); } });
const snapshot = (phase, id = "attempt") => ({ phase, id, error: null, downloadBytes: 46647585, artifact: "runtime", downloaded: 12, total: 100 });
const nodes = () => all(view.element), named = label => nodes().find(n => n.tag === "button" && n.textContent === label);
const checkbox = () => nodes().find(n => n.tag === "input");
const tick = () => new Promise(resolve => setImmediate(resolve));
const poll = async () => { [...scheduled.values()][0](); await tick(); };
view.visible(true); assert.equal(statuses.length, 0, "A closed disclosure starts no installation work");
view.element.open = true; view.element.listeners.toggle(); assert.equal(statuses.length, 1);
statuses[0].resolve(snapshot("idle")); await tick();
assert.equal(reviews.length, 0); assert.equal(starts.length, 0, "Reading the page cannot install");
named("Review installation").click(); reviews[0].resolve(snapshot("review")); await tick();
assert.ok(named("Download and install").disabled);
checkbox().checked = true; checkbox().listeners.change();
named("Download and install").click(); assert.deepEqual([starts[0].id, starts[0].consent], ["attempt", true]);
view.visible(false); assert.equal(stops.length, 0, "Explicit installation continues across navigation");
starts[0].resolve(snapshot("downloading")); await tick();
view.visible(true); statuses[1].resolve(snapshot("downloading")); await tick();
named("Cancel installation").click(); await tick(); assert.deepEqual(stops, ["attempt"]);
assert.ok(view.element.textContent.includes("Previously installed files are preserved"));
named("Review installation").click(); view.visible(false); reviews[1].resolve(snapshot("review", "late")); await tick();
assert.deepEqual(stops, ["attempt", "late"], "A review returned to a hidden page is revoked");
view.visible(true); statuses[2].resolve(snapshot("review", "new")); await tick();
checkbox().checked = true; checkbox().listeners.change(); view.element.open = false; view.element.listeners.toggle(); await tick();
assert.equal(checkbox().checked, false); assert.ok(stops.includes("new"));
view.element.open = true; view.element.listeners.toggle(); statuses[3].resolve(snapshot("downloading")); await tick();
await poll(); statuses[4].reject(Error("private signed URL")); await tick();
assert.ok(view.element.textContent.includes("Could not update installation status"));
assert.ok(!view.element.textContent.includes("private signed URL"));
assert.equal(checkbox().checked, false);
await poll(); statuses[5].resolve(snapshot("installed")); await tick();
assert.ok(view.element.textContent.includes("Installation does not enable automatic voice identification"));
assert.ok(named("Download and install").hidden);
for (const lang of ["pt-BR", "es", "zh"]) { setLang(lang); view.relabel(); assert.ok(!view.element.textContent.includes("Meeting speaker models (experimental)")); }
view.visible(false); assert.equal(scheduled.size, 0);
console.log("Meeting installer: review/consent, navigation, cancellation, stale replies, reconnection and translations pass");
