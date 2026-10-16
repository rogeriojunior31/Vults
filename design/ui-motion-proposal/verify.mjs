import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import vm from 'node:vm';
import { fileURLToPath } from 'node:url';
import { SURFACE_SIZES, fitSurface, placePopover, surfaceFrames } from './layout.mjs';

assert.deepEqual(fitSurface('quick',1600,1000),SURFACE_SIZES.quick);
assert.deepEqual(fitSurface('room',1600,1000),SURFACE_SIZES.room);
assert.equal(SURFACE_SIZES.quick.height,440,'Compact uses the shorter proposed frame.');
assert.ok(SURFACE_SIZES.quick.width*1.14<=720&&SURFACE_SIZES.quick.height*1.14<=640,'The preferred Compact still fits the current logical overlay at the production zoom.');
for(const mode of Object.keys(SURFACE_SIZES))for(const width of [280,320,384,480,520,760,1120,1600])for(const height of [240,280,360,560,760,1000]){
  const fitted=fitSurface(mode,width,height);assert.ok(fitted.width<=width&&fitted.height<=height);
  const surface={left:20,top:80,...fitted};
  for(const anchorTop of [surface.top+90,surface.top+height/2,surface.top+fitted.height-24])for(const content of [120,280,1200]){
    const anchor={left:40,top:anchorTop,bottom:anchorTop+30};
    const p=placePopover(surface,anchor,content,surface.top+86,mode==='room');
    assert.ok(p.left>=0&&p.left+p.width<=surface.width,'A popup fits the surface width.');
    assert.ok(p.top>=0&&p.top+Math.min(content,312,p.maxHeight)<=surface.height,'A popup fits the surface height.');
  }
}
for(const from of Object.values(SURFACE_SIZES))for(const to of Object.values(SURFACE_SIZES))for(const frame of surfaceFrames(from,to)){
  assert.ok(parseFloat(frame.width)>=Math.min(from.width,to.width)-.01&&parseFloat(frame.width)<=Math.max(from.width,to.width)+.01);
  assert.ok(parseFloat(frame.height)>=Math.min(from.height,to.height)-.01&&parseFloat(frame.height)<=Math.max(from.height,to.height)+.01);
}

const source=readFileSync(new URL('vults-ui-proposal.html',import.meta.url),'utf8');
const tree=JSON.parse(execFileSync('python3',['-c',`
from html.parser import HTMLParser
import json,sys
class Tree(HTMLParser):
 def __init__(self):
  super().__init__();self.root={'tag':'document','attrs':{},'children':[]};self.stack=[self.root]
 def handle_starttag(self,tag,attrs):
  node={'tag':tag,'attrs':dict(attrs),'children':[]};self.stack[-1]['children'].append(node)
  if tag not in ['input','img','meta','link','br','hr']:self.stack.append(node)
 def handle_endtag(self,tag):
  for i in range(len(self.stack)-1,0,-1):
   if self.stack[i]['tag']==tag:self.stack=self.stack[:i];break
 def handle_data(self,data):
  if self.stack[-1]['tag'] not in ['script','style']:self.stack[-1]['children'].append(data)
p=Tree();p.feed(sys.stdin.read());print(json.dumps(p.root))
`],{input:source,encoding:'utf8'}));
const nodes=n=>[n,...n.children.flatMap(nodes)];
let document;
const animations=[],documentListeners=new Map();
class Element {
  children=[];attributes={};listeners={};style={setProperty(name,value){this[name]=value;}};value='';scrollHeight=40;scrollTop=0;clientHeight=200;checked=false;
  constructor(tag){this.tagName=tag.toUpperCase();}
  get className(){return this.attributes.class??'';}set className(value){this.attributes.class=value;}
  get id(){return this.attributes.id;}set id(value){this.attributes.id=value;}
  get dataset(){return Object.fromEntries(Object.entries(this.attributes).filter(([key])=>key.startsWith('data-')).map(([key,value])=>[key.slice(5),value]));}
  get hidden(){return 'hidden' in this.attributes;}set hidden(on){if(on)this.attributes.hidden='';else delete this.attributes.hidden;}
  classList={contains:name=>this.className.split(' ').includes(name),toggle:(name,on)=>{const has=this.classList.contains(name);on=on??!has;this.className=this.className.split(' ').filter(n=>n&&n!==name).concat(on?[name]:[]).join(' ');return on;},add:(...names)=>names.forEach(name=>this.classList.toggle(name,true)),remove:(...names)=>names.forEach(name=>this.classList.toggle(name,false))};
  setAttribute(name,value){this.attributes[name]=String(value);}getAttribute(name){return this.attributes[name];}
  get textContent(){return (this.text??'')+this.children.map(n=>n.textContent).join('');}set textContent(value){this.replaceChildren();this.text=String(value);}
  append(...children){for(const node of children){node.remove();node.parentElement=this;this.children.push(node);}}
  prepend(...children){for(const node of [...children].reverse()){node.remove();node.parentElement=this;this.children.unshift(node);}}
  replaceChildren(...children){for(const node of this.children)node.parentElement=null;this.children=[];this.text='';this.append(...children);}
  remove(){if(this.parentElement)this.parentElement.children=this.parentElement.children.filter(n=>n!==this);this.parentElement=null;}
  contains(node){return nodes(this).includes(node);}
  get isConnected(){return nodes(dom).includes(this)||nodes(document.body).includes(this);}
  get firstElementChild(){return this.children[0];}get firstChild(){return this.children[0];}get lastElementChild(){return this.children.at(-1);}
  matches(selector){
    const attr=selector.match(/\[([^=\]]+)(?:="([^"]*)")?\]/);if(attr&&(attr[2]===undefined?!(attr[1]in this.attributes):this.attributes[attr[1]]!==attr[2]))return false;
    selector=selector.replace(/\[[^\]]*\]/g,'');if(!selector)return true;
    if(selector.startsWith('.'))return selector.slice(1).split('.').every(name=>this.classList.contains(name));
    return this.tagName.toLowerCase()===selector;
  }
  querySelectorAll(selector){return nodes(this).slice(1).filter(node=>selector.split(',').some(selector=>{const parts=selector.trim().split(' ');if(!node.matches(parts.pop()))return false;let parent=node.parentElement;while(parts.length){const part=parts.pop();while(parent&&!parent.matches(part))parent=parent.parentElement;if(!parent)return false;parent=parent.parentElement;}return true;}));}
  querySelector(selector){return this.querySelectorAll(selector)[0]??null;}
  addEventListener(type,fn){(this.listeners[type]??=[]).push(fn);}
  dispatchEvent(event){event.target??=this;for(const fn of this.listeners[event.type]??[])fn(event);this['on'+event.type]?.(event);}
  click(){if(this.disabled)return;this.dispatchEvent({type:'click'});if(this.type==='submit'){let form=this.parentElement;while(form&&form.tagName!=='FORM')form=form.parentElement;form?.dispatchEvent({type:'submit',preventDefault(){}});}}focus(){document.activeElement=this;}
  getBoundingClientRect(){return this.classList.contains('product-nav')?{bottom:86,left:0,width:492,height:42}:{top:220,bottom:350,left:0,width:492,height:130};}
  getContext(){return new Proxy({},{get:(t,k)=>k in t?t[k]:()=>{},set:(t,k,v)=>{t[k]=v;return true;}});}
  animate(frames,options){animations.push({element:this,frames,options});return {finished:Promise.resolve(),cancel(){}};}
}
function inflate(tree){const node=new Element(tree.tag);for(const[k,v]of Object.entries(tree.attrs))node.setAttribute(k,v??'');for(const child of tree.children){if(typeof child==='string')node.text=(node.text??'')+child;else node.append(inflate(child));}return node;}
const dom=inflate(tree),root=dom.querySelector('[id="vults-ui-proposal"]');
document={body:new Element('body'),documentElement:new Element('html'),hidden:false,createElement:tag=>new Element(tag),createElementNS:(_ns,tag)=>new Element(tag),createTextNode:text=>{const n=new Element('#text');n.text=String(text);return n;},getElementById:id=>nodes(dom).find(n=>n.id===id),addEventListener:(type,fn)=>documentListeners.set(type,fn),activeElement:null};
const timers=new Map();let timerId=0,saves=0,lastSaved;
const windowListeners=new Map();
const context={document,console,performance,getComputedStyle:()=>({getPropertyValue:()=>'',borderTopLeftRadius:'0'}),Event:class{constructor(type){this.type=type;}},matchMedia:()=>({matches:false,addEventListener(){}}),devicePixelRatio:1,
  setTimeout:(fn,delay)=>{timers.set(++timerId,{fn,delay});return timerId;},clearTimeout:id=>timers.delete(id),addEventListener:(type,fn)=>{windowListeners.set(type,fn);},openai:{widgetState:process.env.PROTOTYPE_STATE?JSON.parse(process.env.PROTOTYPE_STATE):null,setWidgetState:data=>{saves++;lastSaved=JSON.parse(JSON.stringify(data));return Promise.resolve();}}};
context.window=context;vm.createContext(context);
const scripts=[...source.matchAll(/<script>([\s\S]*?)<\/script>/g)];
vm.runInContext(scripts[0][1],context);
const renderer=context.VultsProposalAssets,scenes=new Map(),sceneUpdates=new Map(),birdLooks=[];
const originalUpdate=renderer.Scene.prototype.update,originalHover=renderer.Scene.prototype.hover;
renderer.Scene.prototype.update=function(...args){scenes.set(this.canvas,this);sceneUpdates.set(this.canvas,args);return originalUpdate.apply(this,args);};
renderer.Scene.prototype.hover=function(key){birdLooks.push({canvas:this.canvas,key});return originalHover.call(this,key);};
for(const script of scripts.slice(1))vm.runInContext(script[1],context);
const find=selector=>root.querySelector(selector),all=selector=>root.querySelectorAll(selector);
const P=context.__vultsPrototype,runTimers=()=>{for(const [id,t] of [...timers]){if(t.delay<10)continue;timers.delete(id);t.fn();}};
const change=(node,value)=>{node.value=value;node.dispatchEvent({type:'change'});};
const type=(node,value)=>{node.value=value;node.dispatchEvent({type:'input'});};
const keydown=(node,key,extra={})=>node.dispatchEvent({type:'keydown',key,preventDefault(){},stopPropagation(){},...extra});
const submitEditor=()=>all('.editor-actions button').at(-1).click();
const confirmDialog=()=>all('.confirmation-actions button').at(-1).click();
const text=selector=>find(selector)?.textContent??'';
const reset=()=>{P.setScenario('ready');P.setArea('chat','quick');};

if(process.argv.includes('--restore')){
  assert.equal(saves,0,'Restoration never publishes a new state.');
  assert.equal(find('.chat-view textarea').value,'A draft that survives every layout');
  assert.equal(P.state.area,'summary','An old Compact route opens Now.');
  console.log('Saved prototype state restored without losing the draft; old routes migrate.');process.exit(0);
}
if(process.argv.includes('--coverage')){
const manifest=JSON.parse(readFileSync(new URL('features.json',import.meta.url),'utf8')).features;
/* Coverage: every feature has a place, or a reason it has none. */
const ids=new Set();
for(const f of manifest){
  assert.ok(!ids.has(f.id),'Duplicate feature '+f.id);ids.add(f.id);
  assert.ok(f.at||f.native,f.id+' needs a place or a reason.');
  if(!f.at)continue;
  const at=f.at;P.setScenario(at.scenario??'ready');
  if(at.setting)P.settings[at.setting[0]]=at.setting[1];
  if(at.session)P.views.sessionSelected=at.session;
  if(at.task)P.views.taskSelected=at.task;
  if(at.detail){P.views.flockDetail=at.detail;P.views.taskDetail=at.detail;}
  if(at.activity)P.state.activity=at.activity;
  if(at.category)P.state.category=at.category;
  if(at.face)P.views.connectionFace=at.face;
  if(at.memoryTab)P.views.memoryTab=at.memoryTab;
  P.setArea(at.area,at.mode);
  if(at.scenario)P.setScenario(at.scenario);
  if(at.example)P.showExample(at.example);
  if(at.type)type(find('.chat-view textarea'),at.type);
  assert.ok(find(f.expect),'Feature '+f.id+' ('+f.feature+') is visible at '+JSON.stringify(at)+' as '+f.expect);
  if(at.setting)P.settings[at.setting[0]]=at.setting[0]==='dnd'?'Off':false;
  if(at.type)type(find('.chat-view textarea'),'');
  P.views.flockDetail='session';P.views.taskDetail='overview';P.views.connectionFace='you';P.views.memoryTab='Waiting for you';
}
reset();

/* Every sentence is translated: walk every screen in each language. */
for(const lang of ['pt-BR','es','zh']){
  P.setLanguage(lang);
  for(const f of manifest.filter(f=>f.at)){P.setScenario(f.at.scenario??'ready');P.setArea(f.at.area,f.at.mode);if(f.at.example)P.showExample(f.at.example);}
  for(const area of ['flock','chat','tasks','connections','activity','settings','review'])P.setArea(area,'room');
}
P.setLanguage('pt-BR');
const untranslated=P.missing();
if(untranslated.length&&process.argv.includes('--missing')){console.log(JSON.stringify(untranslated,null,1));process.exit(1);}
assert.equal(untranslated.length,0,'Untranslated sentences: '+untranslated.slice(0,12).join(' | ')+(untranslated.length>12?' …':''));

console.log('coverage ok');process.exit(0);
}
assert.equal(saves,0,'Loading the prototype does not publish state.');
assert.equal(find('.portrait').children[0].tagName,'CANVAS','The production sprite renderer is mounted.');

/* Names: Island · Compact · Lab, and nothing else. */
assert.deepEqual(all('.product-size button').map(b=>b.textContent),['Ilha','Compact','Lab']);
assert.equal(text('.surface-name'),'Compact');
P.setArea('flock','room');assert.equal(text('.surface-name'),'Lab');assert.equal(find('.product-window').getAttribute('aria-label'),'Vults Lab');
P.setMode('full');assert.equal(text('.surface-name'),'Lab','Full screen is the Lab maximized.');assert.equal(find('[data-size="room"]').getAttribute('aria-pressed'),'true');P.setMode('room');
for(const area of ['flock','chat','tasks','connections','activity','settings']){P.setArea(area,'room');assert.equal(/\b(Room|Sala|Nest|R\u00e1pido|Quick)\b/.test(find('.product-window').textContent),false,'No retired surface name in Lab '+area);}

/* Navigation: Compact has three tabs, the Lab five; Settings is the gear in both. */
P.setArea('chat','quick');assert.deepEqual(all('.product-nav button').map(b=>b.dataset.key),['nav-flock','nav-chat','nav-summary']);
P.setArea('flock','room');assert.deepEqual(all('.product-nav button').map(b=>b.dataset.key),['nav-flock','nav-chat','nav-tasks','nav-connections','nav-activity']);
find('[data-key="head-settings"]').click();assert.equal(P.state.area,'settings');
P.setArea('chat','quick');find('[data-key="head-settings"]').click();assert.equal(find('.action-menu').hidden,false,'The Compact gear opens quick settings in place.');assert.equal(P.state.mode,'quick');
change(all('.action-menu select')[1],'1 h');assert.ok(find('[data-key="dnd"]'),'Do not disturb shows its moon.');find('[data-key="dnd"]').click();assert.equal(find('[data-key="dnd"]'),null);
P.setArea('chat','quick');const fixed=find('.surface-shell').getAttribute('data-dimensions');
for(const area of ['flock','summary','chat']){P.setArea(area,'quick');assert.equal(find('.surface-shell').getAttribute('data-dimensions'),fixed,'Every Compact tab keeps the same frame.');}
P.setArea('settings','quick');assert.equal(P.state.mode,'room','Settings always opens in the Lab.');
P.setArea('memory','quick');assert.equal(P.state.area,'chat');assert.equal(find('.room-memory').hidden,false,'Memory opens beside the Lab chat.');

/* The card: Compact density. */
P.setArea('flock','quick');P.setScenario('approval');
assert.equal(text('.request-command'),'npm run build');assert.ok(text('.request-count').includes('2'));assert.ok(find('.request-clock'));
find('[data-key="allow"]').click();assert.equal(text('.request-command'),'npm run lint','1 of N moves to the next request.');
const rules=P.settings.rules.length;find('[data-key="always"]').click();assert.equal(P.settings.rules.length,rules+1,'Always adds an exact rule.');assert.equal(P.settings.rules[0].target,'npm run lint');
assert.equal(find('.request-card'),null,'Answered cards leave.');
P.setScenario('destructive');assert.ok(find('.request-card.destructive'));assert.ok(find('[data-key="deny"]').classList.contains('primary'));assert.equal(find('[data-key="always"]'),null,'Destructive calls never offer Always.');
find('[data-key="deny"]').click();assert.equal(P.sessions[0].status,'idle');
P.setScenario('question');assert.equal(all('.question-option').length,4);find('[data-key="option-2"]').click();assert.equal(P.sessions[0].status,'working','A single choice answers at once.');
P.setScenario('question-multi');assert.equal(find('[data-key="send-answer"]').disabled,true);find('[data-key="option-1"]').click();find('[data-key="option-3"]').click();assert.equal(find('[data-key="send-answer"]').disabled,false);find('[data-key="send-answer"]').click();assert.equal(P.sessions[0].status,'working');
P.setScenario('question');find('[data-key="release"]').click();assert.equal(P.sessions[0].card,false,'Reply in the terminal releases the card.');
P.setScenario('expired');P.setArea('flock','room');assert.equal(all('.request-card button').length,0,'An expired card cannot be answered.');
P.setScenario('approval');assert.ok(find('.request-history'),'The Lab card shows recent answers.');assert.ok(find('.request-reason'));

/* Compact flock: bounded, stable, one focus. */
P.setScenario('ready');P.setArea('flock','quick');
assert.ok(all('.compact-bird-pick').length<=6);assert.equal(find('.session-row'),null,'Compact does not repeat the Lab list.');
const wire=find('.compact-wire canvas'),director=scenes.get(wire);
find('[data-key="session-2"]').dispatchEvent({type:'pointerenter'});assert.equal(birdLooks.at(-1).key,'claude:session-2');
find('[data-key="session-2"]').click();assert.ok(text('.compact-focus').includes('frontend'));assert.equal(find('.compact-wire canvas'),wire,'Selection keeps the same director.');
assert.ok(text('.compact-meta').includes('1'),'Focus shows step, subagents and cost.');
find('[data-key="compact-more"]').click();assert.equal(find('.action-menu').hidden,false);assert.ok(find('.action-menu').textContent.includes('Abrir terminal'));
find('[data-key="open-terminal"]').click();assert.equal(find('.action-menu').hidden,true);
P.setScenario('many');assert.ok(all('.compact-bird-pick').length<=6);assert.ok(text('[data-key="compact-all-sessions"]').includes('16'));
P.setScenario('ready');

/* Now: one inbox. */
P.setArea('summary','quick');const before=all('.now-row').length;assert.ok(before>=7,'Now gathers cards, failures, quiet birds, checks, inbox, tasks and memory.');
assert.ok(find('[data-key="now-needs-session-1"]'));assert.ok(find('[data-key="now-failed-session-4"]'));assert.ok(find('[data-key="now-quiet-session-6"]'));
find('[data-key="seen-inbox-1"]').click();assert.equal(all('.now-row').length,before-1,'✓ Seen removes one row.');
find('[data-key="mark-all-seen"]').click();assert.equal(find('[data-key="mark-all-seen"]').disabled,true);
find('[data-key="keep-going"]').click();assert.equal(find('[data-key="now-quiet-session-6"]'),null);
all('.now-open').find(b=>b.textContent.includes('api')).click();assert.equal(P.state.mode,'room');assert.equal(P.views.flockDetail,'activity','A failure opens its output in the Lab.');

/* Lab: one list and detail for sessions and tasks. */
P.setArea('flock','room');assert.ok(all('.lab-bird-target').length>=5,'Every visible Lab bird is a real control.');
find('[data-key="bird-session-2"]').click();assert.equal(P.views.sessionSelected,'session-2');
find('[data-key="tab-diff"]').click();assert.equal(all('.lab-diff-file').length,16);assert.ok(all('.lab-code-row').length>800);
const reader=find('.lab-diff-scroll');reader.scrollTop=680;find('[data-key="tab-context"]').click();assert.ok(text('.lab-context').includes('~/Projects/frontend'));
find('[data-key="tab-diff"]').click();assert.equal(find('.lab-diff-scroll').scrollTop,680,'Reading position survives tab switches.');
change(all('.lab-diff-toolbar select')[0],'4');assert.equal(all('.lab-diff-file').length,2);change(all('.lab-diff-toolbar select')[0],'8');
find('[data-key="session-6"]').click();assert.equal(find('.lab-diff-scroll'),null,'A session without changes never inherits a patch.');
keydown(find('.detail-tabs'),'ArrowRight');assert.equal(P.views.flockDetail,'activity');
find('[data-key="session-1"]').click();find('[data-key="tab-session"]').click();assert.equal(find('[data-key="task-link"]'),null,'A session you opened yourself has no task.');
assert.ok(find('.lab-sky .proto-sky'),'The Lab flock has a sky above the wire.');
find('[data-key="session-1"]').click();find('[data-key="skip"]').click();assert.ok(find('[data-key="unskip"]'),'A skipped card leaves the detail.');find('[data-key="unskip"]').click();assert.ok(find('.request-card'));

/* Skipping never answers; the needs strip folds away when everything is skipped. */
P.setArea('tasks','room');assert.ok(all('.needs-chip').length>=2);
const firstSkip=all('.needs-chip .icon-action')[0];firstSkip.click();const left=all('.needs-chip').length;
find('[data-key="skip-all"]').click();assert.equal(find('.needs-strip').hidden,true,'With everything skipped the strip disappears.');
assert.equal(P.sessions[0].status,'approval','Skipping does not answer the card.');P.views.skipped.splice(0);

/* Tasks: agents Vults runs on its own, read as a timeline. */
P.setArea('tasks','room');assert.ok(find('.task-side'));assert.equal(all('.task-timeline .stage').length,5);
assert.ok(find('.live-output .request-card'),'A task run waiting for you shows its card in the run.');
assert.equal(P.sessions.some(s=>s.id==='run-task-2'),false,'A task run is not on the wire by default.');
find('[data-key="task-more"]').click();find('[data-key="task-wire"]').click();assert.ok(P.sessions.some(s=>s.id==='run-task-2'),'Show it on the wire puts its bird there.');
P.setArea('flock','room');find('[data-key="run-task-2"]').click();assert.ok(find('[data-key="task-link"]'),'A run on the wire links back to its task.');
P.setArea('tasks','room');find('[data-key="task-more"]').click();find('[data-key="task-wire"]').click();assert.equal(P.sessions.some(s=>s.id==='run-task-2'),false);
find('[data-key="fold-tasks"]').click();assert.ok(find('.task-side.folded'),'The task list folds to a rail.');find('[data-key="fold-tasks"]').click();
const handle=find('.task-side .flex-handle');keydown(handle,'ArrowRight');assert.equal(P.views.panels.tasks.w,320,'The list resizes from the keyboard.');
assert.ok(all('.task-line').length>=5,'Each task says where it is in the list.');
find('[data-key="task-5"]').click();find('[data-key="task-start"]').click();find('[data-key="codex"]').click();
const started=P.tasks.find(t=>t.id==='task-5');assert.equal(started.status,'Doing');assert.ok(started.run,'Starting a task gives it a run.');
find('[data-key="task-stop"]').click();assert.equal(started.status,'Blocked');find('[data-key="task-go-on"]').click();assert.equal(started.status,'Doing');
find('[data-key="task-2"]').click();find('[data-key="run-check-2"]').click();runTimers();assert.equal(P.tasks.find(t=>t.id==='task-2').checks[2],'passed','A criterion that names a command runs as a check.');
find('[data-key="task-3"]').click();find('[data-key="task-review"]').click();assert.equal(P.tasks.find(t=>t.id==='task-3').rounds.length,3);
const prs=all('.connection-row').length;find('[data-key="task-accept"]').click();assert.equal(P.tasks.find(t=>t.id==='task-3').status,'Done');
find('[data-key="new-task"]').click();submitEditor();assert.equal(find('.editor-error').hidden,false,'A task needs a goal.');
type(find('.editor-content input'),'Prototype task');type(find('.editor-content textarea'),'First\nSecond');submitEditor();assert.equal(P.tasks[0].title,'Prototype task');assert.equal(P.tasks[0].criteria.length,2);
find('[data-key="task-more"]').click();assert.ok(find('[data-key="delete-task"]')&&find('[data-key="remove-folder"]'));find('[data-key="task-more"]').click();

/* Connections: a face for you and a face for Zeca. */
P.setArea('connections','room');assert.ok(all('.connection-row').length>=3);
find('[data-key="#361"]').click();assert.equal(find('[data-key="merge"]').disabled,true,'Failed checks cannot be merged.');
find('[data-key="#364"]').click();find('[data-key="merge"]').click();assert.equal(all('.confirmation-actions button').at(-1).classList.contains('danger'),false,'A merge is not styled as destructive.');confirmDialog();assert.equal(find('[data-key="merge"]').disabled,true);
find('[data-key="refresh"]').click();const synced=find('.sync-line').textContent;find('[data-key="refresh"]').click();assert.equal(find('.sync-line').textContent,synced,'Refresh runs at most once a minute.');
find('[data-key="tab-zeca"]').click();assert.equal(all('.zeca-tool').length,3);const toggle=find('[data-key="tool-merge"] .toggle');toggle.click();assert.equal(toggle.getAttribute('aria-checked'),'true');

/* Chat as an agent. */
P.setArea('chat','quick');P.showExample('empty');assert.ok(find('.usage-ring'),'The model shows plan usage.');
find('.model-trigger').click();assert.ok(all('.engine-option').length>=5);find('[data-key="engine-codex"]').click();assert.ok(find('.model-options').textContent.includes('GPT-6.1-Sol'));
change(find('.effort-select'),'High');assert.ok(find('.model-trigger').textContent.includes('Alto'),'Effort shows on the trigger.');find('.model-close').click();
const box=find('.chat-view textarea');type(box,'/');assert.equal(find('.command-popover').hidden,false);assert.equal(all('.command-option').length,5);
keydown(box,'Enter');assert.equal(box.value,'/task ');type(box,'/task Add a dark mode');keydown(box,'Enter');runTimers();
assert.ok(P.tasks.some(t=>t.title==='Add a dark mode'),'/task creates a task.');assert.ok(text('.transcript').includes('Add a dark mode'));
type(box,'what is @fr');assert.ok(all('.command-option')[0].textContent.includes('@frontend'));keydown(box,'Tab');assert.ok(box.value.endsWith('@frontend '));
keydown(box,'Enter');runTimers();assert.ok(text('.transcript').includes('frontend'));
type(box,'/remember Ship the island first');keydown(box,'Enter');runTimers();assert.ok(context.__vultsPrototype.state.messages.at(-1).text.includes('Vults'));
P.showExample('thinking');type(box,'one more thing');keydown(box,'Enter');assert.ok(find('.queued'),'A message sent during a reply waits in the queue.');
P.showExample('permission');assert.ok(find('.request-card.density-chat'));assert.ok(find('[data-key="always"]').textContent.includes('Sempre nesta conversa'));find('[data-key="allow"]').click();assert.ok(find('.card-outcome'),'The chat card resolves in place.');
P.showExample('outside');assert.ok(find('.request-reason.danger-text'));assert.equal(find('[data-key="always"]'),null,'Piping a download to the shell is destructive.');
P.showExample('conversation');assert.ok(find('.tool-details'));assert.ok(all('.message-tools button').length>=1);
P.setArea('chat','room');assert.ok(find('.conversation-row .row-tools'));
find('[data-label="Rename"]').click();type(find('.editor-content input'),'Renamed');submitEditor();assert.ok(text('.room-recent').includes('Renamed'));

/* Every Lab side panel is flexible. */
for(const [area,id] of [['flock','flock-list'],['chat','recent'],['connections','conn'],['settings','settings']]){
  P.views.connectionFace='you';P.setArea(area,'room');const fold=find('[data-key="fold-'+id+'"]');assert.ok(fold,'The '+id+' panel can fold.');
  fold.click();assert.ok(find('.flexible.folded'),'Folding '+id+' leaves a rail.');find('[data-key="rail-'+id+'"]').click();assert.equal(find('.flexible.folded'),null);
  const edge=fold.parentElement;keydown(edge,'ArrowRight');assert.ok(P.views.panels[id].w>0,'The '+id+' panel resizes.');
}

/* Zeca as a companion: he greets with the flock, suggests from it, notices, and reacts. */
P.setArea('chat','quick');P.showExample('empty');assert.ok(/Boa|Bom/.test(text('.intro-copy h2')),'Zeca greets by the time of day.');assert.ok(text('.intro-caption').includes('Vults'),'The greeting says what the flock needs.');
assert.ok(find('[data-key="s-card"]'),'Suggestions follow the flock.');find('[data-key="s-card"]').click();assert.ok(find('.chat-view textarea').value.includes('Vults'),'A suggestion fills the draft; it never sends.');type(find('.chat-view textarea'),'');
P.showExample('conversation');assert.ok(find('.zeca-note'),'Zeca notes what needs you inside the conversation.');assert.ok(find('.reply-chips'));
find('[data-key="chip-card"]').click();assert.equal(P.state.area,'flock','A follow-up can open the card; it never answers it.');
P.setArea('chat','quick');type(find('.chat-view textarea'),'hi');assert.equal(sceneUpdates.get(find('.portrait canvas'))[2].clip,'listen','Zeca listens while you type.');
type(find('.chat-view textarea'),'Why did api fail?');keydown(find('.chat-view textarea'),'Enter');assert.ok(['search','think'].includes(sceneUpdates.get(find('.portrait canvas'))[2].clip),'Zeca searches, then thinks.');runTimers();
assert.ok(text('.transcript').includes('cargo test -p api'),'He answers from the flock.');assert.ok(find('[data-key="chip-f-task"]'));

/* Memory. */
P.setArea('memory','room');const pending=all('.room-memory .memory-change').length;assert.equal(pending,3);
assert.ok(find('.memory-change .diff-remove'),'A change shows current versus proposed.');
find('[data-key="proposal-1"] [data-key="remember"]').click();assert.equal(all('.room-memory .memory-change').length,2);

/* Settings: every category, policies through a diff. */
P.setArea('settings','room');assert.equal(all('.settings-categories button').filter(b=>!b.classList.contains('flex-toggle')&&!b.classList.contains('flex-rail')).length,10);
for(const name of ['general','chat','flock','agents','approvals','projects','activity','connectors','phone','about']){find('.settings-categories [data-key="'+name+'"]').click();assert.ok(find('.settings-content').children.length>1,'Settings '+name+' has content.');}
find('.settings-categories [data-key="approvals"]').click();all('.settings-content button').find(b=>b.textContent==='Editar').click();type(find('.editor-content textarea'),'allow command "npm run build"\nallow command "npm test"');submitEditor();runTimers();
assert.ok(find('.editor-content .diff-add'),'A policy change is reviewed as a diff.');submitEditor();assert.ok(P.settings.policies[0].text.includes('npm test'));

/* Activity. */
P.setArea('activity','room');assert.equal(all('.activity-tabs button').length,6);
for(const id of ['week','projects','history','cost','traces','audit']){find('[data-key="tab-'+id+'"]').click();assert.ok(find('.activity-content').children.length>0,'Activity '+id+' has content.');}
assert.ok(text('.activity-content').includes('npm run lint'),'Card answers appear in Audit: '+text('.activity-content').slice(0,400)+' state:'+P.state.activity);

/* Island. */
P.setMode('mini');assert.equal(find('.product-window').hidden,true);assert.ok(find('.mini-strip canvas'));
find('.mini-strip').click();assert.equal(P.state.mode,'quick');assert.equal(P.state.area,'flock','The island opens Compact on whoever needs you.');

const coverage=execFileSync(process.execPath,[fileURLToPath(import.meta.url),'--coverage',...process.argv.filter(a=>a==='--missing')],{encoding:'utf8'});assert.ok(coverage.includes('coverage ok'),coverage);
const manifest=JSON.parse(readFileSync(new URL('features.json',import.meta.url),'utf8')).features;

assert.equal(P.missing().length,0,'Untranslated in the flows: '+P.missing().join(' | '));
/* Continuity and restore. */
P.setArea('chat','room');type(find('.chat-view textarea'),'A draft that survives every layout');P.setArea('chat','quick');assert.equal(find('.chat-view textarea').value,'A draft that survives every layout');
P.setArea('flock','quick');P.setArea('chat','quick');
const held=JSON.parse(JSON.stringify(lastSaved));held.modelContent.area='connections';held.modelContent.mode='quick';
const restored=execFileSync(process.execPath,[fileURLToPath(import.meta.url),'--restore'],{env:{...process.env,PROTOTYPE_STATE:JSON.stringify(held)},encoding:'utf8'});assert.ok(restored.includes('restored'));
assert.ok(JSON.stringify(lastSaved).length<16384,'Saved state stays under the host limit.');
assert.equal(/\b(?:fetch|XMLHttpRequest|WebSocket)\s*\(/.test(source),false,'The prototype never calls a service.');
assert.ok(Buffer.byteLength(source)<1_000_000);
const shipped=manifest.filter(f=>f.status==='shipped').length,placed=manifest.filter(f=>f.at).length;
console.log(`Prototype checks passed: ${manifest.length} features (${shipped} shipped), ${placed} placed on a screen and checked, the rest native or planned with a reason; one card in three densities; Island · Compact · Lab; every sentence translated in pt-BR, es and zh. No backend calls.`);
