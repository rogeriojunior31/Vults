import { chromium } from '@playwright/test';
import { writeFile } from 'node:fs/promises';
const browser=await chromium.launch({headless:true});
const page=await browser.newPage();await page.goto('http://127.0.0.1:1430/lab/nest.html?still=1');
const birds=await page.evaluate(async()=>{
 const {speciesSet}=await import('/src/character/flock/index.ts');
 const {drawFrame,frameAt}=await import('/src/character/sprites.ts');
 const result={};
 for(const [name,id,clip] of [['claude','atratus','idle'],['approval','atratus','approval'],['codex','atratus','idle'],['qwen','aura','idle'],['idle','atratus','idle']]) {
  const set=speciesSet(id),frame=frameAt(set.clips[clip],1500);
  const points=frame.layers.flatMap(([part,x,y])=>set.parts[part].flatMap((row,j)=>[...row].flatMap((value,i)=>value==='.'?[]:[[x+i+frame.dx,y+j+frame.dy]])));
  const minX=Math.min(...points.map(p=>p[0])),minY=Math.min(...points.map(p=>p[1]));
  const maxX=Math.max(...points.map(p=>p[0])),maxY=Math.max(...points.map(p=>p[1]));
  const canvas=document.createElement('canvas');canvas.width=(maxX-minX+3)*3;canvas.height=(maxY-minY+3)*3;
  drawFrame(canvas.getContext('2d'),set,frame,-minX+1,-minY+1,3);result[name]=canvas.toDataURL();
 }
 return result;
});
await writeFile(new URL('./evidence/bird-species.json',import.meta.url),JSON.stringify(birds));
const atlas=await page.evaluate(async()=>{
 const {speciesSet}=await import('/src/character/flock/index.ts');
 const {drawFrame}=await import('/src/character/sprites.ts');
 const result={};
 for(const id of ['atratus','aura']) {
  const set=speciesSet(id),names=['idle','think','read','search','edit','run','approval','question','done','fail','listen'];
  const clips=Object.fromEntries(names.map(name=>[name,set.clips[name]]));
  const points=Object.values(clips).flatMap(c=>c.frames.flatMap(f=>f.layers.flatMap(([part,x,y])=>set.parts[part].flatMap((row,j)=>[...row].flatMap((value,i)=>value==='.'?[]:[[x+i+f.dx,y+j+f.dy]])))));
  const minX=Math.min(...points.map(p=>p[0]))-1,minY=Math.min(...points.map(p=>p[1]))-1;
  const width=(Math.max(...points.map(p=>p[0]))-minX+2)*3,height=(Math.max(...points.map(p=>p[1]))-minY+2)*3;
  const frames=Object.values(clips).flatMap(c=>c.frames);
  const columns=16,canvas=document.createElement('canvas');canvas.width=columns*width;canvas.height=Math.ceil(frames.length/columns)*height;
  frames.forEach((f,i)=>drawFrame(canvas.getContext('2d'),set,f,(i%columns)*width/3-minX,Math.floor(i/columns)*height/3-minY,3));
  let offset=0;const timeline={};
  for(const [name,clip] of Object.entries(clips)){timeline[name]={loop:clip.loop,frames:clip.frames.map((f,i)=>({index:offset+i,ms:f.ms}))};offset+=clip.frames.length}
  result[id]={image:canvas.toDataURL(),width,height,columns,clips:timeline};
 }
 return result;
});
await writeFile(new URL('./evidence/bird-atlas.json',import.meta.url),JSON.stringify(atlas));
console.log('Espécies originais + 11 clips por espécie exportados para reprodução offline.');await browser.close();
