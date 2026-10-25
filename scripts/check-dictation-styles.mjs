// Exercise production clean-up controls without desktop access or provider requests.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";
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
  replaceChildren(...children) { if (globalThis.document?.activeElement && document.activeElement !== this && this.contains(document.activeElement)) document.activeElement = document.body; this.text = ""; for (const n of this.children) n.parentElement = null; this.children = []; this.append(...children); }
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
  focus() { if (!this.disabled) document.activeElement = this; }
  setSelectionRange(start, end) { this.selectionStart = start; this.selectionEnd = end; }
  getContext() { return { clearRect() {}, fillRect() {} }; }
}
globalThis.document = { body: new Element("body"), documentElement: new Element("html"), createElement: tag => new Element(tag), createElementNS: (_, tag) => new Element(tag) };
globalThis.window = { setTimeout: () => 0, clearTimeout() {} };
const ui = fileURLToPath(new URL("../ui/", import.meta.url));
const bundle = await build({ configFile: false, root: ui, logLevel: "silent", plugins: [{ name: "style-fixture", transform(code, id) {
  if (id.endsWith("/settings/dictation-styles.ts")) return `${code}\nexport { setLang } from "../../i18n";`;
} }], build: { write: false, minify: false, lib: { entry: `${ui}src/surfaces/settings/dictation-styles.ts`, formats: ["es"] } } });
const { createCleanupSettings, setLang } = await import(`data:text/javascript;base64,${Buffer.from(bundle[0].output.find(p => p.type === "chunk").code).toString("base64")}`);
const tick = () => new Promise(resolve => setImmediate(resolve));
let config = { dictationCleanup: false, dictationStyle: "natural", dictationWriteIn: null, dictationSessionsEnglish: false };
let providers = { selected: "local", providers: [{ id: "local", label: "Local <inert>", model: "fixture", local: true, hasKey: false }] };
let apps = {}, reads = 0, providerReads = 0, links = 0, writes = [], hold, failRead = false, failApps = false, failAppWrite = false;
const pending = [];
const backend = { load: async () => { reads++; if (failRead) throw Error("Unavailable"); return hold ?? structuredClone(config); }, providers: async () => { providerReads++; return structuredClone(providers); }, apps: async () => { if (failApps) throw Error("Unreadable"); return { ...apps }; },
  enable: on => { writes.push(["enable",on]); return new Promise((resolve,reject) => pending.push({ resolve: () => { config.dictationCleanup=on; resolve(); },reject })); },
  options: async (style,writeIn,english) => { writes.push(["options",style,writeIn,english]); config={dictationCleanup:config.dictationCleanup,dictationStyle:style,dictationWriteIn:writeIn,dictationSessionsEnglish:english}; },
  app: async (identity,style) => { writes.push(["app",identity,style]); if (failAppWrite) throw Error("Not saved"); if (style === null) delete apps[identity]; else apps[identity]=style; },
};
const view = createCleanupSettings(backend, () => links++); document.body.append(view.element);
const find = key => nodes(view.element).find(n => n.dataset.styleControl === key);
const click = key => { const n=find(key); assert.ok(n,key); if (!n.disabled) n.fire("click"); };
const change = (key,value) => { const n=find(key); assert.ok(n,key); if (!n.disabled) { if (n.tagName === "INPUT") n.checked=value; else n.value=value; n.fire("change"); } };
assert.equal(reads,0,"Constructing a hidden settings page does not read settings");
let late; hold=new Promise(resolve => late=resolve);
view.visible(true); assert.equal(writes.length,0);
view.update({dictationCleanup:true,dictationStyle:"polished"});
late(config); hold=null; await tick();
assert.equal(find("cleanup").checked,true,"A settings event wins over an older startup response");
assert.equal(find("style-polished").getAttribute("aria-pressed"),"true");
assert.equal(view.element.querySelector("details").open,false);
assert.ok(view.element.textContent.includes("Local <inert>"));
assert.ok(view.element.textContent.includes("2,000 attempts"));
assert.equal(writes.length,0,"Reading settings/provider/overrides does not enable cleanup");
find("cleanup").focus(); view.element.parentElement.scrollTop=240;
change("cleanup",false); await tick();
assert.deepEqual(writes.at(-1),["enable",false]);
view.relabel(); assert.equal(find("cleanup").disabled,true);
change("cleanup",false); assert.equal(pending.length,1,"Pending writes cannot duplicate");
view.update({dictationStyle:"prompt"});
pending[0].resolve(); await tick();
assert.equal(find("cleanup").checked,false,"An unrelated style event does not discard the acknowledged cleanup toggle");
assert.equal(find("language"),undefined,"Off hides optional choices");
const previews = nodes(view.element).filter(node => node.tagName === "ARTICLE" && node.classList.contains("dictation-style-card"));
assert.equal(previews.length,4,"Off still explains all four styles without offering paid activation");
assert.ok(view.element.textContent.includes("Remove hesitations and repetitions"));
const beforePreview = writes.length;
previews.forEach(card => card.fire("click"));
assert.equal(writes.length,beforePreview,"Read-only previews cannot enable or configure clean-up");
assert.equal(find("style-natural"),undefined,"Style mutation controls remain absent while off");
assert.equal(document.activeElement,find("cleanup"));
assert.equal(view.element.parentElement.scrollTop,240);
change("cleanup",true); await tick(); pending[1].reject(Error("budget unavailable")); await tick();
assert.equal(find("cleanup").checked,false); assert.ok(view.element.textContent.includes("Local dictation still works"));
change("cleanup",true); await tick(); pending[2].resolve(); await tick();
assert.equal(find("cleanup").checked,true);
click("style-prompt"); await tick(); assert.deepEqual(writes.at(-1),["options","prompt",null,false]);
change("language","1"); await tick(); assert.deepEqual(writes.at(-1),["options","prompt","en",false]);
change("english",true); await tick(); assert.deepEqual(writes.at(-1),["options","prompt","en",true]);
const details=view.element.querySelector("details"); details.open=true; details.fire("toggle");
find("app-name").value="App <script>"; find("app-name").fire("input"); click("app-save"); await tick();
assert.deepEqual(writes.at(-1),["app","App <script>","natural"]);
assert.ok(view.element.textContent.includes("App <script>"));
assert.equal(view.element.querySelectorAll("script").length,0,"App identities are inert text");
assert.equal(view.element.querySelector("details").open,true);
change("app-konsole","3"); await tick(); assert.deepEqual(writes.at(-1),["app","konsole","polished"]);
change("app-konsole","0"); await tick(); assert.deepEqual(writes.at(-1),["app","konsole",null]);
failAppWrite=true;
find("app-name").value="Retained draft"; find("app-name").fire("input"); find("app-save").focus(); click("app-save"); await tick();
assert.equal(find("app-name").value,"Retained draft");
assert.ok(view.element.querySelector("details").textContent.includes("Could not confirm the change"),"App errors stay beside the app controls");
failAppWrite=false; click("app-save"); await tick();
assert.equal(find("app-name").value,"");assert.equal(document.activeElement,find("app-name"),"Saving a new app returns to the next usable input");
failApps=true; view.refresh(); await tick(); assert.ok(view.element.textContent.includes("last list may be out of date"));
assert.equal(find("app-code").disabled,true);
assert.equal(find("cleanup").disabled,false,"An app-list read failure cannot block turning cleanup off");
failApps=false; view.refresh(); await tick();
failRead=true; view.refresh(); await tick(); assert.ok(view.element.textContent.includes("Could not read clean-up settings"));
assert.equal(find("cleanup").disabled,false,"A stale settings view still permits an explicit disable");
failRead=false;
change("cleanup",false); await tick(); pending[3].resolve(); await tick();
providers={selected:"cloud",providers:[{id:"cloud",label:"Cloud",model:"fixture",local:false,hasKey:false}]};
view.refresh(); await tick(); assert.equal(find("cleanup").disabled,true,"Missing key cannot authorize cleanup");
assert.ok(view.element.textContent.includes("dictated text leaves this computer")); click("provider"); assert.equal(links,1);
for (const language of ["pt-BR","es","zh"]) { setLang(language);view.relabel();assert.ok(!view.element.textContent.includes("Clean up with a model")); }
setLang("en");
assert.equal(writes.filter(w=>w[0]==="enable").length,4);
assert.ok(providerReads>0);
console.log("check-dictation-styles: explicit consent, provider disclosure, stale reads/events, pending writes, failure/retry, language/styles, overrides, inert app text and focus/scroll");
