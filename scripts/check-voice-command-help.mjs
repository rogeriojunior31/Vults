// Exercise the shared read-only guide without desktop, microphone or provider access.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";
class Element {
  children = []; listeners = {}; dataset = {}; attributes = {}; value = "";
  constructor(tag) { this.tag = tag; }
  set textContent(value) { this.text = value; this.children = []; }
  get textContent() { return (this.text ?? "") + this.children.map(node => node.textContent).join(""); }
  append(...nodes) { this.children.push(...nodes); }
  replaceChildren(...nodes) { this.children = nodes; }
  addEventListener(name, fn) { this.listeners[name] = fn; }
  setAttribute(name, value) { this.attributes[name] = value; }
  querySelector(tag) { return this.children.find(node => node.tag === tag); }
}
globalThis.document = { createElement: tag => new Element(tag), documentElement: {} };
const root = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({ configFile: false, root, logLevel: "silent", plugins: [{
  name: "guide-language", transform(code, id) {
    if (id.endsWith("/voice-command-help.ts")) return code + '\nexport { setLang } from "./i18n";';
  },
}], build: { write: false, minify: false, lib: { entry: `${root}src/voice-command-help.ts`, formats: ["es"] } } });
const { voiceCommandHelp, setLang } = await import(`data:text/javascript;base64,${Buffer.from(bundles[0].output.find(p => p.type === "chunk").code).toString("base64")}`);
const state = { query: "", scope: "all" };
const view = voiceCommandHelp(state);
const [search, scope] = view.children[1].children;
const rows = view.querySelector("dl").children;
const shown = () => rows.filter(node => node.tag === "dt" && !node.hidden).map(node => node.textContent);
assert.equal(shown().length, 8);
scope.value = "media"; scope.listeners.change();
assert.deepEqual(shown(), ["Media voice commands"]);
search.value = "OBSIDIAN"; search.listeners.input();
assert.equal(shown().length, 0);
assert.equal(view.children.at(-1).hidden, false);
scope.value = "apps"; scope.listeners.change();
assert.deepEqual(shown(), ["Application voice commands"]);
assert.equal(view.children.at(-1).hidden, true);
assert.equal(state.query, "OBSIDIAN");
assert.equal(state.scope, "apps");
assert.equal(voiceCommandHelp(state).children[1].children[0].value, "OBSIDIAN", "Repaint retains the query");
search.value = "<script>"; search.listeners.input();
assert.equal(shown().length, 0);
assert.ok(!view.children.some(node => node.tag === "script"));
for (const lang of ["pt-BR", "es", "zh"]) {
  setLang(lang);
  const localized = voiceCommandHelp({ query: "", scope: "computer" });
  assert.ok(!localized.textContent.includes("All examples"));
  assert.equal(localized.querySelector("dl").children.filter(node => node.tag === "dt" && !node.hidden).length, 1);
}
setLang("pt-BR");
const accented = voiceCommandHelp({ query: "notificacoes", scope: "flock" });
assert.equal(accented.querySelector("dl").children.filter(node => node.tag === "dt" && !node.hidden).length, 1, "Search ignores accents");
console.log("check-voice-command-help: scope/search intersection, empty recovery, retained state, inert text and localized accent-insensitive search");
