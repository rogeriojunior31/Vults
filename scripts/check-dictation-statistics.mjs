// Production statistics renderer with a local backend; no desktop, server, clipboard or writes.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";
const all = node => [node, ...node.children.flatMap(all)];
class Element {
  children = []; attributes = {}; listeners = {}; dataset = {}; style = {}; className = "";
  disabled = false; hidden = false; scrollLeft = 0; scrollTop = 0;
  constructor(tag) { this.tag = tag; }
  set textContent(text) { this.text = text; this.children = []; }
  get textContent() { return (this.text ?? "") + this.children.map(n => n.textContent).join(""); }
  append(...nodes) { for (const node of nodes) { node.parentElement = this; this.children.push(node); } }
  replaceChildren(...nodes) { this.children = []; this.text = ""; this.append(...nodes); }
  addEventListener(name, fn) { this.listeners[name] = fn; }
  setAttribute(name, value) { this.attributes[name] = value; }
  getAttribute(name) { return this.attributes[name]; }
  querySelector(selector) { return all(this).slice(1).find(n => {
    if (selector.startsWith(".")) return n.className.split(" ").includes(selector.slice(1));
    if (selector.startsWith("[data-stats-day=")) return n.dataset.statsDay === selector.slice(17,-2);
    return n.tag === selector;
  }) ?? null; }
  attachShadow() { return this.shadowRoot = new Element("shadow"); }
  focus() { root.shadowRoot.activeElement = this; }
  click() { if (!this.disabled) this.listeners.click?.(); }
}
globalThis.document = { createElement: tag => new Element(tag), createElementNS: (_, tag) => new Element(tag), documentElement: {} };
const kept = new Map();
globalThis.localStorage = { getItem: key => kept.get(key) ?? null, setItem: (key,value) => kept.set(key,value) };
const ui = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({ configFile: false, root: ui, logLevel: "silent", plugins: [{ name: "stats-language", transform(code,id) {
  if (id.endsWith("/dictation/statistics.ts")) return code+'\nexport { setLang } from "../../../i18n";';
} }], build: { write: false, minify: false, lib: { entry: `${ui}src/surfaces/nest/dictation/statistics.ts`, formats: ["es"] } } });
const { createDictationStatistics, setLang } = await import(`data:text/javascript;base64,${Buffer.from(bundles[0].output.find(p => p.type === "chunk").code).toString("base64")}`);
const requests = []; let settings = 0;
const view = createDictationStatistics(period => new Promise((resolve,reject) => requests.push({ period, resolve, reject })), () => settings++);
const root = view.element, nodes = () => all(root.shadowRoot);
const named = label => nodes().find(n => n.tag === "button" && n.textContent === label);
const text = () => root.shadowRoot.querySelector(".page").textContent;
const tick = () => new Promise(resolve => setImmediate(resolve));
const stats = (period, words = 90) => ({ period, from: "2026-10-12", through: "2026-10-18", chart_limited: false,
  usage: { count: words ? 2 : 0, words, secs: 60, saved_minutes: words ? 1 : null, wpm: words ? 90 : null },
  days: [{ day: "2026-10-17", count: 0, words: 0 }, { day: "2026-10-18", count: words ? 2 : 0, words }],
  apps: [{ app: "<script>inert app</script>", words }], apps_complete: true });
assert.equal(requests.length, 0, "Mounting a hidden pane performs no read");
view.refresh(); assert.ok(text().includes("Reading dictation statistics"));
requests[0].reject(Error("database unavailable")); await tick();
assert.ok(text().includes("Could not load dictation statistics"));
named("Try again").click(); requests[1].resolve(stats("week")); await tick();
assert.equal(nodes().filter(n => n.className === "activity-stat").length, 4);
assert.ok(text().includes("90") && text().includes("<script>inert app</script>"));
assert.equal(nodes().some(n => n.tag === "script"), false, "Application names are inert text");
const day = nodes().find(n => n.dataset.statsDay === "2026-10-18"); day.focus(); day.click();
assert.equal(root.shadowRoot.activeElement.dataset.statsDay, "2026-10-18");
view.refresh(); requests[2].reject(Error("stale")); await tick();
assert.ok(text().includes("last loaded view") && text().includes("90"));
named("Last 30 days").click();
assert.equal(requests[3].period, "month");
assert.ok(!text().includes("<script>inert app</script>"), "A new period never labels old data as its own");
named("All time").click(); assert.equal(requests[4].period, "all");
requests[4].resolve({ ...stats("all", 300), chart_limited: true }); await tick();
requests[3].resolve(stats("month", 999)); await tick();
assert.ok(text().includes("300") && !text().includes("999"), "Late periods cannot overwrite the current one");
assert.ok(text().includes("latest 90 days"));
named("Last 7 days").click(); requests[5].resolve({ ...stats("week"), apps_complete: false }); await tick();
assert.ok(text().includes("older dictations have no daily app breakdown"));
view.refresh(); requests[6].resolve(stats("week", 0)); await tick();
assert.ok(text().includes("No dictation recorded in this period") && text().includes("Unknown"));
named("Dictation settings").click(); assert.equal(settings, 1);
named("Last 30 days").click();
let restored;
createDictationStatistics(async period => { restored = period; return stats(period); }, () => {}).refresh();
await tick(); assert.equal(restored, "month", "Recreating the Nest retains its period without retaining statistics data");
for (const lang of ["pt-BR", "es", "zh"]) { setLang(lang); view.relabel(); assert.ok(!text().includes("No dictation recorded in this period.")); }
console.log("check-dictation-statistics: loading/empty/error/stale/retry, period races, measured values, app text safety, partial coverage, keyboard focus and languages");
