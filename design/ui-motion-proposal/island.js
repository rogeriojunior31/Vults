  /* The Island sees, it never answers: badges by shape, the pulse, plan usage. A click opens Compact. */
  const BADGE={approval:'needs',question:'question',finished:'done',failed:'failed'};
  const PLANS=[{name:'Claude Code',agent:'claude',used:42},{name:'Codex',agent:'codex',used:76}];
  function islandSummary(){
    const f=flockFacts(),parts=[];
    if(f.needs.length)parts.push(phrase(f.needs.length===1?'1 needs you':'{n} need you',{n:f.needs.length}));
    if(f.failed.length)parts.push(phrase(f.failed.length===1?'1 failed':'{n} failed',{n:f.failed.length}));
    if(f.working.length)parts.push(phrase('{n} working',{n:f.working.length}));
    return parts.join(' · ')||L('Nothing running');
  }
  function renderIsland(host){
    host.replaceChildren();const f=flockFacts();
    const visible=sessions.filter(s=>!settings['hide-'+s.project]).slice(0,8),badges=el('div','island-badges');badges.setAttribute('aria-hidden','true');
    mountFlock(host,'mini',visible,scene=>{
      badges.replaceChildren();
      for(const slot of scene.slots()){
        const s=visible.find(v=>v.agent+':'+v.id===slot.key);const kind=s&&(s.quiet?'quiet':BADGE[s.status]);if(!kind)continue;
        const badge=el('span','island-badge shape shape-'+kind);badge.style.left=Math.round(slot.x+slot.width-6)+'px';badges.append(badge);
      }
    });
    const plans=el('div','island-usage');plans.setAttribute('aria-hidden','true');
    if(settings.usage)for(const plan of PLANS){const line=el('span','usage-line '+plan.agent+(plan.used>=90?' critical':plan.used>=70?' warning':''));line.style.width=plan.used+'%';plans.append(line);}
    host.append(badges,plans);
    if(settings.github&&unseenCount())host.append(el('span','island-news'));
    if(settings.media)host.append(el('span','island-media',null,A.icon('note',10)));
    host.classList.toggle('pulse-needs',f.needs.length>0&&settings.dnd==='Off');host.classList.toggle('edge-failed',!f.needs.length&&f.failed.length>0);
    host.setAttribute('aria-label',phrase('Open Compact: {summary}',{summary:islandSummary()}));host.setAttribute('data-tooltip',islandSummary());
  }
  $('.mini-strip').addEventListener('click',()=>{
    const first=flockFacts().needs[0];
    if(first){views.sessionSelected=first.id;setArea('flock','quick');}else setMode('quick');
  });
