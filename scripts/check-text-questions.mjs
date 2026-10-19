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
 shortcutKeys: async () => ({talk:"Ctrl+Alt+V"}), onShortcutKeys() {},
};
const ui = normalizePath(fileURLToPath(new URL('../ui/', import.meta.url)));
const entry = 'text-question-check';
const bundles = await build({ configFile: false, root: ui, logLevel: 'silent', plugins: [{ name: entry, enforce: 'pre',
 resolveId: id => id.endsWith(entry) ? '\0'+entry : null,
 load: id => id === '\0'+entry ? `export { questionCard } from ${JSON.stringify(ui+'src/questions.ts')};
 export { connectQuestions } from ${JSON.stringify(ui+'src/question-bridge.ts')};
 export { createQuestionAgents } from ${JSON.stringify(ui+'src/surfaces/settings/question-agents.ts')};
 export { setLang } from ${JSON.stringify(ui+'src/i18n/index.ts')};`
 : id === ui+'src/bridge.ts' ? 'export const Bridge = globalThis.questionBridge;' : null,
 }], build: { write: false, minify: false, lib: { entry, formats: ['es'] } } });
const { questionCard, connectQuestions, createQuestionAgents, setLang } = await import(`data:text/javascript;base64,${Buffer.from(bundles[0].output.find(p=>p.type==='chunk').code).toString('base64')}`);
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

const spoken = (seq, phase, text = null, id = 1, token = 'spoken') => ({...view(seq, token), offer:{...view(seq, token).offer, dictation:{id,phase,text}}});
card.update(spoken(3,'listening'));
input.value='Manual'; input.fire('input');
assert.equal(named(card.element,'Send reply').disabled,true);
input.fire('keydown',{key:'Enter',preventDefault(){},stopPropagation(){}});
assert.equal(calls.length,1,'Enter during recording cannot confirm');
assert.equal(input.disabled,false,'Manual editing remains available while recording');
card.update(spoken(3,'transcribing'));
input.value='Manual edit'; input.fire('input');
input.fire('compositionstart');
card.update(spoken(3,'ready','spoken send'));
await tick(); assert.equal(input.value,'Manual edit','Final dictation waits for IME composition');
input.value='Manual edit composed'; input.fire('compositionend');
assert.equal(input.value,'Manual edit composed spoken send');
await tick();
assert.deepEqual(calls.at(-1).args,['spoken','apply-dictation','Manual edit composed spoken send',1]);
const applying = calls.at(-1);
card.update(spoken(3,'ready','spoken send'));
assert.equal(input.value,'Manual edit composed spoken send','Duplicate final snapshots cannot append twice');
assert.equal(named(card.element,'Send reply').disabled,true,'Apply acknowledgement cannot send a reply');
applying.resolve(); await tick();
card.update(view(3,'spoken'));
assert.equal(named(card.element,'Send reply').disabled,false);
assert.equal(input.value,'Manual edit composed spoken send');
card.update(spoken(4,'ready','new words',2,'overflow'));
await tick(); calls.at(-1).resolve(); await tick();
card.update(view(5,'long')); input.value='x'.repeat(1999);
card.update(spoken(5,'ready','too long',3,'long'));
assert.equal(input.value,'x'.repeat(1999),'Oversized append preserves every manual byte');
assert.equal(named(card.element,'Send reply').disabled,true);
assert.ok(card.element.textContent.includes('transcription does not fit'));
card.update(spoken(6,'listening',null,4,'cancel-capture'));
named(card.element,'Cancel dictation').fire('click'); await tick();
assert.deepEqual(calls.at(-1).args,['cancel-capture','stop-dictation','']); calls.at(-1).resolve(); await tick();
card.update(spoken(7,'ready','old private words',5,'old-capture')); await tick();
const oldApply = calls.at(-1);
card.update(view(8,'replacement')); input.value='Replacement draft';
oldApply.resolve(); await tick();
assert.equal(input.value,'Replacement draft','A late apply ack cannot replace or clear a replacement draft');
card.clear();
card.update(spoken(30,'ready','Cancelled IME words',6,'ime-cancel'));
await tick(); calls.at(-1).resolve(); await tick();
card.update(view(31,'ime-cancel-next')); input.value='Keep manual'; input.fire('compositionstart');
card.update(spoken(32,'ready','cancelled words',7,'ime-cancel-next'));
card.update(view(33,'ime-cancel-next'));
card.update(spoken(32,'ready','cancelled words',7,'ime-cancel-next'));
input.fire('compositionend');
assert.equal(input.value,'Keep manual','A stale Ready snapshot after cancellation cannot append deferred IME text');
card.clear();

card.update(spoken(40,'ready','spoken',8,'failed-apply')); await tick();
calls.at(-1).reject(new Error('lost apply acknowledgement')); await tick();
assert.equal(input.value,'spoken');
assert.equal(named(card.element,'Send reply').disabled,true);
assert.ok(card.element.textContent.includes('Could not apply'));
named(card.element,'Cancel dictation').fire('click'); await tick(); calls.at(-1).resolve(); await tick();
card.update(view(41,'failed-apply'));
assert.equal(input.value,'spoken','Cancelling dictation keeps the local draft after a lost apply acknowledgement');
assert.equal(named(card.element,'Send reply').disabled,false);
card.clear();

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
console.log('check-text-questions: ok (confirmation, IME/modifiers, UTF-8 limits, retained edits, stale ack/status, ordered departure, queued replacement, visibility, local dictation append/IME/cancel/failure, four languages)');


const settingsReads=[], installs=[], previews=[], writes=[], switches=[];
const agents=createQuestionAgents({
 settings:()=>{const d=deferred();settingsReads.push(d);return d.promise;},
 status:()=>{const d=deferred();installs.push(d);return d.promise;},
 set:on=>{const d=deferred();switches.push({...d,on});return d.promise;},
 preview:(agent,install)=>{const d=deferred();previews.push({...d,agent,install});return d.promise;},
 apply:(agent,install,fingerprint)=>{const d=deferred();writes.push({...d,agent,install,fingerprint});return d.promise;},
});
document.body.append(agents.element);agents.visible(true);
const statusRows=[{agent:'claude',path:'/home/test/.claude.json',installed:false,error:null},{agent:'codex',path:'/home/test/.codex/config.toml',installed:true,error:null}];
settingsReads[0].resolve({questionAgents:false});installs[0].resolve(statusRows);await tick();
let control=agents.element.querySelector('input');assert.equal(control.checked,false);
let setup=named(agents.element,'Review setup…');assert.equal(setup.disabled,true,'Default-off access cannot install');
assert.equal(named(agents.element,'Review removal…').disabled,false,'Removal remains available while access is off');
control.checked=true;control.fire('change');await tick();assert.equal(switches.length,1);assert.equal(writes.length,0);
switches[0].resolve();await tick();assert.equal(agents.element.querySelector('input').checked,true);
named(agents.element,'Review setup…').fire('click');await tick();assert.equal(previews.length,1);assert.equal(writes.length,0,'Review cannot write');
previews[0].resolve({diff:'+ literal <script> stays inert',fingerprint:'seen'});await tick();
assert.equal(nodes(agents.element).filter(n=>n.tagName==='SCRIPT').length,0);
assert.ok(named(agents.element,'Write the file'));
named(agents.element,'Write the file').fire('click');named(agents.element,'Write the file')?.fire('click');await tick();
assert.equal(writes.length,1,'A reviewed click consumes one config plan');
assert.deepEqual([writes[0].agent,writes[0].install,writes[0].fingerprint],['claude',true,'seen']);
writes[0].resolve('/tmp/config.bak-2026');await tick();
settingsReads.at(-1).resolve({questionAgents:true});installs.at(-1).resolve(statusRows.map(s=>({...s,installed:true})));await tick();
assert.ok(agents.element.textContent.includes('/tmp/config.bak-2026'));
let row=nodes(agents.element).find(n=>n.tagName==='SECTION'&&n.textContent.includes('Claude Code'));
named(row,'Review removal…').fire('click');await tick();agents.visible(false);
previews.at(-1).resolve({diff:'- old',fingerprint:'old'});await tick();assert.equal(named(agents.element,'Write the file'),undefined,'Departure cannot restore a config plan');
agents.visible(true);agents.changed(false);
settingsReads.at(-1).resolve({questionAgents:true});installs.at(-1).resolve(statusRows);await tick();
assert.equal(agents.element.querySelector('input').checked,false,'A settings event beats an older bootstrap read');
assert.ok(named(agents.element,'Review setup…'),'A consent event cannot swallow the status read');
agents.changed(true);named(agents.element,'Review setup…').fire('click');await tick();agents.changed(false);
previews.at(-1).resolve({diff:'+ stale consent',fingerprint:'stale'});await tick();
assert.equal(named(agents.element,'Write the file'),undefined,'Consent revocation invalidates a late config preview');
const beforeReads=settingsReads.length;
agents.refresh();agents.refresh();
settingsReads[beforeReads+1].resolve({questionAgents:false});installs[beforeReads+1].resolve(statusRows.map(s=>({...s,installed:true})));await tick();
settingsReads[beforeReads].resolve({questionAgents:true});installs[beforeReads].resolve(statusRows);await tick();
assert.equal(nodes(agents.element).filter(n=>n.tagName==='BUTTON'&&n.textContent==='Review removal…').length,2,'An older refresh cannot overwrite newer installed status');
assert.equal(agents.element.querySelector('input').checked,false);
for(const lang of ['pt-BR','es','zh']){setLang(lang);agents.relabel();assert.ok(!agents.element.textContent.includes('Agents may ask for a spoken answer'));}
setLang('en');agents.visible(false);
console.log('Agent question settings: default off, reviewed single write, backup, removal while off, inert diff, navigation and consent races, translations pass');
