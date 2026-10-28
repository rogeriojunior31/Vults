// Exercise the catalog without desktop, microphone or provider access.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";
const nodes = node => [node, ...node.children.flatMap(nodes)];
class Element {
  children = []; listeners = {}; dataset = {}; attributes = {}; value = ""; className = ""; hidden = false;
  constructor(tag) { this.tag = tag; }
  set textContent(value) { this.text = value; this.children = []; }
  get textContent() { return (this.text ?? "") + this.children.map(node => node.textContent).join(""); }
  append(...children) { this.children.push(...children); }
  replaceChildren(...children) { this.text = ""; this.children = children; }
  addEventListener(name, fn) { this.listeners[name] = fn; }
  setAttribute(name, value) { this.attributes[name] = value; }
  querySelectorAll(selector) { return nodes(this).slice(1).filter(node => selector.startsWith(".") ? node.className.split(" ").includes(selector.slice(1)) : selector.startsWith("[data-setting=") ? node.dataset.setting === selector.slice(15, -2) : node.tag === selector); }
  querySelector(selector) { return this.querySelectorAll(selector)[0] ?? null; }
  focus() { document.activeElement = this; }
}
globalThis.document = { createElement: tag => new Element(tag), createElementNS: (_, tag) => new Element(tag), documentElement: {} };
const root = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({ configFile: false, root, logLevel: "silent", plugins: [{
  name: "guide-language", transform(code, id) {
    if (id.endsWith("/voice-command-help.ts")) return code + '\nexport { setLang } from "./i18n";';
  },
}], build: { write: false, minify: false, lib: { entry: `${root}src/voice-command-help.ts`, formats: ["es"] } } });
const { voiceCommandHelp, setLang } = await import(`data:text/javascript;base64,${Buffer.from(bundles[0].output.find(p => p.type === "chunk").code).toString("base64")}`);
const state = { query: "", scope: "move" }, chosen = [];
const view = voiceCommandHelp(state, phrase => chosen.push(phrase));
const control = key => nodes(view).find(n => n.dataset.setting === key);
const detail = () => view.querySelector(".command-category-detail");
const categoryNames = () => view.querySelectorAll(".command-category").map(n => n.textContent);
const search = value => { control("command-example-search").value = value; control("command-example-search").listeners.input(); };
assert.equal(categoryNames().length, 8);
assert.equal(detail().querySelector("h4").textContent, "Move around");
assert.equal(view.querySelectorAll(".command-category-detail").length, 1, "Only the selected category's content is shown");
assert.deepEqual(chosen, [], "Browsing never selects or runs a phrase");
control("command-example-move-1").listeners.click();
assert.deepEqual(chosen, ["open api"], "An example is passed to the editor only");
search("OBSIDIAN");
assert.deepEqual(categoryNames(), ["Applications"]);
assert.equal(state.scope, "apps", "Search finds a category beyond the previous selection");
assert.equal(detail().querySelector("h4").textContent, "Application voice commands");
assert.equal(voiceCommandHelp(state).querySelector("input").value, "OBSIDIAN", "Repaint retains the query and selection");
search("<script>");
assert.equal(categoryNames().length, 0);
assert.equal(view.querySelector(".command-catalog-empty").hidden, false);
assert.ok(!nodes(view).some(node => node.tag === "script"));
control("command-example-search").listeners.keydown({ key: "Escape", preventDefault() {}, stopPropagation() {} });
assert.equal(state.query, ""); assert.equal(categoryNames().length, 8);
const picker = control("command-example-scope"); picker.value = "computer"; picker.listeners.change();
assert.equal(state.scope, "computer");
assert.equal(detail().querySelector("h4").textContent, "System audio voice commands");
const notes = view.querySelector(".command-category-notes"); notes.open = true; notes.listeners.toggle();
const repainted = voiceCommandHelp(state);
assert.equal(repainted.querySelector(".command-category-notes").open, true, "Ordinary view updates preserve expanded requirements");
const categories = view.querySelector(".command-categories"), first = view.querySelectorAll(".command-category")[0];
first.focus(); categories.listeners.keydown({ key: "End", preventDefault() {} });
assert.equal(document.activeElement.dataset.setting, "command-category-safety");
assert.equal(state.scope, "computer", "Arrow navigation does not activate another category");
for (const lang of ["pt-BR", "es", "zh"]) {
  setLang(lang);
  const localized = voiceCommandHelp({ query: "", scope: "computer" });
  assert.ok(!localized.querySelector("h4").textContent.includes("System audio voice commands"));
  assert.equal(localized.querySelectorAll(".command-category-detail").length, 1);
}
setLang("pt-BR");
const accented = voiceCommandHelp({ query: "notificacoes ocultar", scope: "move" });
assert.equal(accented.querySelectorAll(".command-category").length, 1, "Search ignores accents");
console.log("check-voice-command-help: one-category catalog, global search, retained selection/notes, edit-only examples, keyboard, empty recovery, inert text and translations");
