// Meeting settings and notes use the production renderers with deferred local responses.
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
}
const timers = new Map(); let timer = 0;
globalThis.document = { body: new Element('body'), documentElement: new Element('html'), hidden: false,
  createElement: tag => new Element(tag), createTextNode: text => { const n=new Element('text'); n.textContent=text; return n; }, addEventListener() {} };
globalThis.window = { setInterval: fn => { timers.set(++timer,fn); return timer; }, clearInterval: id => timers.delete(id), addEventListener() {} };
const ui=fileURLToPath(new URL('../ui/',import.meta.url));
const entry='/meeting-summary-check.ts';
const bundles=await build({configFile:false,root:ui,logLevel:'silent',plugins:[{name:'meeting-summary-check',resolveId:id=>id===entry?id:null,load:id=>id===entry?`
export { createMeetingSettings } from ${JSON.stringify(ui+'src/surfaces/settings/meeting-summary.ts')};
export { createMeetingSummary } from ${JSON.stringify(ui+'src/surfaces/nest/dictation/summary.ts')};
export { createMeetings } from ${JSON.stringify(ui+'src/surfaces/nest/dictation/meetings.ts')};
export { setLang } from ${JSON.stringify(ui+'src/i18n/index.ts')};`:null}],build:{write:false,minify:false,lib:{entry,formats:['es']}}});
const {createMeetingSettings,createMeetingSummary,createMeetings,setLang}=await import(`data:text/javascript;base64,${Buffer.from(bundles[0].output.find(p=>p.type==='chunk').code).toString('base64')}`);
const tick=()=>new Promise(resolve=>setImmediate(resolve));
const deferred=()=>{let resolve,reject;const promise=new Promise((a,b)=>{resolve=a;reject=b;});return {promise,resolve,reject};};
const named=(root,text)=>nodes(root).find(n=>n.tagName==='BUTTON'&&n.textContent===text);
const click=(root,text)=>{const b=named(root,text);assert.ok(b,text);assert.ok(!b.disabled&&!b.hidden,text+' must be available');b.onclick?.();b.fire('click');};
const providers={selected:'ollama',providers:[{id:'ollama',label:'Ollama',model:'fixture',local:true,hasKey:false}]};
let enabled=false, status={id:7,phase:'idle',error:null,notes:null}, starts=[], cancels=[], failStatus=false;
const api={settings:async()=>({meetingSummary:enabled}),providers:async()=>providers,
  status:async id=>{if(failStatus)throw Error('private provider failure');return {...status,id};},
  begin:id=>{const call={...deferred(),id};starts.push(call);return call.promise;},
  cancel:async id=>{cancels.push(id);status={...status,id,phase:'cancelled',error:null};}};
let routed=0;
const summary=createMeetingSummary(api,()=>routed++); document.body.append(summary.element);
summary.meeting(7,true); summary.setActive(true);await tick();
assert.ok(summary.element.textContent.includes('Meeting summaries are off'));
assert.equal(starts.length,0,'Entering the page never generates notes');
assert.ok(summary.element.textContent.includes('Opening settings leaves Meetings'));
click(summary.element,'Meeting settings');assert.equal(routed,1);
enabled=true;summary.refresh();await tick();
click(summary.element,'Generate summary');named(summary.element,'Generate summary').fire('click');assert.equal(starts.length,1,'Pending generation is single-owner');
status={...status,phase:'generating'};starts[0].resolve(status);await tick();
summary.select('todos');assert.ok(summary.element.textContent.includes('Generating meeting notes'));
click(summary.element,'Cancel summary');await tick();assert.deepEqual(cancels,[7]);
assert.ok(summary.element.textContent.includes('Your transcript is kept'));
click(summary.element,'Generate summary');
const notes={title:'<img src=x onerror=alert(1)>',summary:'Literal <script> text',key_points:['A supported point'],todos:[{text:'Discuss <button>markup</button>',owner:null,due:null}]};
status={...status,phase:'finished',notes};starts[1].resolve(status);await tick();
assert.ok(summary.element.textContent.includes('Owner: Not assigned'));assert.ok(summary.element.textContent.includes('Due: Not stated'));
assert.equal(nodes(summary.element).filter(n=>['IMG','SCRIPT'].includes(n.tagName)).length,0,'Model content is inert text');
summary.select('summary');assert.ok(summary.element.textContent.includes(notes.title));
failStatus=true;summary.refresh();await tick();
assert.ok(summary.element.textContent.includes('Could not update summary status'));
assert.ok(!summary.element.textContent.includes('private provider failure'));
assert.ok(named(summary.element,'Generate summary').disabled);assert.ok(summary.element.textContent.includes(notes.title),'Read failure retains the last notes');
failStatus=false;click(summary.element,'Reload summary status');await tick();
for(const lang of ['pt-BR','es','zh']){setLang(lang);summary.relabel();summary.select('todos');assert.ok(!summary.element.textContent.includes('Owner:'));}
setLang('en');summary.relabel();summary.select('summary');
click(summary.element,'Generate summary');summary.setActive(false);status={...status,phase:'generating',notes:null};starts[2].resolve(status);await tick();assert.ok(cancels.length>=2,'Leaving cancels a late start');

const initial=deferred(),writes=[];let first=true;
const config=createMeetingSettings({load:()=>first?(first=false,initial.promise):Promise.resolve({meetingSummary:enabled}),providers:async()=>providers,enable:on=>{const call={...deferred(),on};writes.push(call);return call.promise;}},()=>routed++);
document.body.append(config.element);config.visible(true);config.update({meetingSummary:true});initial.resolve({meetingSummary:false});await tick();
const check=()=>nodes(config.element).find(n=>n.tagName==='INPUT');
assert.ok(check().checked,'A late bootstrap never overwrites a newer settings event');assert.equal(writes.length,0);
check().checked=false;check().fire('change');assert.ok(check().disabled);config.relabel();assert.ok(check().disabled);
config.update({meetingSummary:true});writes[0].resolve();await tick();assert.ok(check().checked,'A later event beats an old save acknowledgement');
check().checked=false;check().fire('change');writes[1].reject(Error('private key'));await tick();assert.ok(check().checked);assert.ok(config.element.textContent.includes('Could not confirm the change'));assert.ok(!config.element.textContent.includes('private key'));
check().checked=false;check().fire('change');writes[2].resolve();await tick();assert.equal(check().checked,false);
for(const lang of ['pt-BR','es','zh']){setLang(lang);config.relabel();assert.ok(!config.element.textContent.includes('Enable meeting summaries'));}setLang('en');config.visible(false);

const transcript={id:7,microphone:{durationMs:100,segments:[{startMs:0,endMs:100,text:'Original words.'}]},system:{durationMs:100,segments:[]}};
const end=[];enabled=false;status={id:7,phase:'idle',notes:null,error:null};
const meeting=createMeetings({summary:api,status:async()=>({id:7,phase:'finished',elapsedMs:100,maxSeconds:60,microphone:'mic',monitor:'monitor',hasTranscript:true,modelId:'base',language:'en',error:null}),transcript:async()=>transcript,end:async(id,cancel)=>{end.push([id,cancel]);return {id,phase:'cancelled',hasTranscript:false};}},()=>{},()=>routed++);
document.body.append(meeting.element);meeting.setActive(true);await tick();await tick();
assert.equal(meeting.element.attributes["aria-busy"],"false","Background polling must not keep the entire live region busy");
const tabs=()=>nodes(meeting.element).filter(n=>n.attributes.role==='tab');
assert.equal(tabs().length,3);assert.deepEqual(tabs().map(n=>n.tabIndex),[0,-1,-1]);
tabs()[0].fire('keydown',{key:'ArrowRight',preventDefault(){}});assert.equal(tabs()[1].attributes['aria-selected'],'true');
assert.ok(meeting.element.textContent.includes('Meeting summaries are off'));
tabs()[1].fire('keydown',{key:'End',preventDefault(){}});assert.equal(tabs()[2].tabIndex,0);
meeting.setActive(false);await tick();assert.ok(end.some(([id,cancel])=>id===7&&cancel));assert.equal(timers.size,0);
console.log('Meeting summaries: explicit consent, late settings/save races, single start, cancellation, inert notes, fallback, translations, tabs and departure pass');
