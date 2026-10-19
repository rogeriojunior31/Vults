// Typed questions retain drafts and reject stale/IME input on the production components.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build, normalizePath } from "vite";
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
  focus() { document.activeElement = this; this.fire("focus"); }
  blur() { if (document.activeElement === this) { document.activeElement = null; this.fire("blur"); } }
  setSelectionRange(start, end) { this.selectionStart = start; this.selectionEnd = end; }
  getContext() { return { clearRect() {}, fillRect() {} }; }
  get options() { return this.children.filter(n => n.tagName === 'OPTION'); }
}

const events = {}, timers = new Map(); let timer = 0;
globalThis.document = { body: new Element('body'), documentElement: new Element('html'), hidden: false,
 createElement: tag => new Element(tag), createTextNode: text => { const n = new Element('text'); n.textContent = text; return n; },
 addEventListener: (name, fn) => (events[name] ??= []).push(fn) };
globalThis.window = { setInterval: fn => { timers.set(++timer, fn); return timer; }, addEventListener: (name, fn) => (events[name] ??= []).push(fn) };
const deferred = () => { let resolve, reject; const promise = new Promise((a,b) => { resolve=a; reject=b; }); return { promise, resolve, reject }; };
const reads = [], leaves = []; let event;
globalThis.questionBridge = {
 textQuestionStatus: token => { const call = { ...deferred(), token }; reads.push(call); return call.promise; },
 textQuestionAction: async () => {}, textQuestionDepart: () => { const call = deferred(); leaves.push(call); return call.promise; },
 onTextQuestion: cb => { event = cb; },
};
const ui = normalizePath(fileURLToPath(new URL('../ui/', import.meta.url)));
const entry = 'text-question-check';
const bundles = await build({ configFile: false, root: ui, logLevel: 'silent', plugins: [{ name: entry, enforce: 'pre',
 resolveId: id => id.endsWith(entry) ? '\0'+entry : null,
 load: id => id === '\0'+entry ? `export { questionCard } from ${JSON.stringify(ui+'src/questions.ts')};
 export { connectQuestions } from ${JSON.stringify(ui+'src/question-bridge.ts')};
 export { setLang } from ${JSON.stringify(ui+'src/i18n/index.ts')};`
 : id === ui+'src/bridge.ts' ? 'export const Bridge = globalThis.questionBridge;' : null,
 }], build: { write: false, minify: false, lib: { entry, formats: ['es'] } } });
const { questionCard, connectQuestions, setLang } = await import(`data:text/javascript;base64,${Buffer.from(bundles[0].output.find(p=>p.type==='chunk').code).toString('base64')}`);
const tick = () => new Promise(resolve => setImmediate(resolve));
const view = (seq, token = 'a') => ({ seq, surface: 'island', phase: 'pending', offer: { token, question: '<img src=x> Which branch?', draft: '', clientPid: 42, seconds: 119 } });
const named = (root, text) => nodes(root).find(n => n.tagName==='BUTTON' && n.textContent===text);
const calls = [], keys = [];
const card = questionCard({ act: (...args) => { const call = { ...deferred(), args }; calls.push(call); return call.promise; }, keyboard: on=>keys.push(on), changed() {} });
document.body.append(card.element);
card.update(view(1));
const input = card.element.querySelector('textarea');
assert.equal(nodes(card.element).filter(n=>n.tagName==='IMG').length, 0, 'Question markup remains inert');
input.value = 'My manual draft'; input.fire('input');
card.update(view(1)); assert.equal(input.value, 'My manual draft', 'Polling does not replace local edits');
for (const modifier of ['shiftKey','ctrlKey','altKey','metaKey','isComposing','repeat']) input.fire('keydown', { key:'Enter', [modifier]:true });
input.fire('keydown', { key:'Enter', keyCode:229 });
input.fire('compositionstart'); input.fire('keydown', { key:'Enter' }); input.fire('compositionend');
await tick(); assert.equal(calls.length,0, 'IME, modifiers and repeats cannot confirm');
input.fire('keydown', { key:'Enter', preventDefault() {}, stopPropagation() {} });
named(card.element,'Send reply').fire('click'); await tick();
assert.equal(calls.length,1, 'Enter plus a duplicate click consumes one confirmation');
assert.deepEqual(calls[0].args,['a','send','My manual draft']);
assert.equal(input.disabled,true);
card.update(view(2,'b')); input.value='New private text';
calls[0].resolve(); await tick(); assert.equal(input.value,'New private text','Old send acknowledgement cannot clear replacement draft');
for (const lang of ['pt-BR','es','zh']) { setLang(lang); card.update(view(2,'b')); assert.equal(input.value,'New private text'); assert.ok(!named(card.element,'Send reply')); }
setLang('en'); card.update(view(2,'b'));
input.value='字'.repeat(700); input.fire('input'); assert.equal(named(card.element,'Send reply').disabled,true,'Bound replies by UTF-8 bytes');
card.clear(); assert.equal(input.value,''); assert.ok(!card.element.textContent.includes('Which branch?'));
assert.equal(card.element.hidden,true);

let mounted = null;
const connection = connectQuestions('island', card => { mounted = card; if (card && !card.parentElement) document.body.append(card); }, () => {});
assert.equal(reads.length,1);
event(view(10,'old')); assert.ok(mounted);
const editor = mounted.querySelector('textarea'); editor.value='Private departed draft';
connection.depart(); assert.equal(mounted,null); assert.equal(editor.value,'');
reads[0].resolve(view(10,'old')); await tick(); assert.equal(mounted,null,'Late status cannot restore departed text');
event(view(11,'queued-replacement')); assert.equal(mounted,null,'Queued replacement waits for ordered departure');
leaves[0].resolve({seq:12,surface:'',phase:'',offer:null}); await tick();
event(view(11,'queued-replacement')); assert.equal(mounted,null,'Old replacement stays behind the departure sequence barrier');
event(view(13,'fresh')); assert.ok(mounted,'A new question after departure may be shown');
reads.at(-1).resolve({seq:12,surface:'',phase:'',offer:null}); await tick(); assert.ok(mounted,'An older status cannot hide the fresh question');
document.hidden=true; for (const fn of events.visibilitychange) fn(); assert.equal(mounted,null);
event(view(14,'hidden')); assert.equal(mounted,null,'Hidden pages never restore prompt or draft');
leaves.at(-1).resolve({seq:15,surface:'',phase:'',offer:null}); await tick();
document.hidden = false;
const before = reads.length;
const another = connectQuestions('island', card => { mounted = card; }, () => {});
event(view(20,'before-leave')); another.depart();
leaves.at(-1).resolve({seq:21,surface:'',phase:'',offer:null});
event(view(22,'after-ordered-leave')); await tick();
assert.equal(mounted,null);
reads[before].resolve(view(20,'before-leave')); await tick();
assert.equal(reads.length,before+2,'A stale in-flight status cannot swallow the post-Leave refresh');
reads.at(-1).resolve(view(22,'after-ordered-leave')); await tick();
assert.ok(mounted,'The fresh status discovers a new post-barrier question');
another.depart(); leaves.at(-1).resolve({seq:23,surface:'',phase:'',offer:null}); await tick();
console.log('check-text-questions: ok (confirmation, IME/modifiers, UTF-8 limits, retained edits, stale ack/status, ordered departure, queued replacement, visibility, four languages)');
