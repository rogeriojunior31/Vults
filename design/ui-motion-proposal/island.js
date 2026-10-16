  /* The Island sees, it never answers: badges by shape and plan usage, nothing blinks. A click opens Compact.
     A bird with nothing to do stays green on the wire for 30 s, then takes off and circles in the sky
     (the production Sky and its thermal). Work brings it back down. */
  const BADGE={approval:'needs',question:'question',finished:'done',failed:'failed'};
  const PLANS=[{name:'Claude Code',agent:'claude',used:42},{name:'Codex',agent:'codex',used:76}];
  const REST_MS=30_000;
  const resting=s=>['finished','idle'].includes(s.status)&&!s.quiet;
  // The prototype always shows daytime: the thermal only rises by day.
  A.setDayPart('day');
  function markRest(now=Date.now()){for(const s of sessions){if(resting(s))s.restSince??=now;else s.restSince=null;}}
  const soaring=s=>prefs.motion&&resting(s)&&s.restSince!==null&&Date.now()-s.restSince>=REST_MS;
  const skies=new Map();
  function skyFor(kind){
    let sky=skies.get(kind);
    if(!sky){sky=new A.Sky(()=>{for(const scene of activeScenes)scene.refresh();});sky.canvas.classList.add('proto-sky');skies.set(kind,sky);}
    return sky;
  }
  // The scene leaves a bird out while its sky owns it; both share the session's key.
  const skyOwns=kind=>key=>skies.get(kind)?.owns(key)??false;
  function attachSky(kind,host,scene,list,onPlace){
    const sky=skyFor(kind);host.append(sky.canvas);
    const place=()=>{
      const box=host.getBoundingClientRect(),width=Math.min(720,Math.round(host.clientWidth||box.width)),left=Math.max(0,Math.round(((host.clientWidth||box.width)-width)/2));
      sky.canvas.style.left=left+'px';
      const stage=scene.canvas.getBoundingClientRect(),anchors=new Map();
      for(const at of scene.anchors())anchors.set(at.key,{x:stage.left-box.left-left+at.x,y:stage.top-box.top+at.y,scale:at.scale});
      sky.place(anchors,{left:0,top:0,width,height:Math.round(host.clientHeight||box.height),radius:kind==='mini'?20:0});
      sky.update(list.filter(soaring).map(s=>({...s,status:'idle'})),flockVisible());
      onPlace?.();
    };
    place();skyPlacers.set(kind,place);
  }
  const skyPlacers=new Map();
  // Once a second: who has rested long enough to fly, and their badges.
  window.setInterval?.(()=>{
    markRest();
    if(state.mode==='mini'){skyPlacers.get('mini')?.();paintBadges?.();}
    else if(state.area==='flock'&&['room','full'].includes(state.mode))skyPlacers.get('lab')?.();
  },1000);
  markRest();
  function islandSummary(){
    const f=flockFacts(),parts=[];
    if(f.needs.length)parts.push(phrase(f.needs.length===1?'1 needs you':'{n} need you',{n:f.needs.length}));
    if(f.failed.length)parts.push(phrase(f.failed.length===1?'1 failed':'{n} failed',{n:f.failed.length}));
    if(f.working.length)parts.push(phrase('{n} working',{n:f.working.length}));
    const up=sessions.filter(soaring).length;if(up)parts.push(phrase('{n} in the sky',{n:up}));
    return parts.join(' · ')||L('Nothing running');
  }
  let paintBadges=null;
  function renderIsland(host){
    host.replaceChildren();const f=flockFacts();
    const visible=sessions.filter(s=>!settings['hide-'+s.project]).slice(0,8),badges=el('div','island-badges');badges.setAttribute('aria-hidden','true');
    let scene;
    paintBadges=()=>{
      badges.replaceChildren();if(!scene)return;
      for(const slot of scene.slots()){
        const s=visible.find(v=>v.agent+':'+v.id===slot.key);if(!s||soaring(s))continue;
        const kind=s.quiet?'quiet':resting(s)?'done':BADGE[s.status];if(!kind)continue;
        const badge=el('span','island-badge shape shape-'+kind);badge.style.left=Math.round(slot.x+slot.width-6)+'px';badges.append(badge);
      }
      host.setAttribute('aria-label',phrase('Open Compact: {summary}',{summary:islandSummary()}));host.setAttribute('data-tooltip',islandSummary());
    };
    scene=mountFlock(host,'mini',visible,()=>paintBadges());
    const plans=el('div','island-usage');plans.setAttribute('aria-hidden','true');
    if(settings.usage)for(const plan of PLANS){const line=el('span','usage-line '+plan.agent+(plan.used>=90?' critical':plan.used>=70?' warning':''));line.style.width=plan.used+'%';plans.append(line);}
    host.append(badges,plans);
    if(settings.github&&unseenCount())host.append(el('span','island-news'));
    if(settings.media)host.append(el('span','island-media',null,A.icon('note',10)));
    attachSky('mini',host,scene,visible);paintBadges();paintDictationIsland(host);
    host.classList.toggle('edge-needs',f.needs.length>0&&settings.dnd==='Off');host.classList.toggle('edge-failed',!f.needs.length&&f.failed.length>0);
  }
  $('.mini-strip').addEventListener('click',()=>{
    const first=flockFacts().needs[0];
    if(first){views.sessionSelected=first.id;setArea('flock','quick');}else setMode('quick');
  });
