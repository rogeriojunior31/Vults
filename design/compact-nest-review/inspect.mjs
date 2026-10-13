import { chromium } from '@playwright/test';
import { mkdir, writeFile, readFile } from 'node:fs/promises';
if(process.argv.includes('--deep')) { await import('./deep.mjs'); process.exit(0); }
if(process.argv.includes('--proposal')) { await import('./verify.mjs'); process.exit(0); }
if(process.argv.includes('--birds')) { await import('./birds.mjs'); process.exit(0); }
const out = new URL('./evidence/', import.meta.url);
await mkdir(out, { recursive: true });
const browser = await chromium.launch({headless: true});
const compactOnly=process.argv.includes('--compact-only');
const settingsOnly=process.argv.includes('--settings-only');
const results = compactOnly||settingsOnly?JSON.parse(await readFile(new URL('inspection.json',out),'utf8')).filter(r=>settingsOnly?!r.name.startsWith('settings-'):!r.name.startsWith('compact-')&&!r.name.startsWith('pill-')):[];
const mock = {
  api_key_status: false, app_settings: {sounds:true,autostart:false,foldAfter:15,volume:50,zeca:true,lang:'en',zecaSpecies:'atratus',settingsPath:'~/.config/vults/settings.json',dataPath:'~/.local/share/vults/'},
  rules_list:[],policies_list:[],projects_list:{},connectors_status:[],shortcut_keys:{},current_view:{sessions:[],approval:null,alerts:[]},nest_is_full:false,monitors:[],voice_status:{models:[],selected:null,language:null,system:'en',downloading:[],ready:false},agents_doctor:[],
};
mock.phone_settings={on:false,transport:'relay',relay:null,listen:null,devices:[]};
mock.projects_list={'~/Projects/Vults':{mute:false,pin:true,hide:false},'~/Projects/site':{mute:true,pin:false,hide:false}};
mock.web_settings={on:false,engine:'brave',searxngUrl:null,hasKey:false};
mock.api_providers={selected:'anthropic',providers:[]};
mock.activity={history:true,today:'2026-10-12',weeks:['2026-10-05'],grid:[],week:{monday:'2026-10-05',sunday:'2026-10-11',active_secs:22800,turns:41,steps:512,commands:87,files:34,added:1217,removed:312,allowed:14,denied:2,answered:5,questions:6,failed:1,top_agent:'claude',top_project:'Vults',busiest_day:'2026-10-07',longest_secs:11100,days:[3000,5400,7800,2400,4200,0,1200],projects:[],headline:'41 turns, 6 h 20 min with your agents, most on Vults.'}};
mock.github_calendar={state:'off'};
async function capture(name, url, width, height, action, settings=false) {
 const page = await browser.newPage({viewport:{width,height},reducedMotion:'reduce'});
 const errors=[]; page.on('pageerror',e=>errors.push(e.message));
 if(settings) await page.addInitScript(mock=>{window.__TAURI_INTERNALS__={invoke:async(cmd,args)=>{
  if(cmd==='install_status')return {agent:args.agent,configPath:`~/.${args.agent}/settings.json`,hookPath:'~/.local/share/vults/bin/vults-hook',hookReady:true,installed:true,error:null,codex:args.agent==='codex'?{hooksDisabled:false,untrusted:0,total:10,error:null}:null};
  if(cmd==='plugin:app|version')return '0.1.7'; return mock[cmd]??null;
 },transformCallback:()=>0,metadata:{currentWindow:{label:'nest-window'},currentWebview:{label:'settings'}}};},mock);
 await page.goto('http://127.0.0.1:1430'+url); await page.waitForTimeout(350);
 if(action) await action(page);
 await page.waitForTimeout(100);
 const root=settings?'body':url.includes('nest.html')?'#nest':'#island';
 const data=await page.evaluate(root=>{
  const base=document.querySelector(root), bounds=base.getBoundingClientRect();
  const all=[...base.querySelectorAll('*')].filter(e=>{const r=e.getBoundingClientRect();return r.width&&r.height&&getComputedStyle(e).visibility!=='hidden'});
  const label=e=>(e.getAttribute('aria-label')||e.innerText||e.title||'').trim().slice(0,95);
  const path=e=>e.tagName.toLowerCase()+'.'+[...e.classList].slice(0,3).join('.');
  const outside=all.filter(e=>{const r=e.getBoundingClientRect();return r.right>bounds.right+2||r.left<bounds.left-2}).map(e=>({element:path(e),label:label(e),x:Math.round(e.getBoundingClientRect().x),width:Math.round(e.getBoundingClientRect().width)}));
  const controls=all.filter(e=>e.matches('button,input,select,textarea,[role=button],[role=tab],a'));
  const unnamed=controls.filter(e=>!label(e)&&!e.getAttribute('placeholder')&&!(e.id&&document.querySelector(`label[for="${e.id}"]`))).map(path);
  const small=controls.filter(e=>{const r=e.getBoundingClientRect();return r.width<24||r.height<24}).map(e=>({element:path(e),label:label(e),w:Math.round(e.getBoundingClientRect().width),h:Math.round(e.getBoundingClientRect().height)}));
  return {bounds:{width:Math.round(bounds.width),height:Math.round(bounds.height)},outside:outside.slice(0,25),unnamed,small:small.slice(0,25),text:base.innerText.slice(0,16000),tabs:controls.filter(e=>e.matches('[role=tab]')).map(e=>({label:label(e),selected:e.getAttribute('aria-selected'),controls:e.getAttribute('aria-controls')}))};
 },root);
 const shot=name+'-'+width+'.png';
 if(settings) await page.screenshot({path:new URL(shot,out).pathname});
 else await page.locator(root).screenshot({path:new URL(shot,out).pathname});
 const cdp=await page.context().newCDPSession(page);
 const ax=await cdp.send('Accessibility.getFullAXTree');
 const namelessAX=ax.nodes.filter(n=>!n.ignored&&['button','checkbox','combobox','textbox','slider','tab'].includes(n.role?.value)&&!n.name?.value).map(n=>({role:n.role.value,node:n.backendDOMNodeId}));
 results.push({name,url,width,height,shot,errors,namelessAX,...data});
 await writeFile(new URL('inspection.json',out),JSON.stringify(results,null,2));
 console.log(name,width,'errors',errors.length,'outside',data.outside.length,'unnamed',data.unnamed.length);
 await page.close();
}
if(!compactOnly&&!settingsOnly) for(const [width,height] of [[1180,820],[760,640],[480,360]]) {
 for(const tab of ['flock','tasks','chat','week','projects','history','cost','traces','audit','connections']) await capture('nest-'+tab,`/lab/nest.html?still=1&tab=${tab}`,width,height);
 await capture('nest-memory','/lab/nest.html?still=1&tab=chat&memory=1',width,height);
 await capture('nest-approval','/lab/nest.html?still=1',width,height,p=>p.locator('.roost-card').first().click());
 for(const group of ['Pull requests','Actions','Releases']) await capture('nest-connections-'+group.toLowerCase().replaceAll(' ','-'),'/lab/nest.html?still=1&tab=connections',width,height,p=>p.locator('.nest-filter').filter({hasText:group}).click());
}
if(!compactOnly) for(const section of ['general','agents','chat','approvals','projects','activity','connectors','phone','flock','about']) for(const [width,height] of [[1180,820],[480,360]]) await capture('settings-'+section,`/settings.html?still=1#${section}`,width,height,null,true);
if(settingsOnly) { await browser.close(); process.exit(0); }
for(const [name,state,tab] of [['flock','busy-flock'],['activity','busy-flock','Activity'],['settings','busy-flock','Settings'],['connections','github-card','Connections'],['chat','chat'],['approval','approval'],['destructive','destructive-approval'],['question','question-card'],['failed','failed'],['overflow','big-flock'],['chat-permission','chat-permission']]) {
 await capture('compact-'+name,`/lab/?still=1&t=1500&state=${state}&open=1`,1000,1000,tab?p=>p.locator(`#island .tabs button[title="${tab}"]`).click():null);
}
for(const state of ['idle-flock','busy-flock','big-flock','failed']) await capture('pill-'+state,`/lab/?still=1&t=1500&state=${state}`,1000,600);
for(const tab of ['flock','tasks','chat','projects','connections']) await capture('empty-'+tab,`/lab/nest.html?still=1&empty=1&tab=${tab}`,760,640);
await capture('nest-without-zeca','/lab/nest.html?still=1&zeca=0',760,640);
await capture('nest-stale','/lab/nest.html?still=1&tab=connections&stale=1',760,640);
await capture('nest-fresh','/lab/nest.html?still=1&tab=chat&fresh=1',760,640);
await writeFile(new URL('inspection.json',out),JSON.stringify(results,null,2));
await browser.close();
