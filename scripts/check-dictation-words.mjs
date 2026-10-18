// Exercise the production Your words owner, without a browser server or native writes.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";
import { build } from "vite";
class Element {
  children = []; listeners = {}; attributes = {}; dataset = {}; scrollTop = 0; className = ""; value = ""; style = {}; hidden = false; isConnected = true;
  constructor(tag) { this.tag = tag; }
  set textContent(value) { this.text = value; this.children = []; }
  get textContent() { return (this.text ?? "") + this.children.map(n => n.textContent).join(""); }
  append(...nodes) { for (const n of nodes) n.parentElement = this; this.children.push(...nodes); }
  replaceChildren(...nodes) { this.text = ""; this.children = []; this.append(...nodes); }
  addEventListener(name, fn) { this.listeners[name] = fn; }
  setAttribute(name, value) { this.attributes[name] = value; }
  getAttribute(name) { return this.attributes[name] ?? null; }
  querySelector(selector) { return all(this).slice(1).find(n => selector.startsWith("[data-history-action=") ? n.dataset.historyAction === selector.slice(22, -2) : selector.startsWith(".") ? n.className === selector.slice(1) : n.tag === selector) ?? null; }
  contains(node) { return all(this).includes(node); }
  closest() { return null; }
  focus() { document.activeElement = this; }
  setCustomValidity(value) { this.validationMessage = value; }
  reportValidity() { return !this.validationMessage; }
  click() { if (!this.disabled) this.listeners.click?.(); }
}
const all = n => [n, ...n.children.flatMap(all)];
globalThis.document = { createElement: tag => new Element(tag), createElementNS: (_, tag) => new Element(tag), documentElement: {}, activeElement: null };
globalThis.window = { clearTimeout() {}, setTimeout() { return 0; } };
const root = fileURLToPath(new URL('../ui/', import.meta.url));
const bundles = await build({ configFile: false, root, logLevel: 'silent', plugins: [{ name: 'words-language', transform(code,id) { if (id.endsWith('/dictation/words.ts')) return `${code}\nexport { setLang } from '../../../i18n';`; } }], build: { write: false, minify: false, lib: { entry: `${root}src/surfaces/nest/dictation/words.ts`, formats: ['es'] } } });
const { createYourWords, setLang } = await import(`data:text/javascript;base64,${Buffer.from(bundles[0].output.find(p=>p.type==='chunk').code).toString('base64')}`);
const tick = () => new Promise(resolve=>setImmediate(resolve));
let data={words:['Vults'], replacements:[], snippets:[]}, reads=0, writes=[], failure=false, failRead=false, pending, resolveTry;
const view=createYourWords({
 words: async()=>{reads++; if(failRead)throw Error('Read failed'); return structuredClone(data);},
 change: async edit=>{writes.push(edit); if(pending)await pending; if(failure)throw Error('Write failed');
  if(edit.action==='addWord') data.words.push(edit.word);
  if(edit.action==='deleteWord')data.words=data.words.filter(w=>w!==edit.word);
  if(edit.action==='setReplacement')data.replacements=[{id:1,from:edit.from,to:edit.to}];
  if(edit.action==='addSnippet')data.snippets.push({id:1,trigger:edit.trigger,text:edit.text});
 },
 trySentence: async()=>new Promise(resolve=>{resolveTry=resolve;}),
},()=>{});
const nodes=()=>[...all(view.dictionary),...all(view.snippets)];
const named=(text,page=view.dictionary)=>all(page).find(n=>n.tag==='button'&&n.textContent===text);
const field=(text,page=view.dictionary)=>all(page).find(n=>n.tag==='label'&&n.children[0]?.textContent===text).children[1];
const type=(input,value)=>{input.value=value;input.listeners.input?.();};
assert.equal(reads,0,'Hidden construction never reads user data');
view.refresh();await tick();assert.equal(reads,1);
const word=field('New word');type(word,'Zeca');failure=true;named('Add word').click();await tick();
assert.equal(word.value,'Zeca','Failed save retains the draft');assert.ok(view.dictionary.textContent.includes('Could not save'));
failure=false;let release;pending=new Promise(r=>release=r);named('Add word').click();await tick();const count=writes.length;named('Add word').click();assert.equal(writes.length,count,'Pending action cannot duplicate writes');
release();pending=null;await tick();assert.equal(word.value,'');assert.ok(view.dictionary.textContent.includes('Zeca'));
const from=field('When I say'),to=field('Write');type(from,'cloud code');type(to,'Claude Code');named('Save replacement').click();await tick();assert.deepEqual(writes.at(-1),{action:'setReplacement',from:'cloud code',to:'Claude Code'});
const body=field('Paste',view.snippets),trigger=field('When I say',view.snippets);type(trigger,'signature');type(body,'  <script>inert</script>\n$literal  ');named('Add snippet',view.snippets).click();await tick();assert.equal(writes.at(-1).text,'  <script>inert</script>\n$literal  ');
assert.ok(view.snippets.textContent.includes('<script>inert</script>'));const text=all(view.snippets).find(n=>n.tag==='p'&&n.textContent.includes('<script>'));assert.equal(text.children.length,0,'User markup is inert');
const remove=all(view.dictionary).find(n=>n.attributes['aria-label']==='Delete Vults');const before=writes.length;remove.click();assert.equal(writes.length,before);named('Cancel').click();assert.equal(writes.length,before);all(view.dictionary).find(n=>n.attributes['aria-label']==='Delete Vults').click();named('Delete').click();await tick();assert.ok(!data.words.includes('Vults'));
const sentence=field('Try a sentence');type(sentence,'old');named('Test sentence').click();await tick();type(sentence,'new');resolveTry({text:'stale',cancelled:false,segments:[{text:'stale',kind:null,rule:null}]});await tick();assert.ok(!view.dictionary.textContent.includes('stale'),'Typing invalidates an old preview');
named('Test sentence').click();await tick();resolveTry({text:'literal',cancelled:false,segments:[{text:'<b>literal</b>',kind:'replacement',rule:'source'}]});await tick();assert.ok(view.dictionary.textContent.includes('<b>literal</b>'));assert.equal(all(view.dictionary).find(n=>n.tag==='mark').children.length,0);
type(from,'kept draft');view.dictionary.scrollTop=120;view.relabel();assert.equal(view.dictionary.scrollTop,120);assert.equal(from.value,'kept draft');
failRead=true;named('Refresh').click();await tick();assert.ok(view.dictionary.textContent.includes('Zeca'));assert.ok(view.dictionary.textContent.includes('Could not load'));failRead=false;named('Refresh').click();await tick();
for(const language of ['pt-BR','es','zh']){setLang(language);view.relabel();assert.ok(!view.dictionary.textContent.includes('Names, spelling and replacements'));assert.equal(from.value,'kept draft');}
console.log('check-dictation-words: CRUD, explicit deletion, literal bodies, pending/failure/retry, retained drafts/scroll, stale previews, inert text and three translations');
