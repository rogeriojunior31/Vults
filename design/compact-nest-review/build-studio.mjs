import { build } from 'vite';
import { readFile, writeFile, mkdtemp, rm } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';

// Compile the existing Studio flows with a presentation adapter; never edit its sources.
export async function buildStudio(){
 const here=dirname(fileURLToPath(import.meta.url)),root=resolve(here,'../..'),source=resolve(here,'../agent-studio');
 const draft=await mkdtemp(resolve(here,'.studio-build-')),out=await mkdtemp(resolve(tmpdir(),'vults-review-studio-'));
 try{
  let js=await readFile(resolve(source,'studio.js'),'utf8');
  js=js.replaceAll("'./imports.js'","'../../agent-studio/imports.js'").replaceAll("'./catalog.json'","'../../agent-studio/catalog.json'").replaceAll("'../../ui/","'../../../ui/");
  await writeFile(resolve(draft,'adapt.js'),js+'\n'+await readFile(resolve(here,'studio-bridge.js'),'utf8'));
  let html=await readFile(resolve(source,'entry.html'),'utf8');
  html=html.replace('<link rel="stylesheet" href="./studio.css">','<style>'+(await readFile(resolve(source,'studio.css'),'utf8')).replaceAll('../../ui/','../../../ui/')+'\n'+await readFile(resolve(here,'studio-adapt.css'),'utf8')+'\n'+await readFile(resolve(root,'ui/src/themes.css'),'utf8')+'\n'+await readFile(resolve(here,'theme.css'),'utf8')+'</style>').replace('src="./studio.js"','src="./adapt.js"');
  await writeFile(resolve(draft,'entry.html'),html);
  await build({root,configFile:false,base:'./',logLevel:'warn',build:{outDir:out,emptyOutDir:true,rolldownOptions:{input:resolve(draft,'entry.html')}}});
  const path=resolve(out,'design/compact-nest-review',draft.split('/').at(-1),'entry.html');
  html=await readFile(path,'utf8');const script=html.match(/<script[^>]*src="([^"]+)"[^>]*><\/script>/);
  const bundle=await readFile(resolve(dirname(path),script[1]),'utf8');
  return html.replace(script[0],'<script type="module">'+bundle.replaceAll('</script','<\\/script')+'</script>')+'\n<!-- Reference license\n'+await readFile(resolve(source,'REFERENCE-LICENSE'),'utf8')+'\n-->';
 }finally{await rm(draft,{recursive:true,force:true});await rm(out,{recursive:true,force:true})}
}
