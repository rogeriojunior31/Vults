// Meeting settings and notes use the production renderers with deferred local responses.
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
  focus() { document.activeElement = this; }
  setSelectionRange(start, end) { this.selectionStart = start; this.selectionEnd = end; }
  getContext() { return { clearRect() {}, fillRect() {} }; }
  get options() { return this.children.filter(n => n.tagName === 'OPTION'); }
}
const timers = new Map(); let timer = 0;
globalThis.document = { body: new Element('body'), documentElement: new Element('html'), hidden: false,
  createElement: tag => new Element(tag), createElementNS: (_ns, tag) => new Element(tag), createTextNode: text => { const n=new Element('text'); n.textContent=text; return n; }, addEventListener() {} };
globalThis.window = { setInterval: fn => { timers.set(++timer,fn); return timer; }, clearInterval: id => timers.delete(id), addEventListener() {} };
const ui=normalizePath(fileURLToPath(new URL('../ui/',import.meta.url)));
const entry='meeting-summary-check';
const bundles=await build({configFile:false,root:ui,logLevel:'silent',plugins:[{name:'meeting-summary-check',resolveId:id=>id.endsWith(entry)?'\0'+entry:null,load:id=>id==='\0'+entry?`
export { createMeetingSettings } from ${JSON.stringify(ui+'src/surfaces/settings/meeting-summary.ts')};
export { createMeetingSummary } from ${JSON.stringify(ui+'src/surfaces/nest/dictation/summary.ts')};
export { createMeetings } from ${JSON.stringify(ui+'src/surfaces/nest/dictation/meetings.ts')};
export { createMeetingArchive } from ${JSON.stringify(ui+'src/surfaces/nest/dictation/archive.ts')};
export { setLang } from ${JSON.stringify(ui+'src/i18n/index.ts')};`:null}],build:{write:false,minify:false,lib:{entry,formats:['es']}}});
const {createMeetingSettings,createMeetingSummary,createMeetings,createMeetingArchive,setLang}=await import(`data:text/javascript;base64,${Buffer.from(bundles[0].output.find(p=>p.type==='chunk').code).toString('base64')}`);
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
assert.ok(summary.element.querySelector('details').open,'Failure opens the disclosure containing Reload');
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
let longStatus={id:12,phase:'recording',elapsedMs:75*60*1000,maxSeconds:14400,microphone:'fixture mic',monitor:'fixture monitor',hasTranscript:false,modelId:'base',language:'en',error:null};
const longMeeting=createMeetings({status:async()=>longStatus,end:async()=>({...longStatus,phase:'cancelled'})},()=>{},()=>{});
document.body.append(longMeeting.element);longMeeting.setActive(true);await tick();await tick();
assert.ok(longMeeting.element.textContent.includes('Record up to four hours.'));
assert.ok(longMeeting.element.classList.contains('meeting-before-transcript'));
assert.ok(!meeting.element.classList.contains('meeting-before-transcript'));
const longClock=nodes(longMeeting.element).find(n=>n.attributes['aria-label']==='Recording time');
assert.equal(longClock.textContent,'1:15:00');
longStatus={...longStatus,phase:'transcribing',progress:{track:'system',completedMs:75*60*1000,totalMs:4*60*60*1000,stage:'transcription'}};
for(const callback of timers.values()) callback();await tick();await tick();
assert.ok(longMeeting.element.textContent.includes('1:15:00 of 4:00:00 processed'));
longMeeting.setActive(false);await tick();assert.equal(timers.size,0);
console.log('Meeting durations: four-hour disclosure, hour clocks and long-track progress pass');
let initialPrefsFail=true;
const recovery=createMeetingSummary({...api,settings:async()=>{if(initialPrefsFail)throw Error('Unreadable settings');return {meetingSummary:false};}},()=>routed++);
document.body.append(recovery.element);recovery.setActive(true);await tick();
const recoveryDetails=recovery.element.querySelector('details');assert.ok(!recoveryDetails.hidden&&recoveryDetails.open,'Initial failure exposes recovery even before a transcript');
click(recovery.element,'Reload summary status');initialPrefsFail=false;await tick();click(recovery.element,'Reload summary status');await tick();
assert.ok(!recoveryDetails.hidden,'Consent off still exposes Meeting settings');assert.ok(!named(recovery.element,'Meeting settings').hidden);recovery.setActive(false);

const saved={id:'meeting-1',recorded_ms:1700000000000,title:'Literal <img> caf\u00e9 meeting',
 microphone:{duration_ms:3000,segments:[{start_ms:0,end_ms:500,text:'Private microphone <script> text',speaker:0},{start_ms:500,end_ms:1000,text:'Uncertain words',speaker:null}]},
 system:{duration_ms:5000,segments:[{start_ms:2000,end_ms:2500,text:'Independent system clock',speaker:0}]},
 speakers:[{source:'microphone',id:0,name:null},{source:'system',id:0,name:'Different person'}],notes};
let savedRows=[saved], nextRead=null, nextList=null, renameRequests=[], exportRequests=[], deleteRequests=[], saveRequests=[];
const archiveApi={
 save:(id,title)=>{const call={...deferred(),id,title};saveRequests.push(call);return call.promise;},
 list:async()=>nextList?nextList.promise:{epoch:0,rows:savedRows.map(({id,title,recorded_ms})=>({id,title,recorded_ms}))},
 read:async id=>nextRead?nextRead.promise:{epoch:0,meeting:structuredClone(savedRows.find(m=>m.id===id)??null)},
 rename:(id,source,speaker,name)=>{const call={...deferred(),id,source,speaker,name};renameRequests.push(call);return call.promise;},
 remove:id=>{const call={...deferred(),id};deleteRequests.push(call);return call.promise;},
 export:async(...args)=>{exportRequests.push(args);return false;},
};
const archiveMedia = { matches: false, addEventListener(_name, callback) { this.changed = callback; } };
window.matchMedia = () => archiveMedia;
const archive=createMeetingArchive(archiveApi);document.body.append(archive.element);archive.setActive(true);await tick();
assert.ok(archive.element.textContent.includes(saved.title));assert.equal(renameRequests.length+exportRequests.length+deleteRequests.length,0,'Reading never mutates or exports');
const accentSearch = nodes(archive.element).find(n=>n.tagName==='INPUT'&&n.type==='search');
accentSearch.value='CAFE';accentSearch.fire('input');
assert.ok(archive.element.textContent.includes(saved.title), 'Meeting title search ignores case and accents');
accentSearch.value='';accentSearch.fire('input');
const search=nodes(archive.element).find(n=>n.tagName==='INPUT'&&n.type==='search');search.value='absent';search.fire('input');assert.ok(archive.element.textContent.includes('No saved meetings match'));
click(archive.element,'Clear search');assert.equal(search.value,'');assert.equal(document.activeElement,search);
search.value='absent';search.fire('input');
for (const composing of [{ isComposing: true }, { keyCode: 229 }]) {
 search.fire('keydown',{key:'Escape',...composing,preventDefault(){assert.fail('Composition belongs to the input method');}});
 assert.equal(search.value,'absent');
}
search.fire('keydown',{key:'Escape',preventDefault(){},stopPropagation(){}});assert.equal(search.value,'');
const archiveBody = archive.element.querySelector('.meeting-body');
const archiveSidebar = archive.element.querySelector('.meeting-archive-sidebar');
const archiveList = archive.element.querySelector('.meeting-archive-list');
const replaceList = archiveList.replaceChildren.bind(archiveList);
// Removing the list can clamp browser scroll offsets before replacement rows are appended.
archiveList.replaceChildren = (...children) => { archiveBody.scrollTop = 0; archiveSidebar.scrollTop = 0; replaceList(...children); };
archiveBody.scrollTop = 320;
named(archive.element,saved.title+new Date(saved.recorded_ms).toLocaleString('en',{dateStyle:'medium',timeStyle:'short'})).fire('click');await tick();
assert.equal(search.hidden,true,'Narrow detail keeps its existing return path');
archiveMedia.matches=true;archiveMedia.changed();
assert.equal(search.hidden,false,'Wide detail preserves the searchable meeting list');
assert.equal(archive.element.querySelector('.meeting-archive-list').hidden,false);
assert.equal(archive.element.querySelector('.meeting-archive-row').getAttribute('aria-pressed'),'true');
archiveMedia.matches=false;archiveMedia.changed();assert.equal(search.hidden,true,'Narrow resize preserves the current detail');
assert.ok(archive.element.textContent.includes('Private microphone <script> text'));assert.ok(!archive.element.textContent.includes('Independent system clock'));
archiveBody.scrollTop = 700;
nextList = deferred();
click(archive.element, 'Back to saved meetings');
assert.equal(archiveBody.scrollTop, 320, 'Returning from a long transcript restores the list position');
assert.equal(document.activeElement.dataset.meetingId, saved.id, 'Back returns keyboard focus to the originating meeting');
search.focus();
nextList.resolve({epoch:0, rows:savedRows.map(({id,title,recorded_ms})=>({id,title:title+' updated',recorded_ms}))});
await tick(); nextList = null;
assert.equal(document.activeElement, search, 'Late list refresh never steals newer focus');
assert.equal(archiveBody.scrollTop, 320);
archive.open(saved.id); await tick();
archiveMedia.matches = true; archiveMedia.changed();
archiveSidebar.scrollTop = 180;
archive.element.querySelector('.meeting-archive-row').focus();
setLang('es'); archive.relabel();
assert.equal(archiveSidebar.scrollTop, 180, 'Relabeling preserves the independent wide list scroll');
assert.equal(document.activeElement.dataset.meetingId, saved.id, 'Relabeling preserves meeting identity for keyboard focus');
setLang('en'); archive.relabel();
archiveMedia.matches = false; archiveMedia.changed();

assert.ok(archive.element.textContent.includes('Unknown voice'));assert.equal(nodes(archive.element).filter(n=>['IMG','SCRIPT'].includes(n.tagName)).length,0);
click(archive.element,'System audio');assert.ok(archive.element.textContent.includes('Different person'));assert.ok(archive.element.textContent.includes('00:00:02.000'));
click(archive.element,'Microphone');
let nameInput=nodes(archive.element).find(n=>n.attributes['aria-label']==='Name for Voice 1 (Microphone)');
nameInput.value='<b>Person</b>';nameInput.fire('input');click(archive.element,'Save name');named(archive.element,'Save name').fire('click');
assert.equal(renameRequests.length,1,'Renaming is single-owner');assert.deepEqual([renameRequests[0].source,renameRequests[0].speaker,renameRequests[0].name],['microphone',0,'<b>Person</b>']);
renameRequests[0].reject(Error('private database failure'));await tick();assert.equal(nameInput.value,'<b>Person</b>');assert.ok(archive.element.textContent.includes('Your draft is kept'));assert.ok(!archive.element.textContent.includes('private database failure'));
for(const lang of ['pt-BR','es','zh']){setLang(lang);archive.relabel();assert.ok(!named(archive.element,'Saved meetings'));assert.ok(nodes(archive.element).some(n=>n.tagName==='INPUT'&&n.value==='<b>Person</b>'),'Relabel keeps drafts');}
setLang('en');archive.relabel();nameInput=nodes(archive.element).find(n=>n.attributes['aria-label']==='Name for Voice 1 (Microphone)');
click(archive.element,'Save name');renameRequests[1].resolve(true);await tick();assert.ok(archive.element.textContent.includes('<b>Person</b>'));assert.equal(nodes(archive.element).filter(n=>n.tagName==='B').length,0);
click(archive.element,'Summary');assert.ok(archive.element.textContent.includes(notes.summary));click(archive.element,'To-dos');assert.ok(archive.element.textContent.includes('Owner: Not assigned'));
const selectors=nodes(archive.element).filter(n=>n.tagName==='SELECT');selectors[0].value='srt';selectors[0].fire('change');assert.ok(named(archive.element,'Export…').disabled,'Subtitles require an explicit source');
selectors[1].value='system';selectors[1].fire('change');click(archive.element,'Export…');await tick();assert.deepEqual(exportRequests[0],['meeting-1','srt','system','Export meeting']);assert.ok(archive.element.textContent.includes('Export cancelled. No file was written.'));
nextRead=deferred();click(archive.element,'Reload');click(archive.element,'Delete…');assert.equal(deleteRequests.length,0,'Delete requires confirmation');click(archive.element,'Keep meeting');assert.equal(deleteRequests.length,0);
click(archive.element,'Delete…');click(archive.element,'Delete saved meeting');named(archive.element,'Delete saved meeting').fire('click');assert.equal(deleteRequests.length,1);
savedRows=[];deleteRequests[0].resolve(true);await tick();nextRead.resolve({epoch:0,meeting:saved});await tick();nextRead=null;
assert.ok(!archive.element.textContent.includes(saved.title),'A pre-delete detail read cannot restore erased text');assert.ok(archive.element.textContent.includes('No saved meetings yet.'));
savedRows=[saved];nextList=deferred();click(archive.element,'Reload');search.value='Private meeting title';search.fire('input');archive.invalidate();nextList.resolve({epoch:0,rows:[saved]});await tick();nextList=null;assert.ok(!archive.element.textContent.includes(saved.title),'Global erasure invalidates pending list reads');assert.equal(search.value,'','Erasure/departure also clears private search text');
archive.setActive(false);

const full=createMeetings({archive:archiveApi,summary:api,transcript:async()=>({id:7,microphone:{durationMs:1000,segments:[{startMs:0,endMs:500,text:'Current text'}]},system:{durationMs:0,segments:[]}}),
 status:async()=>({id:7,phase:'finished',hasTranscript:true,elapsedMs:1000}),prepare:async()=>{throw Error('No implicit review');},begin:async()=>{throw Error('No implicit recording');},end:async(id,cancel)=>({id,phase:'cancelled',hasTranscript:false}),recover:async()=>{throw Error('No implicit recovery');}});
document.body.append(full.element);full.setActive(true);await tick();await tick();
const titleInput=nodes(full.element).find(n=>n.attributes['aria-label']==='Meeting title');titleInput.value='Explicit title';titleInput.fire('input');click(full.element,'Save meeting');named(full.element,'Saving meeting…').fire('click');assert.equal(saveRequests.length,1);assert.equal(saveRequests[0].title,'Explicit title');
saveRequests[0].resolve('meeting-1');await tick();assert.ok(full.element.textContent.includes('Further changes on this recording do not update'));
click(full.element,'View saved meeting');await tick();assert.ok(full.element.textContent.includes(saved.title));full.setActive(false);assert.equal(timers.size,0);
console.log('Meeting archive: explicit snapshot/save, inert source-local detail/notes, name drafts and failures, export source/cancellation, confirmed single delete, stale reads/global erase and translations pass');
