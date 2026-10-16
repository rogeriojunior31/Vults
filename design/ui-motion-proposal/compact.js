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
  function sessionRequest(s,onDone) {
    const destructive=s.step==='git clean -fd',question=s.status==='question';
    const card=el('section','permission-preview',null,el('strong',null,question?'Question':'Permission required'),el('p','pane-note',question?'Choose how to continue.':destructive?'Destructive actions always ask.':'Run the project build'),el('pre',null,s.step),el('p','mono',s.cwd));
    const decide=allowed=>{s.status=allowed?'working':'idle';s.activity=allowed?'run':null;s.card=s.waiting=false;recordAudit(allowed?'Allowed':'Denied',s.step);onDone();toast('Simulated decision only');remember();};
    if(question){
      const answer=choose('Your answer',['Choose an answer','Review first','Continue with checks'],'Choose an answer',value=>sendAnswer.disabled=value==='Choose an answer');
      const sendAnswer=key(button('Send answer','primary',()=>{s.status='working';s.card=s.waiting=false;recordAudit('Answered',answer.value);onDone();remember();}),'send-answer');sendAnswer.disabled=true;card.append(answer,el('div','inline-actions',null,sendAnswer));
    }else card.append(el('div','inline-actions',null,key(button('Deny',destructive?'primary':'quiet-control',()=>decide(false)),'deny'),key(button('Allow once',destructive?'quiet-control':'primary',()=>decide(true)),'allow')));
    return card;
  }
  function flockFacts() {
    const needs=sessions.filter(s=>s.status==='approval'||s.status==='question');
    const failed=sessions.filter(s=>s.status==='failed'),working=sessions.filter(s=>s.status==='working'||s.status==='thinking');
    const priority=s=>s.status==='approval'||s.status==='question'?0:s.status==='failed'?1:['working','thinking'].includes(s.status)?2:3;
    return {needs,failed,working,ordered:[...sessions].sort((a,b)=>priority(a)-priority(b))};
  }
  function sessionBrief(s) {
    if(state.scenario==='expired'&&s.id==='session-1')return 'The request expired. Nothing is waiting.';
    if(s.status==='approval')return 'Waiting for your permission';
    if(s.status==='question')return 'Waiting for your answer';
    if(s.status==='failed')return 'A check failed. The output needs review.';
    if(s.status==='finished')return 'The agent finished its turn';
    if(s.status==='thinking')return 'Thinking about the next step';
    if(s.status==='working'&&s.activity==='read')return 'Reading project context';
    if(s.status==='working'&&s.activity==='search')return 'Searching for information';
    if(s.status==='working')return s.activity==='edit'?'Editing a project file':s.activity==='run'?'Running project checks':'Working on the current step';
    return 'Ready for the next turn';
  }
  function openQuickSession(s){views.sessionSelected=s.id;setArea('flock','room');}
  function quickQuestion(s){
    setArea('chat','quick');const question=phrase('What is {project} doing now?',{project:s.project});
    input.value=input.value.trim()?input.value+'\n'+question:question;input.dispatchEvent(new Event('input'));input.focus({preventScroll:true});
  }
  function renderCompactFlock(host) {
    host.classList.add('compact-flock');
    host.append(pageHeading('Flock',null,key(button('Open in Room','quiet-control',()=>setArea('flock','room')),'compact-open-room')));
    if(stageState(host,'No sessions right now.'))return;
    const capacity=Math.max(1,Math.min(6,Math.floor((windowEl.getBoundingClientRect().width-32)/64)));
    const facts=flockFacts(),visible=facts.ordered.slice(0,capacity),stage=el('div','flock-stage compact-wire');
    let selected=sessions.find(s=>s.id===views.sessionSelected)??facts.ordered[0];
    if(!selected){host.append(el('p','pane-note','No sessions right now.'));return;}
    if(!visible.some(s=>s.id===selected.id)){visible[visible.length-1]=selected;}
    // Attention chooses who is visible; the wire keeps each bird's relative place.
    visible.sort((a,b)=>sessions.indexOf(a)-sessions.indexOf(b));
    const picks=el('div','compact-bird-picks'),focus=el('section','compact-focus'),queue=el('div','compact-attention'),footer=el('div','compact-footer',null,queue);
    const signals=el('div','compact-signals');signals.setAttribute('aria-label',L('Session overview'));
    function signal(rows,one,many,glyph,tone){
      const b=key(button(null,'compact-signal '+tone,()=>select(rows[0])),'signal-'+glyph);b.disabled=!rows.length;
      b.append(A.icon(glyph,13),el('span','counter',phrase(rows.length===1?one:many,{n:rows.length})));signals.append(b);
    }
    signal(facts.needs,'1 request','{n} requests','person','attention');signal(facts.failed,'1 failure','{n} failures','retry','failure');signal(facts.working,'1 working','{n} working','play','');
    host.append(signals,stage,focus,footer);const flock=mountFlock(stage,false,visible);stage.append(picks);
    picks.style.gridTemplateColumns='repeat('+visible.length+',minmax(0,1fr))';
    visible.forEach(s=>{const b=key(button(null,'compact-bird-pick',()=>select(s)),s.id);b.append(el('span',null,s.project));b.setAttribute('aria-label',phrase('Select {project}: {status}',{project:s.project,status:L(sessionBrief(s))}));b.setAttribute('data-tooltip',s.project);bindBirdFocus(b,flock,s);picks.append(b);});
    if(sessions.length>visible.length)footer.append(key(button(phrase('See all {n} sessions',{n:sessions.length}),'compact-all',()=>setArea('flock','room')),'compact-all-sessions'));
    function select(s){if(!s)return;selected=s;views.sessionSelected=s.id;if(!visible.some(v=>v.id===s.id)){setArea('flock','quick');return;}paint(true);remember();}
    function paint(animate=false){
      for(const b of picks.children)b.setAttribute('aria-pressed',String(b.dataset.key===selected.id));
      focus.setAttribute('aria-live','polite');
      focus.replaceChildren(el('div','compact-focus-heading',null,el('strong',null,selected.project),el('span',null,labelFor(selected.agent))),el('p',null,sessionBrief(selected)));
      const pending=selected.status==='approval'||selected.status==='question';
      const action=key(button(pending?'Review request':selected.status==='failed'?'Review failure':selected.status==='finished'?'View result':'Follow in Room','primary',()=>{
        if(!pending){openQuickSession(selected);return;}
        let close;const card=sessionRequest(selected,()=>{close();setArea('flock','quick');$('.compact-session-action')?.focus({preventScroll:true});});close=openEditor('Review request',card);
      }),'compact-session-action');action.classList.add('compact-session-action');
      const ask=key(button('Ask Zeca','quiet-control',()=>quickQuestion(selected)),'compact-ask');ask.disabled=!settings.zeca;focus.append(el('div','inline-actions',null,action,ask));
      queue.replaceChildren();
      const next=facts.ordered.find(s=>s.id!==selected.id&&['approval','question','failed'].includes(s.status));
      if(next){const b=key(button(null,'compact-next',()=>select(next)),'compact-next');b.append(A.icon(next.status==='failed'?'retry':'person',13),el('span',null,phrase('Also needs attention: {project}',{project:next.project})),A.icon('chevron',12));queue.append(b);}
      if(animate)enter(focus,'snappy',0,1);
    }
    paint();
  }
  function compactInsight(kind,title,body,glyph,action,tone=''){
    const b=key(button(null,'compact-insight '+tone,action),'insight-'+kind);
    b.append(A.icon(glyph,17),el('span','compact-insight-copy',null,el('strong',null,title),el('small',null,body)),A.icon('chevron',13));return b;
  }
  function openConnection(row){views.connectionSelected=row.id;views.connectionQuery='';views.connectionSection='Pull requests';setArea('connections','room');}
  function renderCompactSummary(host) {
    host.classList.add('compact-summary');
    host.append(pageHeading('Overview',null,key(button('Open Activity','quiet-control',()=>setArea('activity','room')),'compact-open-activity')));
    if(stageState(host,'No updates right now.'))return;
    const failed=settings.github?prs.filter(row=>!row.checks&&!row.merged):[],ready=settings.github?prs.filter(row=>row.checks&&!row.merged):[],reviews=tasks.filter(row=>row.status==='For review');
    const feed=el('div','compact-feed');host.append(feed);
    if(failed.length){const row=failed[0];feed.append(compactInsight('checks',failed.length===1?'A check needs your attention':phrase('{n} pull requests need attention',{n:failed.length}),row.repo+' '+row.id+' · '+L(row.title),'retry',()=>openConnection(row),'attention'));}
    if(reviews.length){const row=reviews[0];feed.append(compactInsight('task',reviews.length===1?'A task is ready for review':phrase('{n} tasks are ready for review',{n:reviews.length}),row.title,'tasks',()=>{views.taskQuery=L(row.title);views.taskFilter='All tasks';setArea('tasks','room');}));}
    if(settings.memory&&memory.pending.length)feed.append(compactInsight('memory',memory.pending.length===1?'A memory proposal is waiting':phrase('{n} memory proposals are waiting',{n:memory.pending.length}),'Review before Zeca remembers','sideRight',()=>{views.memoryTab='Waiting for you';setArea('memory','room');}));
    if(!feed.children.length)feed.append(el('div','compact-clear',null,A.icon('check',20),el('strong',null,'Nothing needs your attention'),el('p',null,'Open Room when you want the full picture.')));
    if(ready.length)host.append(compactInsight('ready',ready.length===1?'1 pull request passed its checks':phrase('{n} pull requests passed their checks',{n:ready.length}),'Review before merging','pull',()=>openConnection(ready[0]),'quiet'));
    const last=auditRows[0];
    if(settings.history&&last)host.append(el('div','compact-recent',null,el('span',null,'Latest decision'),el('span',null,last.project+' · '+L(last.answer)),key(button('Audit','',()=>{state.activity='audit';setArea('activity','room');}),'compact-open-audit')));
  }
