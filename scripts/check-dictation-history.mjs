// Exercise the production history owner; no browser server or native clipboard is needed.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";
import { build } from "vite";
class Element {
  children = []; listeners = {}; attributes = {}; dataset = {}; scrollTop = 0; className = ""; value = ""; style = {}; hidden = false; isConnected = true;
  constructor(tag) { this.tag = tag; }
  set textContent(value) { this.text = value; this.children = []; }
  get textContent() { return (this.text ?? "") + this.children.map(n => n.textContent).join(""); }
  append(...nodes) { for (const n of nodes) n.parentElement = this; this.children.push(...nodes); }
  replaceChildren(...nodes) { this.text = ""; this.children = []; this.append(...nodes); }
  addEventListener(name, fn) { this.listeners[name] = fn; }
  setAttribute(name, value) { this.attributes[name] = value; }
  getAttribute(name) { return this.attributes[name] ?? null; }
  querySelector(selector) { return all(this).slice(1).find(n => selector.startsWith("[data-history-action=") ? n.dataset.historyAction === selector.slice(22, -2) : selector.startsWith(".") ? n.className === selector.slice(1) : n.tag === selector) ?? null; }
  contains(node) { return all(this).includes(node); }
  closest() { return null; }
  focus() { document.activeElement = this; }
  click() { if (!this.disabled) this.listeners.click?.(); }
}
const all = n => [n, ...n.children.flatMap(all)];
globalThis.document = { createElement: tag => new Element(tag), createElementNS: (_, tag) => new Element(tag), documentElement: {} };
globalThis.window = { clearTimeout() {}, setTimeout() { return 0; } };
let copied = [], copyFails = false;
Object.defineProperty(globalThis, "navigator", { configurable: true, value: { clipboard: { async writeText(text) { if (copyFails) throw Error("No clipboard"); copied.push(text); } } } });
const root = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({ configFile: false, root, logLevel: "silent", plugins: [{
  name: "history-test-language", transform(code, id) { if (id.endsWith("/dictation/history.ts")) return `${code}\nexport { setLang } from "../../../i18n";`; },
}], build: { write: false, minify: false, lib: { entry: `${root}src/surfaces/nest/dictation/history.ts`, formats: ["es"] } } });
const { createDictationHistory, setLang } = await import(`data:text/javascript;base64,${Buffer.from(bundles[0].output.find(p => p.type === "chunk").code).toString("base64")}`);
const tick = () => new Promise(resolve => setImmediate(resolve));
const row = (id, text) => ({ id, at: 2_000_000_000 - (id === 2 ? 86_400 : 0), app: "<img src=x>", session: "codex:api", heard: `heard ${text}`, text, words: 2, secs: 2.5, style: "polished", outputLanguage: "es", language: "en", fixes: null, outcome: "copied" });
let data = { retention: "30", rows: [row(1, "<script>inert text</script>"), row(2, "Other words")], today: { words: 4, secs: 5, count: 2 } };
let loads = 0, removals = [], clears = 0, again = [], failRead = false, failAction = false, settings = 0, deferred, pendingAction;
const view = createDictationHistory({
  load: async () => { loads++; if (deferred) return deferred; if (failRead) throw Error("Database unavailable"); return structuredClone(data); },
  remove: async id => { if (failAction) throw Error("Not erased"); removals.push(id); data.rows = data.rows.filter(r => r.id !== id); },
  clear: async () => { clears++; data.rows = []; },
  again: async id => { if (pendingAction) await pendingAction; if (failAction) throw Error("Busy"); again.push(id); },
}, () => settings++);
const nodes = () => all(view.element);
const named = text => nodes().find(n => n.tag === "button" && (n.textContent === text || n.title === text));
const entries = () => nodes().filter(n => n.className === "dictation-entry");
const input = view.element.querySelector("input");
const search = value => { input.value = value; input.listeners.input(); };
assert.equal(loads, 0, "Constructing a hidden page never reads history");
assert.ok(view.element.textContent.includes("Reading dictation history"));
view.refresh(); await tick();
const loadsBeforeKeys = loads;
view.setKeys({ talk: "Meta+F8" });
assert.equal(loads, loadsBeforeKeys, "Reading shortcut display never reloads history");
assert.equal(nodes().find(n => n.className === "dictation-keycaps").textContent, "MetaF8", "The hero uses the actual binding, not a hardcoded default");
view.setKeys({});
assert.equal(nodes().find(n => n.className === "dictation-keycaps").textContent, "Not bound");
assert.equal(entries().length, 2);
assert.ok(entries()[0].textContent.includes("Polished"));
assert.ok(entries()[0].textContent.includes("In Spanish"));
assert.equal(nodes().filter(n => n.className === "dictation-day").length, 2);
assert.ok(view.element.textContent.includes("<script>inert text</script>"));
assert.equal(entries()[0].children[0].children.length, 0, "Transcript HTML is inert");
assert.ok(!view.element.textContent.includes("corrections"), "Unknown correction counts remain absent");
search(" HEARD OTHER "); assert.equal(entries().length, 1);
assert.equal(loads, 1, "Filtering never reopens the database");
search("missing"); assert.equal(entries().length, 0);
assert.ok(view.element.textContent.includes("No dictations match"));
assert.equal(nodes().find(n => n.className === "dictation-results").textContent, "0 entries");
named("Clear search").click();
assert.equal(input.value, ""); assert.equal(document.activeElement, input);
assert.equal(entries().length, 2);
search("missing");
input.listeners.keydown({ key: "Escape", preventDefault() {}, stopPropagation() {} });
assert.equal(input.value, ""); assert.equal(entries().length, 2);
assert.equal(nodes().find(n => n.className === "dictation-results").textContent, "2 entries");
assert.equal(nodes().find(n => n.className === "dictation-maintenance").open, undefined, "Bulk deletion starts in a closed maintenance disclosure");
const heard = nodes().find(n => n.className === "dictation-heard");
heard.open = true; heard.listeners.toggle();
heard.querySelector("summary").focus();
view.element.scrollTop = 120;
view.refresh(); await tick();
assert.equal(view.element.scrollTop, 120);
assert.equal(document.activeElement.dataset.historyAction, "heard-1", "Live refresh restores disclosure focus");
assert.equal(nodes().find(n => n.className === "dictation-heard").open, true);
assert.equal(view.element.querySelector("input"), input, "Reads retain the editable search node");
named("Copy").click(); await tick();
assert.deepEqual(copied, [data.rows[0].text]);
copyFails = true; named("Copy").click(); await tick();
assert.ok(view.element.textContent.includes("Copy failed. Try again."));
copyFails = false;
named("Use again").focus(); named("Use again").click(); await tick();
assert.equal(document.activeElement.dataset.historyAction, "0-1", "A completed action retains its row focus");
assert.deepEqual(again, [1]);


assert.ok(view.element.textContent.includes("Nothing is submitted"));
assert.equal(copied.length, 1, "Use again routes through the worker, not a second UI clipboard owner");
for (const failed of [false, true]) {
  let finishAction;
  pendingAction = new Promise(resolve => { finishAction = resolve; });
  failAction = failed;
  named("Use again").focus(); named("Use again").click();
  input.focus(); search("Other");
  finishAction(); await tick(); pendingAction = null;
  assert.equal(document.activeElement, input, "Pending history actions preserve a newer search focus on success and failure");
  assert.equal(input.value, "Other");
  search("");
}
failAction = false;
named("Delete…").focus(); named("Delete…").click();
assert.equal(document.activeElement.textContent, "Cancel", "Deletion starts with the safe alternative focused");
named("Cancel").click();
assert.equal(document.activeElement.dataset.historyAction, "2-1", "Cancelling returns to the originating row");
named("Delete…").click(); assert.deepEqual(removals, []);
assert.ok(view.element.textContent.includes("Erase this dictation?"));
failAction = true; named("Erase").click(); await tick();
assert.equal(entries().length, 2); assert.ok(view.element.textContent.includes("Could not complete"));
assert.equal(document.activeElement.textContent, "Cancel", "Failed deletion keeps a usable confirmation with safe focus");
failAction = false; named("Erase").click(); await tick();
assert.deepEqual(removals, [1]); assert.equal(entries().length, 1);
named("Erase dictation history…").click(); named("Cancel").click();
assert.equal(clears, 0, "Erase needs its own confirmation");
named("Erase dictation history…").click(); named("Erase").click(); await tick();
assert.equal(clears, 1); assert.equal(entries().length, 0);
assert.ok(view.element.textContent.includes("Nothing dictated yet"));
assert.ok(!view.element.textContent.includes("Unknown"), "Usage totals survive erasure");
data = { retention: "0", rows: [], today: { words: 10, secs: .5, count: 1 } };
view.refresh(); await tick(); assert.ok(view.element.textContent.includes("history is not kept"));
assert.ok(view.element.textContent.includes("Unknown"), "A sub-second sample has no speed measurement");
failRead = true; view.refresh(); await tick();
assert.ok(view.element.textContent.includes("Could not load"));
assert.ok(view.element.textContent.includes("last loaded view"));
failRead = false;
let oldResolve;
deferred = new Promise(resolve => oldResolve = resolve); view.refresh();
deferred = null; data = { retention: "30", rows: [row(3, "Newest result")], today: null }; view.refresh(); await tick();
oldResolve({ retention: "30", rows: [row(4, "Old result")], today: null }); await tick();
assert.equal(entries().length, 1); assert.ok(view.element.textContent.includes("Newest result"));
assert.ok(!view.element.textContent.includes("Old result"));
view.invalidate(); failRead = true; view.refresh(); await tick();
assert.equal(entries().length, 0, "An erase/retention event removes the stale transcript even if the next read fails");
assert.ok(!view.element.textContent.includes("Newest result"));
failRead = false; data.rows = [row(5, "x".repeat(8000))]; view.refresh(); await tick();
for (const lang of ["pt-BR", "es", "zh"]) { setLang(lang); view.relabel(); assert.ok(!view.element.textContent.includes("Dictation history")); }
setLang("en"); view.relabel(); named("Dictation settings").click(); assert.equal(settings, 1);

data.rows = [{...row(9,"Command completed."),outcome:"command",style:"command",heard:"mute api"}];
view.refresh();await tick();
assert.equal(entries().length,1);
assert.ok(view.element.textContent.includes("Voice command"));
assert.ok(!nodes().some(node=>node.tag==="button" && node.textContent==="Use again"),"Command history never offers replay or paste");
assert.ok(!view.element.textContent.includes("Unknown app"));
assert.ok(nodes().some(node=>node.tag==="button" && node.textContent==="Copy"),"Copying the result stays an explicit text action");

const metrics = nodes().find(n => n.className === "dictation-summary");
assert.equal(metrics.open, false, "Small or unqualified layouts start with optional figures folded");
assert.ok(view.element.children.indexOf(nodes().find(n => n.className === "dictation-toolbar")) > view.element.children.indexOf(nodes().find(n => n.className === "dictation-hero")), "Shortcut and figures follow the prototype header");
metrics.open = true; metrics.listeners.toggle();
view.refresh(); await tick();
assert.equal(metrics.open, true, "Refresh retains the explicit statistics disclosure choice");
metrics.open = false; metrics.listeners.toggle();
view.relabel();
assert.equal(metrics.open, false, "Relabel retains explicitly closed figures");

const commandView = createDictationHistory({
  load: async () => structuredClone(data), remove: async id => { data.rows = data.rows.filter(r => r.id !== id); },
  clear: async () => { throw Error("The filtered command page cannot clear all dictations"); },
  again: async () => { throw Error("Commands cannot replay"); },
}, () => settings++, true);
data.rows.push(row(10, "ordinary dictation"));
commandView.refresh(); await tick();
const commandNodes = () => all(commandView.element);
assert.equal(commandNodes().filter(n => n.className === "dictation-entry").length, 1);
assert.ok(!commandView.element.textContent.includes("ordinary dictation"));
assert.ok(commandNodes().find(n => n.className === "dictation-figures").hidden, "Command page has no dictation figures");
assert.ok(commandNodes().find(n => n.tag === "button" && n.textContent === "Erase dictation history…").hidden, "No bulk erasure from a filtered page");
assert.ok(!commandNodes().some(n => n.tag === "button" && (n.textContent === "Use again" || n.title === "Use again")));
const commandSearch = commandView.element.querySelector("input");
commandSearch.value = "mute api"; commandSearch.listeners.input();
assert.equal(commandNodes().filter(n => n.className === "dictation-entry").length, 1, "Heard text is searchable");
commandSearch.value = "missing"; commandSearch.listeners.input();
assert.ok(commandView.element.textContent.includes("No commands match your search"));
commandSearch.value = ""; commandSearch.listeners.input();
commandNodes().find(n => n.tag === "button" && (n.textContent === "Delete…" || n.title === "Delete…")).click();
commandNodes().find(n => n.tag === "button" && n.textContent === "Erase").click(); await tick();
assert.deepEqual(data.rows.map(r => r.id), [10], "Individual erase preserves ordinary history");
assert.ok(commandView.element.textContent.includes("No voice commands yet"));
for (const language of ["pt-BR", "es", "zh"]) {
  setLang(language); commandView.relabel();
  assert.ok(!commandView.element.textContent.includes("No voice commands yet"));
}
setLang("en");
window.matchMedia = () => ({ matches: true });
const measured = createDictationHistory({ load: async () => ({ retention: "30", rows: [], today: { words: 45, secs: 10, count: 1 } }), remove: async () => {}, clear: async () => {}, again: async () => {} }, () => {});
measured.refresh(); await tick();
assert.equal(all(measured.element).find(n => n.className === "dictation-summary").open, true, "Wide history presents measured figures beside the shortcut");
delete window.matchMedia;
const css = readFileSync(`${root}src/surfaces/nest/nest.css`, "utf8");
assert.match(css, /#nest\s*>\s*\[role="tabpanel"\]\[hidden\][^{]*\{[^}]*display:\s*none/,
  "The tab owner's hidden rule must outrank class-based flex/grid pane styles, independent of source order");
assert.match(css, /\.nest-pane\.nest-dictation-pane\s*\{[^}]*overflow:\s*hidden/);
assert.match(css, /\.dictation-history\s*\{[^}]*min-height:\s*0[^}]*overflow-y:\s*auto/);
assert.match(css, /\.dictation-text[^}]*overflow-wrap:\s*anywhere/);
console.log("check-dictation-history: ok (states, measured/unknown totals, inert text, grouped/local search, explicit mutations, clipboard failure, stale reads, erase invalidation, translations, bounded scrolling)");
