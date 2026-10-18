// Production Settings diagnostics with a local backend: no microphone, desktop or network.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";
const all = node => [node, ...node.children.flatMap(all)];
class Element {
  children = []; attributes = {}; listeners = {}; dataset = {}; className = ""; disabled = false;
  constructor(tag) { this.tag = tag; }
  set textContent(text) { this.text = text; this.children = []; }
  get textContent() { return (this.text ?? "") + this.children.map(n => n.textContent).join(""); }
  append(...nodes) { for (const node of nodes) { node.parentElement = this; this.children.push(node); } }
  replaceChildren(...nodes) { this.children = []; this.text = ""; this.append(...nodes); }
  addEventListener(name, fn) { this.listeners[name] = fn; }
  setAttribute(name, value) { this.attributes[name] = value; }
  contains(node) { return all(this).includes(node); }
  focus() { document.activeElement = this; }
  click() { if (!this.disabled) this.listeners.click?.(); }
}
globalThis.document = { createElement: tag => new Element(tag), documentElement: {} };
const scheduled = new Map(); let timer = 0;
globalThis.setTimeout = callback => { scheduled.set(++timer, callback); return timer; };
globalThis.clearTimeout = id => scheduled.delete(id);
const ui = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({ configFile: false, root: ui, logLevel: "silent", plugins: [{ name:"diagnostics-language", transform(code,id) {
  if (id.endsWith("/dictation-doctor.ts")) return code+'\nexport { setLang } from "../../i18n";';
} }], build:{write:false,minify:false,lib:{entry:`${ui}src/surfaces/settings/dictation-doctor.ts`,formats:["es"]}} });
const { createDictationDoctor, setLang } = await import(`data:text/javascript;base64,${Buffer.from(bundles[0].output.find(p=>p.type==="chunk").code).toString("base64")}`);
const reports = [], starts = [], polls = [], stops = [];
const pending = list => new Promise((resolve,reject) => list.push({resolve,reject}));
const view = createDictationDoctor({ report:() => pending(reports), start:() => pending(starts),
  status:id => pending(polls).then(snapshot => ({ ...snapshot, id })), stop:async id => { stops.push(id); } });
const nodes = () => all(view.element), text = () => view.element.textContent;
const named = name => nodes().find(n=>n.tag==="button" && n.textContent===name);
const tick = () => new Promise(resolve=>setImmediate(resolve));
const nextPoll = async () => { const [id,fn] = scheduled.entries().next().value; scheduled.delete(id); fn(); await tick(); };
const empty = {count:0,p50_ms:null,p95_ms:null};
const report = { system:{host:"ALSA",device:"<script>inert mic</script>",rate:48000,session:"Wayland",shortcuts:"ready",keyboard:"off",kwin:"unverified",gpu:false,vad:true},
  counters:{loops:2,retries:1,copied:3},timings:{total:{count:2,p50_ms:500,p95_ms:700},stt:empty,pipeline:empty,cleanup:empty,insert:empty} };
assert.equal(reports.length,0); assert.equal(starts.length,0);
view.visible(true); assert.equal(reports.length,1); assert.equal(starts.length,0,"Reading diagnostics never opens a mic");
reports[0].reject(Error("private backend error")); await tick();
assert.ok(text().includes("Could not read diagnostics")); assert.ok(!text().includes("private backend error"));
named("Refresh diagnostics").click(); reports[1].resolve(report); await tick();
assert.ok(text().includes("48000 Hz") && text().includes("500") && text().includes("Unknown"));
assert.ok(text().includes("<script>inert mic</script>")); assert.ok(!nodes().some(n=>n.tag==="script"));
assert.equal(nodes().filter(n=>n.tag==="table").length,1);
named("Refresh diagnostics").click(); reports[2].reject(Error("failed")); await tick();
assert.ok(text().includes("last readings may be out of date") && text().includes("48000 Hz"));
named("Test microphone").click(); assert.equal(starts.length,1); assert.equal(named("Test microphone").disabled,true);
starts[0].resolve(1); await tick();
const stop = named("Stop test"); stop.focus();
polls[0].resolve({phase:"running", elapsed_ms:2500,bands:Array(9).fill(.5)}); await tick();
assert.equal(named("Stop test"),stop,"Live meter updates retain the focused stop action");
assert.equal(document.activeElement,stop); assert.ok(text().includes("Speak now"));
assert.ok(nodes().filter(n=>n.tag==="progress").every(n=>n.value===.5));
await nextPoll(); polls[1].resolve({phase:"done",bands:[],noise_db:-60,verdict:"ok"}); await tick();
assert.ok(text().includes("-60 dBFS") && text().includes("Input level looks usable"));
assert.equal(named("Test microphone").disabled,false);
// Hide while opening: its late success must be stopped, never adopted by the new page.
named("Test microphone").click(); view.visible(false); starts[1].resolve(2); await tick();
assert.ok(stops.includes(2)); assert.equal(polls.length,2);
view.visible(true); reports[3].resolve(report); await tick();
named("Test microphone").click(); starts[2].reject(Error("input missing")); await tick();
assert.ok(text().includes("Could not test the microphone"));
named("Test microphone").click(); starts[3].resolve(3); await tick();
view.visible(false); polls[2].resolve({phase:"done",bands:[],noise_db:-9,verdict:"noisy"}); await tick();
assert.ok(stops.includes(3) && !text().includes("-9 dBFS"),"Late test results cannot redraw a departed page");
view.visible(true); reports[4].resolve({...report,timings:null}); await tick();
assert.ok(text().includes("Could not read timing history"));
for (const lang of ["pt-BR","es","zh"]) { setLang(lang); view.relabel(); assert.ok(!text().includes("Test microphone")); }
view.visible(false);
console.log("check-dictation-diagnostics: read-only probes, error/stale/retry, unknown timings, inert names, explicit mic, bands/focus, cancellation races and languages");
