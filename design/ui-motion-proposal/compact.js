  function bindBirdFocus(control,flock,session){
    const look=()=>{if(prefs.motion)flock.hover(session.agent+':'+session.id);};
    const leave=()=>flock.hover(null);
    control.addEventListener('pointerenter',look);control.addEventListener('pointerleave',leave);
    control.addEventListener('focus',look);control.addEventListener('blur',leave);
  }
  function resizeCompactFlock(){
    if(state.mode!=='quick'||state.area!=='flock')return;
    const host=$('.workspace-view'),focus=document.activeElement?.getAttribute('data-key');
    stopScenes();host.replaceChildren();renderCompactFlock(host);
    if(focus)host.querySelector('[data-key="'+focus+'"]')?.focus({preventScroll:true});
  }
  function quickQuestion(s){
    setArea('chat','quick');const question=phrase('What is {project} doing now?',{project:s.project});
    input.value=input.value.trim()?input.value+'\n'+question:question;input.dispatchEvent(new Event('input'));input.focus({preventScroll:true});
  }
  /* Compact · Flock: one session at a time. A waiting card is answered right here. */
  function renderCompactFlock(host) {
    host.classList.add('compact-flock');
    if(stageState(host,'No sessions right now.'))return;
    const capacity=Math.max(1,Math.min(6,Math.floor((windowEl.getBoundingClientRect().width-32)/64)));
    const facts=flockFacts(),shown=facts.ordered.filter(s=>!settings['hide-'+s.project]),visible=shown.slice(0,capacity);
    let selected=sessions.find(s=>s.id===views.sessionSelected)??shown[0];
    if(!selected){host.append(el('p','pane-note','No sessions right now.'));return;}
    if(!visible.some(s=>s.id===selected.id)){visible[visible.length-1]=selected;}
    // Attention chooses who is visible; the wire keeps each bird's relative place.
    visible.sort((a,b)=>sessions.indexOf(a)-sessions.indexOf(b));
    const pending=['approval','question'].includes(selected.status);
    const stage=el('div','flock-stage compact-wire');
    const picks=el('div','compact-bird-picks'),focus=el('section','compact-focus'),queue=el('div','compact-attention'),footer=el('div','compact-footer',null,queue);
    const signals=el('div','compact-signals');signals.setAttribute('aria-label',L('Session overview'));
    function signal(rows,one,many,shape,tone){
      const b=key(button(null,'compact-signal tone-'+tone,()=>select(rows[0])),'signal-'+shape);b.disabled=!rows.length;
      b.append(el('span','shape shape-'+shape),el('span','counter',phrase(rows.length===1?one:many,{n:rows.length})));signals.append(b);
    }
    signal(facts.needs,'1 request','{n} requests','needs','needs');signal(facts.failed,'1 failure','{n} failures','failed','failed');signal(facts.working,'1 working','{n} working','working','working');
    if(facts.quiet.length)signal(facts.quiet,'1 quiet','{n} quiet','quiet','quiet');
    signals.append(key(button('Open in Lab','compact-open',()=>setArea('flock','room')),'compact-open-lab'));
    host.append(signals,stage,focus,footer);const flock=mountFlock(stage,'quick',visible);stage.append(picks);
    picks.style.gridTemplateColumns='repeat('+visible.length+',minmax(0,1fr))';
    visible.forEach(s=>{const b=key(button(null,'compact-bird-pick',()=>select(s)),s.id);b.append(el('span',null,s.project));b.setAttribute('aria-label',phrase('Select {project}: {status}',{project:s.project,status:L(sessionBrief(s))}));b.setAttribute('data-tooltip',L(sessionBrief(s)));bindBirdFocus(b,flock,s);picks.append(b);});
    if(shown.length>visible.length)footer.append(key(button(phrase('See all {n} in Lab',{n:shown.length}),'compact-all',()=>setArea('flock','room')),'compact-all-sessions'));
    function select(s){if(!s)return;views.sessionSelected=s.id;if(!visible.some(v=>v.id===s.id)||['approval','question'].includes(s.status)!==pending){resizeCompactFlock();remember();return;}selected=s;paint(true);remember();}
    function paint(animate=false){
      for(const b of picks.children)b.setAttribute('aria-pressed',String(b.dataset.key===selected.id));
      focus.setAttribute('aria-live','polite');
      const s=selected,more=key(iconButton('list','Quick actions',()=>sessionActions(more,s)),'compact-more');
      focus.replaceChildren(el('div','compact-focus-heading',null,el('strong',null,s.project),el('span',null,labelFor(s.agent)),sessionChip(s),more));
      const meta=[phrase('Step {n}',{n:s.step_count}),...(s.subagents?[phrase(s.subagents===1?'1 subagent':'{n} subagents',{n:s.subagents})]:[]),...(s.cost?[raw('≈ $'+s.cost.toFixed(2))]:[])];
      if(['approval','question'].includes(s.status)){focus.append(requestCard(s,'compact',()=>resizeCompactFlock()));}
      else{
        focus.append(el('p','compact-meta counter',null,document.createTextNode(meta.map(L).join(' · '))),el('p',null,sessionBrief(s)));
        if(s.quiet)focus.append(quietActions(s));
        else{
          const label=s.status==='failed'?'Review failure':s.status==='finished'?'View result':'Follow in Lab';
          const action=key(button(label,'primary',()=>{views.sessionSelected=s.id;views.flockDetail=s.status==='finished'?'diff':s.status==='failed'?'activity':'session';setArea('flock','room');}),'compact-session-action');
          const ask=key(button('Ask Zeca','quiet-control',()=>quickQuestion(s)),'compact-ask');ask.disabled=!settings.zeca;focus.append(el('div','inline-actions',null,action,ask));
        }
      }
      queue.replaceChildren();
      const next=facts.ordered.find(o=>o.id!==s.id&&(['approval','question','failed'].includes(o.status)));
      if(next){const b=key(button(null,'compact-next',()=>select(next)),'compact-next');b.append(el('span','shape shape-'+(next.status==='failed'?'failed':'needs')),el('span',null,phrase('Also needs you: {project}',{project:next.project})),A.icon('chevron',12));queue.append(b);}
      if(animate)enter(focus,'snappy',0,1);
    }
    paint();
  }
  /* Compact · Now: one inbox for everything that asks for an action. Each row opens the right Lab view. */
  let digestSeen=false;
  function unseenCount(){return [...github.inbox,...github.pulls,...github.actions].filter(r=>r.isNew&&!settings.githubSeen[r.id]).length;}
  function nowRows(){
    const rows=[],f=flockFacts();
    f.needs.forEach(s=>rows.push({id:'needs-'+s.id,shape:s.status==='question'?'question':'needs',urgent:true,title:phrase(s.status==='question'?'{project} asks a question':'{project} wants to run {command}',{project:s.project,command:s.step}),body:phrase('{agent} · goes back to the terminal in {time}',{agent:labelFor(s.agent),time:'1:12'}),open:()=>{views.sessionSelected=s.id;setArea('flock','quick');}}));
    f.failed.forEach(s=>rows.push({id:'failed-'+s.id,shape:'failed',urgent:true,title:phrase('{project}: a check failed',{project:s.project}),body:raw(s.step),open:()=>{views.sessionSelected=s.id;views.flockDetail='activity';setArea('flock','room');}}));
    f.quiet.forEach(s=>rows.push({id:'quiet-'+s.id,shape:'quiet',title:phrase('{project} has been quiet for {n} min',{project:s.project,n:s.quiet}),body:'It may be waiting for input in its terminal.',inline:quietActions(s),open:()=>{views.sessionSelected=s.id;setArea('flock','quick');}}));
    if(settings.github){
      github.pulls.filter(p=>p.checks==='failed'&&!p.merged).forEach(p=>rows.push({id:'pr-'+p.id,shape:'failed',urgent:true,title:phrase('{repo} {ref}: checks failed',{repo:p.repo,ref:p.id}),body:p.title,open:()=>openConnection('pulls',p.id)}));
      github.actions.filter(a=>a.checks==='failed').forEach(a=>rows.push({id:'act-'+a.id,shape:'failed',title:phrase('{repo} main is red',{repo:a.repo}),body:phrase('{check} failed {age} ago',{check:a.checkList.find(c=>c[1]==='failed')[0],age:a.age}),open:()=>openConnection('actions',a.id)}));
      github.inbox.filter(i=>!settings.githubSeen[i.id]).forEach(i=>rows.push({id:'inbox-'+i.id,shape:'news',title:phrase('{kind}: {title}',{kind:L(i.kind),title:L(i.title)}),body:phrase('{repo} {ref} · {author}',{repo:i.repo,ref:i.ref,author:i.author}),seen:i.id,open:()=>openConnection('inbox',i.id)}));
    }
    tasks.filter(t=>t.review==='waiting').forEach(t=>rows.push({id:'task-'+t.id,shape:'needs',title:phrase('Task ready for your review: {task}',{task:L(t.title)}),body:phrase('{n} review round · {agent}',{n:t.rounds.length,agent:t.engine}),open:()=>openTask(t.id,'review')}));
    if(settings.memory&&memory.pending.length)rows.push({id:'memory',shape:'news',title:phrase(memory.pending.length===1?'A memory proposal is waiting':'{n} memory proposals are waiting',{n:memory.pending.length}),body:'You decide what Zeca keeps.',open:()=>openMemory()});
    if(settings.github){const ready=github.pulls.filter(p=>p.checks==='passed'&&!p.merged);if(ready.length)rows.push({id:'ready',shape:'done',quiet:true,title:phrase(ready.length===1?'1 pull request passed its checks':'{n} pull requests passed their checks',{n:ready.length}),body:'Review before merging',open:()=>openConnection('pulls',ready[0].id)});}
    return rows;
  }
  function renderNow(host) {
    host.classList.add('compact-summary');
    const markAll=key(button('Mark all seen','',()=>{for(const r of [...github.inbox,...github.pulls,...github.actions])settings.githubSeen[r.id]=true;setArea('summary','quick');toast('All marked as seen');}),'mark-all-seen');markAll.disabled=!unseenCount();
    host.append(pageHeading('Now',null,markAll));
    if(stageState(host,'Nothing needs your attention'))return;
    const f=flockFacts();
    if(!digestSeen&&(f.failed.length||sessions.some(s=>s.status==='finished')))host.append(key(el('div','away-digest',null,A.icon('moon',13),el('span',null,phrase('While you were away: {done} finished, {failed} failed.',{done:sessions.filter(s=>s.status==='finished').length,failed:f.failed.length})),iconButton('close','Dismiss',()=>{digestSeen=true;setArea('summary','quick');})),'away-digest'));
    const feed=el('div','now-feed');host.append(feed);
    const rows=nowRows();
    rows.forEach(row=>{
      const item=key(el('article','now-row'+(row.quiet?' quiet':'')),'now-'+row.id);
      const open=button(null,'now-open',row.open);open.append(el('span','shape shape-'+row.shape),el('span','now-copy',null,el('strong',null,row.title),el('small',null,row.body)),A.icon('chevron',13));
      item.append(open);
      if(row.inline)item.append(row.inline);
      if(row.seen)item.append(key(button('✓ Seen','now-seen',()=>{settings.githubSeen[row.seen]=true;setArea('summary','quick');remember();}),'seen-'+row.seen));
      feed.append(item);
    });
    if(!rows.length)feed.append(el('div','compact-clear',null,A.icon('check',20),el('strong',null,'Nothing needs your attention'),el('p',null,'Open the Lab when you want the full picture.')));
    const last=auditRows[0];
    if(settings.history&&last)host.append(el('div','compact-recent',null,el('span',null,'Latest decision'),el('span',null,null,document.createTextNode(last.project+' · '+L(last.answer)+' · '+last.command)),key(button('Audit','',()=>{state.activity='audit';setArea('activity','room');}),'compact-open-audit')));
  }
  /* Quick settings: the gear in Compact. Everything else lives in the Lab's Settings. */
  function openQuickSettings(trigger){
    const presence=choose('Presence',['Island','Panel','Quiet','Paused'],settings.presence,value=>changedSetting('presence',value));
    const dnd=choose('Do not disturb',['Off','30 min','1 h','4 h'],settings.dnd,value=>{changedSetting('dnd',value);});
    const fold=choose('Fold the island',[5,10,15,30,60].map(n=>({value:String(n),label:n+' s'})),String(settings.fold),value=>changedSetting('fold',Number(value)));
    openActionMenu(trigger,'Quick settings',[
      {label:'Presence',control:presence},{label:'Do not disturb',control:dnd},{label:'Fold the island',control:fold},
      {label:'Zeca',control:makeSwitch('zeca','Zeca')},{label:'Sounds',control:makeSwitch('sounds','Sounds')},{label:'Keep a history',control:makeSwitch('history','Keep a history')},
      {label:'All settings',icon:'gear',key:'all-settings',action:()=>openSettings('general')},
    ]);
  }
