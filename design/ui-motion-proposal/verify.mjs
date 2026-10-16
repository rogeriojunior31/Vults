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
  get firstElementChild(){return this.children[0];}get lastElementChild(){return this.children.at(-1);}
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
  getContext(){return {clearRect(){},fillRect(){},drawImage(){}};}
  animate(frames,options){animations.push({element:this,frames,options});return {finished:Promise.resolve(),cancel(){}};}
}
function inflate(tree){const node=new Element(tree.tag);for(const[k,v]of Object.entries(tree.attrs))node.setAttribute(k,v??'');for(const child of tree.children){if(typeof child==='string')node.text=(node.text??'')+child;else node.append(inflate(child));}return node;}
const dom=inflate(tree),root=dom.querySelector('[id="vults-ui-proposal"]');
document={body:new Element('body'),documentElement:new Element('html'),hidden:false,createElement:tag=>new Element(tag),createElementNS:(_ns,tag)=>new Element(tag),getElementById:id=>nodes(dom).find(n=>n.id===id),addEventListener:(type,fn)=>documentListeners.set(type,fn),activeElement:null};
const timers=new Map();let timerId=0,saves=0,lastSaved;
const windowListeners=new Map();
const context={document,console,performance,Event:class{constructor(type){this.type=type;}},matchMedia:()=>({matches:false,addEventListener(){}}),devicePixelRatio:1,
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
if(process.argv.includes('--compact-restore')){
  assert.equal(saves,0);assert.equal(find('[data-key="nav-summary"]').getAttribute('aria-pressed'),'true');
  assert.ok(find('.compact-summary'));assert.equal(find('.product-window').classList.contains('tight'),false);
  console.log('Legacy Compact navigation restored into Overview.');process.exit(0);
}
if(process.argv.includes('--restore')){
  assert.equal(saves,0,'Restoration never publishes a new state.');
  assert.equal(find('.chat-view textarea').value,'A draft that survives every layout');
  assert.equal(find('.review-row textarea').value,'Keep the approved composer');
  console.log('Saved prototype state restored without losing draft or review notes.');process.exit(0);
}
assert.equal(saves,0,'Loading a proposal must not publish state.');
assert.equal(find('.portrait').children[0].tagName,'CANVAS','The production sprite renderer is mounted.');
assert.equal(all('.message').length,0);
assert.equal(find('.surface-name').textContent,'Compact');
assert.equal(find('.product-size').hidden,false);
find('[data-example="models"]').click();assert.equal(find('.model-popover').hidden,false);
assert.equal(all('.model-option').length,4);
const engine=find('.engine-select');engine.value='Codex';engine.dispatchEvent({type:'change'});
assert.ok(find('.model-options').textContent.includes('GPT-6.1-Sol'));
all('.model-option')[1].click();await Promise.resolve();assert.equal(find('.model-popover').hidden,true);assert.ok(find('.model-trigger').textContent.includes('GPT-6.1-Sol'));
all('.composer-left button')[0].click();assert.equal(all('.attachment').length,1);
const input=find('textarea');input.value='A sample question';input.dispatchEvent({type:'input'});
let prevented=false;input.dispatchEvent({type:'keydown',key:'Enter',isComposing:true,preventDefault(){prevented=true;}});assert.equal(prevented,false);assert.equal(all('.message.you').length,0);
find('.send-action').click();assert.equal(all('.message.you').length,1);assert.equal(find('.send-action').disabled,false,'Stop remains reachable during the simulated turn.');
find('.send-action').click();
find('[data-example="notification"]').click();assert.equal(find('.notice-slot').hidden,false);find('.notice-close').click();await Promise.resolve();assert.equal(find('.notice-slot').hidden,true);
find('[data-area="tasks"]').click();assert.equal(all('.task-row').length,3);
const search=find('.workspace-view .search-field input');search.value='nonexistent';search.dispatchEvent({type:'input'});assert.equal(all('.task-row').length,0);
search.value='';search.dispatchEvent({type:'input'});
const criteria=find('.criteria-body input');criteria.checked=false;criteria.dispatchEvent({type:'change'});assert.ok(find('.criteria summary').textContent.includes('1/3'));
find('[data-area="connections"]').click();assert.equal(all('.connection-row').length,3);
all('.detail-actions button')[1].click();assert.equal(find('.dialog-overlay').hidden,false);all('.confirmation-actions button')[0].click();assert.equal(find('.dialog-overlay').hidden,true);
all('.detail-actions button')[0].click();assert.equal(find('.workspace-view').hidden,true);assert.ok(find('textarea').value.length>0);
find('[data-area="settings"]').click();assert.ok(all('.setting-row').length>3);
assert.equal(find('.product-window').getAttribute('aria-label'),'Vults Room');
assert.equal(find('.product-size').hidden,false);
assert.equal(find('.return-compact').hidden,false);
const toggle=find('.toggle');assert.equal(toggle.getAttribute('aria-checked'),'false');toggle.click();assert.equal(toggle.getAttribute('aria-checked'),'true');
all('.settings-categories button')[1].click();assert.ok(find('.settings-content').textContent.includes('Zeca'));
find('.return-compact').click();assert.equal(find('.workspace-view').hidden,true);
assert.equal(find('.product-window').getAttribute('aria-label'),'Vults Compact');
assert.ok(find('textarea').value.length>0,'Returning from Room keeps the Compact draft.');
all('.product-nav button').at(-1).click();assert.equal(find('.product-window').getAttribute('aria-label'),'Vults Room');
const route=(area,mode)=>all('[data-area]').find(n=>n.dataset.area===area&&(!mode||n.dataset.mode===mode)).click();
const change=(node,value)=>{node.value=value;node.dispatchEvent({type:'change'});};
const type=(node,value)=>{node.value=value;node.dispatchEvent({type:'input'});};
const submitEditor=()=>all('.editor-actions button').at(-1).click();
const confirmExample=()=>all('.confirmation-actions button').at(-1).click();
route('tasks');
find('[data-key="new-task"]').click();assert.equal(find('.editor-overlay').hidden,false);
submitEditor();assert.equal(find('.editor-error').hidden,false,'An empty task goal cannot be saved.');
type(find('.editor-content input'),'Prototype task');
type(find('.editor-content textarea'),'First criterion\nSecond criterion');
submitEditor();assert.equal(find('.editor-overlay').hidden,true);assert.equal(all('.task-row').length,4);assert.ok(all('.task-row')[0].textContent.includes('Prototype task'));
route('connections');
find('[data-key="#361"]').click();assert.equal(all('.detail-actions button')[1].disabled,true,'Failed checks cannot be merged.');
find('[data-key="#364"]').click();all('.detail-actions button')[1].click();confirmExample();assert.equal(all('.detail-actions button')[1].disabled,true,'A simulated merge cannot run twice.');
route('flock','room');assert.equal(all('.session-row').length,5);assert.equal(find('.flock-stage canvas').tagName,'CANVAS');
assert.equal(all('.lab-bird-target').length,5,'Every visible Lab bird has a real selection control.');
const labBird=find('[data-key="bird-session-2"]');labBird.focus();labBird.click();
assert.equal(find('.lab-head select').value,'session-2');assert.ok(find('.session-detail').textContent.includes('frontend'));
assert.equal(document.activeElement,find('[data-key="bird-session-2"]'),'Selecting a bird retains keyboard focus.');
assert.equal(find('[data-key="bird-session-2"]').getAttribute('aria-pressed'),'true');
find('[data-key="bird-session-1"]').click();
const labFrame=find('.surface-shell').getAttribute('data-dimensions');find('[data-key="lab-diff"]').click();
assert.equal(all('.lab-bottom-tabs button').length,5);assert.equal(all('.lab-diff-file').length,16,'The full reader displays a multi-file patch.');
assert.ok(all('.lab-code-row').length>800,'The demonstration exercises a long patch instead of a two-line placeholder.');
assert.equal(find('.session-grid'),null,'Diff reading uses the content area without a competing session card or list.');
assert.equal(find('.flock-stage'),null,'The scene gives its space to the selected detail tab.');
assert.equal(find('[data-key="lab-pending"]').hidden,false,'A waiting permission remains reachable while reading changes.');
const readPosition=find('.lab-diff-scroll');readPosition.scrollTop=680;readPosition.scrollLeft=140;
find('[data-key="lab-context"]').click();assert.ok(find('.lab-context').textContent.includes('/projects/Vults'));
find('[data-key="lab-diff"]').click();assert.equal(find('.lab-diff-scroll').scrollTop,680);assert.equal(find('.lab-diff-scroll').scrollLeft,140);
change(all('.lab-diff-toolbar select')[1],'ui/src/island/chat.ts');assert.equal(all('.lab-diff-file').length,1);
find('.lab-diff-scroll').scrollTop=85;change(all('.lab-diff-toolbar select')[1],'all');assert.equal(all('.lab-diff-file').length,16);assert.equal(find('.lab-diff-scroll').scrollTop,680);
change(all('.lab-diff-toolbar select')[0],'4');assert.equal(all('.lab-diff-file').length,2,'The selected step determines the actual files displayed.');
change(all('.lab-diff-toolbar select')[0],'8');assert.equal(all('.lab-diff-file').length,16);
const bottom=find('.lab-bottom-tabs');bottom.dispatchEvent({type:'keydown',key:'ArrowRight',preventDefault(){}});assert.ok(find('.lab-step'));
find('[data-key="lab-diff"]').click();assert.equal(find('.surface-shell').getAttribute('data-dimensions'),labFrame,'Detail tabs never resize the Lab.');
find('[data-key="lab-pending"]').click();assert.ok(find('.permission-preview'));assert.equal(find('.lab-head select').value,'session-1');
find('[data-key="allow"]').click();assert.equal(find('.permission-preview'),null,'A decision updates only the simulated request.');
change(find('.scenario-select'),'destructive');assert.ok(find('.permission-preview pre').textContent.includes('git clean -fd'));assert.ok(find('[data-key="deny"]').classList.contains('primary'));assert.equal(all('.permission-preview button').length,2,'Destructive calls never offer a persistent allow rule.');
change(find('.scenario-select'),'question');assert.equal(find('[data-key="send-answer"]').disabled,true);change(find('.permission-preview select'),'Review first');find('[data-key="send-answer"]').click();assert.equal(find('.permission-preview'),null);
change(find('.scenario-select'),'expired');assert.equal(all('.permission-preview button').length,0,'An expired request cannot be answered.');
change(find('.scenario-select'),'many');assert.equal(all('.session-row').length,15);change(find('.scenario-select'),'ready');
find('[data-lab-diff]').click();assert.equal(find('.lab-head select').value,'session-2');assert.equal(all('.lab-diff-file').length,16);
change(find('.lab-head select'),'session-3');assert.equal(find('.lab-diff-scroll'),null,"Sessions without captured changes do not inherit another session's patch.");
change(find('.lab-head select'),'session-2');assert.equal(all('.lab-diff-file').length,16);
route('activity');assert.equal(all('.activity-tabs button').length,6);
for(const tab of all('.activity-tabs button')){const name=tab.dataset.key;find('[data-key="'+name+'"]').click();assert.ok(find('.activity-content').children.length>0,'Activity '+name+' has content.');}
find('[data-key="audit"]').click();type(find('.workspace-view .search-field input'),'npm run build');find('.record-table button').click();assert.ok(find('.editor-content').textContent.includes('npm run build'),'A simulated decision appears in Audit.');find('.editor-close').click();
route('settings');assert.equal(all('.settings-categories button').length,10);
for(const name of ['general','chat','flock','agents','approvals','projects','activity','connectors','phone','about']){
  find('.settings-categories [data-key="'+name+'"]').click();assert.ok(find('.settings-content').children.length>1,'Settings '+name+' has usable content.');
}
find('.settings-categories [data-key="chat"]').click();
const memoryOpen=all('.settings-content button').find(n=>n.textContent==='Abrir Mem\u00f3ria');assert.ok(memoryOpen);memoryOpen.click();
assert.equal(all('.memory-change').length,2);
all('.content-row .inline-actions button')[1].click();assert.equal(all('.memory-change').length,1,'Remember removes one pending proposal.');
all('.workspace-view .segment button')[1].click();assert.equal(all('.workspace-view .content-row').length,2,'Accepted proposal joins remembered facts.');
all('.workspace-view .segment button')[2].click();all('.workspace-view .content-row button')[0].click();
all('.workspace-view .segment button')[0].click();assert.equal(all('.memory-change').length,2,'Undo returns the proposal to review.');
route('chat','room');type(find('.chat-view textarea'),'A draft that survives every layout');
assert.deepEqual(all('.product-nav button').slice(0,3).map(b=>b.dataset.key),['nav-flock','nav-chat','nav-tasks'],'Room puts Chat second, before Tasks.');
assert.equal(find('.room-recent').hidden,false);
change(find('.scenario-select'),'empty');assert.equal(find('.room-suggestions').hidden,false);
find('[data-size="quick"]').click();assert.equal(find('.room-suggestions').hidden,true,'Compact never shows suggested prompts.');assert.equal(find('.chat-view textarea').value,'A draft that survives every layout');
find('[data-size="mini"]').click();assert.equal(find('.product-window').hidden,true);assert.equal(find('.mini-strip').hidden,false);assert.ok(find('.mini-strip canvas'));
find('.mini-strip').click();assert.equal(find('.product-window').hidden,false);assert.equal(find('.product-window').getAttribute('aria-label'),'Vults Compact');
find('[data-key="nav-chat"]').click();assert.equal(find('.chat-view textarea').value,'A draft that survives every layout');
change(find('.scenario-select'),'ready');
const fixedCompact=find('.surface-shell').getAttribute('data-dimensions');
for(const key of ['nav-flock','nav-summary','nav-chat']){find('[data-key="'+key+'"]').click();assert.equal(find('.surface-shell').getAttribute('data-dimensions'),fixedCompact,'Every Compact tab has the same outer dimensions.');}
assert.deepEqual(all('.product-nav button').map(b=>b.dataset.key),['nav-flock','nav-chat','nav-summary','nav-settings']);
find('[data-key="nav-flock"]').click();
assert.equal(all('.session-row').length,0,'Compact does not repeat the full Room session list.');
assert.ok(all('.compact-bird-pick').length<=6);assert.equal(find('.permission-preview'),null);
assert.equal(find('[data-key="signal-person"]').textContent,'1 pedido');
assert.equal(find('[data-key="signal-retry"]').textContent,'1 falha');
const flockCanvas=find('.compact-wire canvas'),flockScene=scenes.get(flockCanvas),firstBird=flockScene.vults.get('codex:session-1').bird;
const editBird=find('[data-key="session-2"]');editBird.dispatchEvent({type:'pointerenter'});
assert.equal(birdLooks.at(-1).key,'claude:session-2','Looking targets the chosen agent and session.');
editBird.dispatchEvent({type:'pointerleave'});assert.equal(birdLooks.at(-1).key,null);
editBird.dispatchEvent({type:'focus'});assert.equal(birdLooks.at(-1).key,'claude:session-2','Keyboard focus gives the same character response.');
editBird.click();assert.ok(find('.compact-focus').textContent.includes('frontend'));assert.equal(find('.compact-wire canvas'),flockCanvas,'Selection keeps the same animation director.');
find('[data-key="nav-chat"]').click();find('[data-key="nav-flock"]').click();
assert.equal(find('.compact-wire canvas'),flockCanvas,'Returning from another tab reuses the living scene.');
for(const [phase,status,activity,summary] of [['read','working','read','Lendo'],['edit','working','edit','Editando'],['run','working','run','checks'],['think','thinking','think','Pensando'],['done','finished',null,'terminou'],['fail','failed','run','falhou'],['rest','idle',null,'Pronto']]){
  change(find('.scenario-select'),phase);const shown=sceneUpdates.get(flockCanvas)[0],vults=shown.find(s=>s.id==='session-1');
  assert.equal(vults.status,status);assert.equal(vults.activity,activity);assert.ok(find('.compact-focus').textContent.includes(summary),'The visible explanation matches the real renderer state.');
  assert.equal(scenes.get(find('.compact-wire canvas')),flockScene,'Changing state preserves the character director.');
  assert.equal(flockScene.vults.get('codex:session-1').bird,firstBird,'A new state updates the same bird without resetting its animation.');
  assert.deepEqual([...shown].map(v=>v.species).sort(),['atratus','aura','burrovianus','papa','vultur'].sort(),'Each session retains its species.');
}
change(find('.scenario-select'),'ready');

const beforeRequest=lastSaved.privateContent.draft;
find('[data-key="compact-session-action"]').click();
assert.equal(find('.editor-overlay').hidden,false);assert.equal(find('.permission-preview pre').textContent,'npm run build');
assert.ok(find('.permission-preview .mono').textContent.includes('/projects/Vults'));
assert.equal(find('[data-key="signal-person"]').textContent,'1 pedido','Opening a review never grants a permission.');
find('.editor-close').click();
find('[data-key="compact-ask"]').click();assert.equal(find('.product-window').getAttribute('aria-label'),'Vults Compact');
assert.ok(find('.chat-view textarea').value.startsWith(beforeRequest));
assert.ok(find('.chat-view textarea').value.includes('Vults'));type(find('.chat-view textarea'),'A draft that survives every layout');
find('[data-key="nav-flock"]').click();change(find('.scenario-select'),'many');
assert.ok(all('.compact-bird-pick').length<=6);assert.ok(find('[data-key="compact-all-sessions"]').textContent.includes('15'));
windowListeners.get('resize')();assert.ok(all('.compact-bird-pick').length<=6);
find('[data-key="compact-all-sessions"]').click();assert.equal(all('.session-row').length,15);
change(find('.scenario-select'),'ready');find('[data-size="quick"]').click();
change(find('.scenario-select'),'destructive');find('[data-key="compact-session-action"]').click();
assert.equal(find('.permission-preview pre').textContent,'git clean -fd');assert.ok(find('[data-key="deny"]').classList.contains('primary'));
find('[data-key="deny"]').click();assert.equal(find('.editor-overlay').hidden,true);assert.equal(find('[data-key="signal-person"]').disabled,true);
change(find('.scenario-select'),'ready');find('[data-key="session-4"]').click();find('[data-key="compact-session-action"]').click();
assert.equal(find('[data-key="session-4"]').getAttribute('aria-pressed'),'true','Opening Room retains the focused session.');
find('[data-size="quick"]').click();find('[data-key="nav-summary"]').click();
assert.ok(find('.compact-summary'));assert.equal(find('.workspace-view').classList.contains('compact-flock'),false);
assert.equal(all('.compact-feed .compact-insight').length,3,'Overview limits attention rows to actionable categories.');
assert.ok(find('[data-key="insight-checks"]').textContent.includes('#361'),'Failing checks have priority over healthy PRs.');
find('[data-key="insight-checks"]').click();assert.equal(find('[data-key="#361"]').getAttribute('aria-pressed'),'true');
assert.equal(all('.detail-actions button')[1].disabled,true);find('[data-size="quick"]').click();
assert.equal(find('[data-key="nav-summary"]').getAttribute('aria-pressed'),'true');
find('[data-key="insight-task"]').click();assert.equal(all('.task-row').length,1);
find('[data-size="quick"]').click();find('[data-key="insight-memory"]').click();assert.equal(all('.memory-change').length,2);
find('[data-size="quick"]').click();find('[data-key="nav-chat"]').click();
const legacyRestore=execFileSync(process.execPath,[fileURLToPath(import.meta.url),'--compact-restore'],{env:{...process.env,PROTOTYPE_STATE:JSON.stringify({modelContent:{area:'connections',mode:'quick'},privateContent:{views:{quickArea:'connections'}}})},encoding:'utf8'});assert.ok(legacyRestore.includes('restored'));
find('[data-example="empty"]').click();const shapeCount=()=>animations.filter(a=>a.element===find('.surface-shell')).length,beforePopup=shapeCount();
find('.model-trigger').click();assert.equal(find('.model-popover').parentElement,find('.popover-layer'),'Selectors live outside the scrolling transcript and composer.');
assert.equal(find('.surface-shell').getAttribute('data-dimensions'),fixedCompact);assert.equal(shapeCount(),beforePopup,'Opening a popup does not start a surface resize.');
find('.model-close').click();find('.model-trigger').click();await Promise.resolve();assert.equal(find('.model-popover').hidden,false,'Reopening before exit completion keeps the menu visible.');
documentListeners.get('pointerdown')({target:document.body});await Promise.resolve();assert.equal(find('.model-popover').hidden,true,'Clicking outside the whole preview dismisses the menu.');
find('[data-example="conversation"]').click();find('.transcript').scrollTop=80;find('.model-trigger').click();assert.equal(find('.transcript').scrollTop,80,'Opening a selector preserves the reading position.');find('.model-close').click();find('[data-key="nav-flock"]').click();find('[data-key="nav-chat"]').click();assert.equal(find('.transcript').scrollTop,80,'Returning from another Compact tab preserves reading position.');
find('[data-size="mini"]').click();find('.mini-strip').click();assert.equal(find('[data-key="nav-chat"]').getAttribute('aria-pressed'),'true','The island returns to the last Compact tab.');
assert.ok(shapeCount()>beforePopup,'Mode switches animate the persistent surface shell.');
for(const animation of animations.filter(a=>a.element===find('.surface-shell')))assert.ok(animation.frames.every(frame=>!('transform'in frame)),'Surface motion never stretches the content or sprites.');
change(find('.viewport-select'),'short');const shortSize=find('.surface-shell').getAttribute('data-dimensions');assert.ok(Number(shortSize.split('x')[1])<=360);find('.model-trigger').click();assert.equal(find('.surface-shell').getAttribute('data-dimensions'),shortSize);find('.model-close').click();change(find('.viewport-select'),'normal');
route('activity');change(find('.scenario-select'),'error');assert.ok(find('.empty-state').textContent.includes('rascunho'));find('.empty-state button').click();assert.ok(find('.activity-content'));
change(find('.scenario-select'),'loading');assert.equal(all('.loading-line').length,2);change(find('.scenario-select'),'ready');
route('review');assert.equal(all('.review-row').length,8);assert.equal(all('.review-row .status').filter(n=>n.textContent==='Dire\u00e7\u00e3o aprovada').length,2);
const note=find('.review-row textarea');type(note,'Keep the approved composer');assert.equal(lastSaved.privateContent.notes.chat,'Keep the approved composer');
all('.workspace-view .segment button')[1].click();assert.equal(all('.release-step').length,4);
assert.ok(JSON.stringify(lastSaved).length<16384,'Saved review state stays under the host limit.');
const held=lastSaved, beforeRestore=saves;windowListeners.get('openai:set_globals')({detail:{globals:{widgetState:held}}});assert.equal(saves,beforeRestore,'Host state restoration cannot loop through setWidgetState.');
const restored=execFileSync(process.execPath,[fileURLToPath(import.meta.url),'--restore'],{env:{...process.env,PROTOTYPE_STATE:JSON.stringify(held)},encoding:'utf8'});assert.ok(restored.includes('restored'));
route('settings');find('.settings-categories [data-key="general"]').click();change(find('.settings-content select'),'en');assert.ok(find('.settings-content').textContent.includes('Language & sound'));change(find('.settings-content select'),'zh');assert.ok(find('.settings-content').textContent.includes('\u8bed\u8a00\u4e0e\u58f0\u97f3'));change(find('.settings-content select'),'pt-BR');
find('.settings-categories [data-key="flock"]').click();const zeca=all('.settings-content .toggle')[0];zeca.click();assert.equal(find('[data-key="nav-chat"]').disabled,true,'Zeca off preserves independent workspace navigation.');zeca.click();
all('.settings-content .toggle').find(n=>n.dataset.label==='Animations').click();const reducedCount=shapeCount();route('chat','room');find('[data-size="quick"]').click();assert.equal(shapeCount(),reducedCount,'Reduced motion changes modes without geometry animation.');find('[data-key="nav-flock"]').click();const stillCanvas=find('.compact-wire canvas');
assert.equal(scenes.get(stillCanvas).layout.flights,false,'A paused scene never freezes midway through a new arrival.');
const lookCount=birdLooks.length;find('[data-key="session-2"]').dispatchEvent({type:'pointerenter'});assert.equal(birdLooks.length,lookCount,'Reduced motion disables character hover reactions.');
find('[data-key="nav-chat"]').click();find('[data-example="notification"]').click();assert.equal(find('.notice-slot').inert,false,'Reduced motion still leaves the notification dismiss action usable.');find('.notice-close').click();assert.equal(find('.notice-slot').hidden,true);
route('settings');find('.settings-categories [data-key="flock"]').click();all('.settings-content .toggle').find(n=>n.dataset.label==='Animations').click();
find('.settings-categories [data-key="activity"]').click();all('.settings-content .setting-row button').at(-1).click();confirmExample();
route('chat','room');assert.equal(all('.message').length,0);assert.equal(find('.chat-view textarea').value,'');assert.equal(all('.room-recent button').length,3,'Clearing example history removes old conversations as well.');
route('activity');assert.ok(find('.empty-state'),'Cleared example records cannot reappear from static fixtures.');
assert.equal(/\b(?:fetch|XMLHttpRequest|WebSocket)\s*\(/.test(source),false,'Prototype behavior never calls external services.');
assert.ok(saves>0);assert.ok(Buffer.byteLength(source)<1_000_000);
console.log('Prototype checks passed: bounded surface/popup geometry, clickable Lab birds, bottom detail tabs, multi-file patch reading and retained scroll, living flock continuity, pointer/keyboard character responses, state clips, stable species, simplified Compact overview and flock, bounded bird counts, scoped request review, stable Compact tabs, shared-shell motion without content scaling, second-position Room Chat, popup reopen/exit races, manual scroll and drafts, reduced motion, notifications, tasks, merge guards, decisions, Activity, Settings, Memory, review notes and restoration. No backend calls; native visual acceptance remains pending.');
