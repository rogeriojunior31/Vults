import { readFile, writeFile, readdir } from 'node:fs/promises';
import { findings, coverage } from './findings.mjs';
import { buildStudio } from './build-studio.mjs';
const here=new URL('./',import.meta.url),root=new URL('../../',here);
const bird='data:image/png;base64,'+(await readFile(new URL('docs/assets/zeca.png',root))).toString('base64');
const species=JSON.parse(await readFile(new URL('evidence/bird-species.json',here),'utf8'));
const atlas=JSON.parse(await readFile(new URL('evidence/bird-atlas.json',here),'utf8'));
const studio=await buildStudio();
const theme=await readFile(new URL('ui/src/themes.css',root),'utf8')+'\n'+await readFile(new URL('theme.css',here),'utf8');
const app=(await readFile(new URL('proposal.html',here),'utf8')).replace('__NATIVE_THEME__',theme).replace('__ROADMAP_SCRIPT__',await readFile(new URL('roadmap.js',here),'utf8')).replace('__BIRD_DATA__',bird).replace('__BIRD_SPECIES__',JSON.stringify(species)).replace('__BIRD_ATLAS__',JSON.stringify(atlas)).replace('__STUDIO_APP__',Buffer.from(studio).toString('base64'));
const captures=JSON.parse(await readFile(new URL('evidence/inspection.json',here),'utf8'));
const images={};
for(const name of await readdir(new URL('evidence/',here))) if(name.endsWith('.png')&&!name.startsWith('proposal-'))images[name]='data:image/png;base64,'+(await readFile(new URL('evidence/'+name,here))).toString('base64');
let shell=await readFile(new URL('index-template.html',here),'utf8');
shell=shell.replace('__NATIVE_THEME__',theme).replace('__BIRD_DATA__',bird).replace('__APP_BASE64__',Buffer.from(app).toString('base64')).replace('__REVIEW_DATA__',JSON.stringify({findings,coverage,captures:captures.map(({name,width,height,shot,errors})=>({name,width,height,shot,errors})),images}).replaceAll('<','\\u003c'));
try{const checks=JSON.parse(await readFile(new URL('evidence/proposal-checks.json',here),'utf8'));shell=shell.replace('Validação do protótipo em andamento.',`${checks.cases} combinações · ${checks.interactions} interações verificadas · ${checks.errors.length===0&&checks.failures.length===0?'sem falhas nos checks executados':'consulte proposal-checks.json'}`)}catch{}
await writeFile(new URL('index.html',here),shell);
console.log(`HTML autônomo criado: ${captures.length} capturas, ${findings.length} achados, ${(Buffer.byteLength(shell)/1024/1024).toFixed(1)} MB.`);
