  const settings = { zeca:true, sounds:false, volume:50, notifications:true, presence:'Island', fold:15, hover:false, autostart:false, media:false, usage:false, history:true, memory:true, summary:false, suggestions:false, sleep:false, embeddings:false, voice:true, access:'Asks first', native:true, web:false, github:true, visitors:true, animations:prefs.motion, species:'atratus', outfit:'none', pool:'Brazil', retention:'90 days', language:'pt-BR',dailyBudget:10,phone:false,provider:'CLI',chatFolder:'Vults',recordsCleared:false,rules:[{command:'npm run build',project:'Vults'}] };
  const views = { taskQuery:'', taskFilter:'All tasks', connectionQuery:'', connectionSection:'Pull requests', connectionSelected:'#364', sessionQuery:'', sessionFilter:'All sessions', sessionSelected:'session-1', flockDetail:'session', flockStep:0, flockFile:'all', recordQuery:'', week:0, day:3, memoryTab:'Waiting for you', recentQuery:'',chatScroll:0,quickArea:'chat',roomArea:'flock' };
  const tasks = [
    {id:'task-1',title:'Refine the chat composer',repo:'Vults',engine:'Codex',status:'In progress',done:2,criteria:['Keep the draft on one surface','Keep the transcript readable','Preserve keyboard navigation'],checked:[true,true,false],workspace:true,issue:'#364'},
    {id:'task-2',title:'Make model selection easier',repo:'Vults',engine:'Claude Code',status:'For review',done:3,criteria:['Keep the draft on one surface','Keep the transcript readable','Preserve keyboard navigation'],checked:[true,true,true],workspace:true,issue:''},
    {id:'task-3',title:'Review the connection checks',repo:'frontend',engine:'Codex',status:'Ready to continue',done:1,criteria:['Keep the draft on one surface','Keep the transcript readable','Preserve keyboard navigation'],checked:[true,false,false],workspace:false,issue:''},
  ];
  const prs = [{id:'#364',title:'Refine the chat composer',repo:'Vults',checks:true,merged:false},{id:'#363',title:'Make model selection easier',repo:'Vults',checks:true,merged:false},{id:'#361',title:'Review the connection checks',repo:'frontend',checks:false,merged:false}];
  const sessions = [
    {id:'session-1',agent:'codex',project:'Vults',status:'approval',activity:'run',species:'atratus',step:'npm run build',step_count:12,subagents:0},
    {id:'session-2',agent:'claude',project:'frontend',status:'working',activity:'edit',species:'aura',step:'ui/src/chat.ts',step_count:8,subagents:1},
    {id:'session-3',agent:'codex',project:'docs',status:'finished',activity:null,species:'papa',step:'docs/guide.md',step_count:6,subagents:0},
    {id:'session-4',agent:'opencode',project:'api',status:'failed',activity:'run',species:'vultur',step:'cargo test',step_count:4,subagents:0},
    {id:'session-5',agent:'claude',project:'design',status:'thinking',activity:'think',species:'burrovianus',step:'Review the layout',step_count:3,subagents:0},
  ].map(s => ({...s,cwd:'/projects/'+s.project,attention:s.status==='approval'?'needs-you':'quiet',card:s.status==='approval',waiting:s.status==='approval',raise:true,steps:['Read project context',s.step],note:null,editor:'Terminal',bird_chosen:false,branch:'feature/ui',roost:s.project}));
  A.assignSpecies(sessions);
  const memory = { pending:[{id:'proposal-1',place:'Vults',text:'Keep the transcript readable'},{id:'proposal-2',place:'About you',text:'Preserve keyboard navigation'}], facts:[{id:'fact-1',place:'Vults',text:'Keep the draft on one surface'}], history:[] };
  const decisions = { chat:'approved', connections:'approved', flock:'pending', tasks:'pending', activity:'pending', memory:'pending', settings:'pending', motion:'pending' };
  const notes = {};
  const conversations = [{id:'current',title:'Current conversation',messages:[],draft:''},{id:'recent',title:'What changed recently?',messages:[{who:'you',text:'What changed recently?'},{who:'zeca',text:'I reviewed the recent changes.',sample:true}],draft:''}];
  let taskDraft={title:'',repo:'Vults',criteria:'',engine:'Codex'}, activeScenes=[], flockObserver, flockResize=() => {}, lastManualScroll=0;
  // Keep each surface's director alive so navigation does not replay arrivals.
  const flockScenes=new Map();
  let surfaceAnimation,contentAnimation,lastSurface;
  const surface=$('.surface-shell');
  function layoutSurface(animate=false) {
    const stage=$('.review-stage'),box=stage.getBoundingClientRect(),preview=$('.viewport-select').value;
    const availableWidth=Math.max(0,(stage.clientWidth||box.width||1168)-24);
    const availableHeight=preview==='short'?360:Math.max(280,(window.innerHeight||1000)-Math.max(0,box.top)-20);
    const next=A.fitSurface(state.mode,preview==='narrow'?Math.min(480,availableWidth):availableWidth,availableHeight);
    const measured=surface.getBoundingClientRect(),previous={width:measured.width,height:measured.height,radius:Number.parseFloat(window.getComputedStyle?.(surface).borderTopLeftRadius)||lastSurface?.radius||next.radius};
    surfaceAnimation?.cancel();contentAnimation?.cancel();
    for(const name of ['width','height'])surface.style[name]=next[name]+'px';surface.style.borderRadius=next.radius+'px';surface.setAttribute('data-mode',state.mode);
    windowEl.style.width=Math.max(0,next.width-2)+'px';windowEl.style.height=Math.max(0,next.height-2)+'px';windowEl.style.left='50%';windowEl.style.transform='translateX(-50%)';
    windowEl.classList.toggle('tight',next.height<(state.mode==='quick'?380:460));windowEl.classList.toggle('very-short',next.height<360);surface.setAttribute('data-dimensions',next.width+'x'+next.height);
    if(animate&&lastSurface&&prefs.motion&&surface.animate&&(Math.abs(previous.width-next.width)>1||Math.abs(previous.height-next.height)>1)){
      surfaceAnimation=surface.animate(A.surfaceFrames(previous,next),{duration:480,fill:'none'});
      const content=state.mode==='mini'?$('.mini-strip'):windowEl;
      contentAnimation=content.animate([{opacity:0,translate:'0 4px'},{opacity:1,translate:'0 0'}],{duration:200,delay:70,easing:'cubic-bezier(0.16,1,0.3,1)',fill:'backwards'});
    }
    lastSurface=next;positionOpenMenus();resizeFlock();
  }
  const menuAnimations=new Map();
  function positionMenu(panel,trigger) {
    if(panel.hidden)return;
    const rect=windowEl.getBoundingClientRect(),anchor=trigger.getBoundingClientRect();
    panel.style.maxHeight='312px';
    const placement=A.placePopover(rect,anchor,panel.scrollHeight,$('.product-nav').getBoundingClientRect().bottom,state.mode!=='quick');
    for(const name of ['left','top','width','maxHeight'])panel.style[name]=placement[name]+'px';panel.style.transformOrigin=placement.origin;
  }
  function positionOpenMenus(){positionMenu(popover,modelTrigger);positionMenu(history,historyTrigger);}
  function openMenu(panel,trigger){
    menuAnimations.get(panel)?.cancel();panel.setAttribute('data-open','true');panel.hidden=false;panel.inert=false;trigger.setAttribute('aria-expanded','true');positionMenu(panel,trigger);enter(panel,'snappy',4,.99);
  }
  function closeMenu(panel,trigger,returnFocus=false,animate=true){
    menuAnimations.get(panel)?.cancel();panel.setAttribute('data-open','false');trigger.setAttribute('aria-expanded','false');panel.inert=true;
    if(!panel.hidden&&prefs.motion&&animate&&panel.animate){const exit=panel.animate([{opacity:1,transform:'none'},{opacity:0,transform:'translateY(4px) scale(.99)'}],{duration:120,easing:'cubic-bezier(0.16,1,0.3,1)'});menuAnimations.set(panel,exit);exit.finished.then(()=>{if(panel.getAttribute('data-open')==='false')panel.hidden=true;}).catch(()=>{});}
    else panel.hidden=true;
    if(returnFocus)trigger.focus({preventScroll:true});
  }
  function settleMotion(){for(const animation of surface.getAnimations?.({subtree:true})??[])animation.cancel();surfaceAnimation?.cancel();contentAnimation?.cancel();noticeAnimation?.cancel();$('.notice-slot').hidden=!state.notice;for(const panel of [popover,history]){menuAnimations.get(panel)?.cancel();panel.hidden=panel.getAttribute('data-open')!=='true';}}
  window.addEventListener('resize',()=>{layoutSurface(false);resizeCompactFlock();});
  const key = (node,value) => {node.setAttribute('data-key',value);return node;};
  const labelFor = agent => ({claude:'Claude Code',codex:'Codex',opencode:'OpenCode'})[agent]??agent;
  const statusFor = s => ({approval:'Needs you',working:'In progress',thinking:'Thinking…',finished:'Finished',failed:'Failed',idle:'Idle'})[s]??s;
  const phrase = (text,vars) => {let out=L(text);for(const [key,value] of Object.entries(vars))out=out.replaceAll('{'+key+'}',String(value));return out;};
  function snapshot() {
    const data={settings,views,decisions,notes,mode:state.mode,category:state.category,activity:state.activity,conversation:state.conversation,draft:input.value.slice(0,2000),files:state.files,messages:state.messages.slice(-6).map(m => ({...m,text:m.text.slice(0,500)})),tasks:tasks.slice(0,12),memory,taskDraft};
    return JSON.stringify(data).length<14000?data:{settings,views,decisions,notes,mode:state.mode,draft:data.draft,category:state.category,activity:state.activity};
  }
  function restorePrototype(saved) {
    const data=saved?.privateContent, nav=saved?.modelContent;
    if (nav && AREAS.includes(nav.area)) state.area=nav.area;
    if (nav?.mode && ['mini','quick','room','full'].includes(nav.mode)) state.mode=nav.mode;
    if (nav?.engine && ['Claude Code','Codex'].includes(nav.engine)) state.engine=nav.engine;
    if (typeof nav?.model==='string') state.model=nav.model;
    if (!data || typeof data!=='object') return;
    if (data.settings && typeof data.settings==='object') for(const name of Object.keys(settings)) if (typeof data.settings[name]===typeof settings[name]) settings[name]=data.settings[name];
    if (data.settings) for(const [name,value] of Object.entries(data.settings)) if((name.startsWith('agent-')||name.startsWith('pin-')||name.startsWith('mute-'))&&typeof value==='boolean')settings[name]=value;
    if (data.views && typeof data.views==='object') for(const name of Object.keys(views)) if (typeof data.views[name]===typeof views[name]) views[name]=data.views[name];
    views.quickArea=['activity','connections'].includes(views.quickArea)?'summary':['flock','chat','summary'].includes(views.quickArea)?views.quickArea:'chat';
    if (typeof data.draft==='string') input.value=data.draft;
    if (Array.isArray(data.files)) state.files=data.files.filter(name=>typeof name==='string').slice(0,4);
    if (Array.isArray(data.messages)) state.messages=data.messages.filter(m => ['you','zeca'].includes(m.who)&&typeof m.text==='string').slice(-8);
    if (data.decisions) for (const area of Object.keys(decisions)) if (['approved','pending','refine'].includes(data.decisions[area])) decisions[area]=data.decisions[area];
    if (data.notes) for(const [area,note] of Object.entries(data.notes)) if(typeof note==='string' && area in decisions) notes[area]=note.slice(0,800);
    if (Array.isArray(data.tasks)) {const valid=data.tasks.filter(row=>typeof row.id==='string'&&typeof row.title==='string'&&typeof row.repo==='string'&&Array.isArray(row.criteria)&&Array.isArray(row.checked)&&['Codex','Claude Code'].includes(row.engine)&&['In progress','For review','Ready to continue'].includes(row.status));if(valid.length===data.tasks.length)tasks.splice(0,tasks.length,...valid.slice(0,15));}
    if (data.memory && ['pending','facts','history'].every(name=>Array.isArray(data.memory[name]))) Object.assign(memory,data.memory);
    if (data.taskDraft && typeof data.taskDraft.title==='string') taskDraft={...taskDraft,...data.taskDraft};
    if (['general','chat','flock','agents','approvals','projects','activity','connectors','phone','about'].includes(data.category)) state.category=data.category;
    if (['week','projects','history','cost','traces','audit'].includes(data.activity)) state.activity=data.activity;
    language=['en','pt-BR','es','zh'].includes(settings.language)?settings.language:'pt-BR';A.setLang(language);
    prefs.motion=settings.animations&&!matchMedia('(prefers-reduced-motion: reduce)').matches;
    A.setZeca(settings.zeca);A.setZecaSpecies(settings.species);A.setZecaLook(settings.outfit==='none'?null:settings.outfit);
    root.classList.toggle('motion-off',!prefs.motion);
  }
  log.addEventListener('wheel',() => lastManualScroll=Date.now(),{passive:true});
  log.addEventListener('touchstart',() => lastManualScroll=Date.now(),{passive:true});
  log.addEventListener('pointerdown',() => lastManualScroll=Date.now());
  function resizeFlock(){flockResize();}
  function stopScenes() {for(const scene of activeScenes) scene.setActive(false);activeScenes=[];flockObserver?.disconnect();flockResize=() => {};}
  function mountFlock(host,mini=false,source=sessions,onLayout) {
    const base=mini?A.COMPACT_SCENE:state.mode==='quick'?A.HABITAT_SCENE:A.NEST_HABITAT_SCENE;
    const layout={...base,flights:base.flights&&prefs.motion},cacheKey=(mini?'mini':state.mode==='quick'?'quick':'room')+'-'+prefs.motion;
    let scene=flockScenes.get(cacheKey);
    if(!scene){scene=new A.Scene(layout);flockScenes.set(cacheKey,scene);}
    activeScenes.push(scene);host.append(scene.canvas);scene.canvas.setAttribute('role','img');scene.canvas.setAttribute('aria-label',L('Live session birds'));
    const update=() => {
      const width=Math.floor(host.getBoundingClientRect().width||layout.width);
      const scale=layout.vults.scale, count=Math.min(source.length,Math.max(1,Math.floor(width/(scale*32))));
      const shown=source.slice(0,mini?8:count), gutter=mini?48:0, available=width-gutter;
      scene.resize(width,shown.length,i=>({x:Math.round(gutter+(i+.5)*available/shown.length-scale*12),wire:layout.vults.at(0).wire}));
      scene.update(shown,null,mini&&settings.zeca?{clip:'idle',agent:'codex'}:null);scene.setActive(!document.hidden&&(mini||!windowEl.classList.contains('tight')));
      onLayout?.(scene,shown,width);
      if(!prefs.motion)window.setTimeout(()=>scene.setActive(false),0);
    };
    update();flockResize=update;
    if (typeof ResizeObserver!=='undefined') {flockObserver=new ResizeObserver(update);flockObserver.observe(host);}
    return scene;
  }
  document.addEventListener('visibilitychange',()=> {for(const scene of activeScenes)scene.setActive(!document.hidden&&prefs.motion);});
  function paintSuggestions() {
    const suggestions=$('.room-suggestions');suggestions.replaceChildren();
    suggestions.hidden=state.mode==='quick'||state.mode==='mini'||state.messages.length>0||state.busy;
    if(!suggestions.hidden) ['What changed recently?','What is this project?','Explain the last error'].forEach(word=>suggestions.append(button(word,'',()=> {input.value=L(word);input.dispatchEvent(new Event('input'));input.focus();})));
  }
  function stageState(host,emptyTitle,emptyAction) {
    if(!['empty','loading','error'].includes(state.scenario))return false;
    const panel=el('section','empty-state');panel.setAttribute('role',state.scenario==='error'?'alert':'status');
    if(state.scenario==='loading')panel.append(el('h3',null,'Loading'),el('div','loading-line'),el('div','loading-line'));
    else if(state.scenario==='error')panel.append(A.icon('retry',20),el('h3','state-error','Could not update this view.'),el('p',null,'Your draft and selections stay here.'),button('Try again','quiet-control',()=> {state.scenario='ready';setArea(state.area,state.mode);}));
    else panel.append(A.icon('list',22),el('h3',null,emptyTitle),el('p',null,'This is an example empty state.'),emptyAction??button('Show sample data','quiet-control',()=> {state.scenario='ready';setArea(state.area,state.mode);}));
    host.append(panel);return true;
  }
  function openEditor(title,content,onSave,saveLabel='Save') {
    const overlay=$('.editor-overlay'), dialog=$('.editor-dialog'), form=dialog.querySelector('form'), previous=document.activeElement;
    dialog.querySelector('h2').textContent=L(title);$('.editor-content').replaceChildren(content);$('.editor-error').hidden=true;
    const close=()=> {overlay.hidden=true;windowEl.querySelector('.product-nav').inert=false;$('.chat-shell').inert=false;$('.workspace-view').inert=false;previous?.isConnected?previous.focus():$('.product-nav button')?.focus();};
    const cancel=button('Cancel','quiet-control',close), save=button(saveLabel,'primary',()=>{});save.type='submit';
    $('.editor-actions').replaceChildren(cancel,...(onSave?[save]:[]));$('.editor-close').replaceChildren(A.icon('close',14));$('.editor-close').onclick=close;
    form.onsubmit=event=> {event.preventDefault();try{if(onSave?.()!==false){close();remember();}}catch(error){$('.editor-error').textContent=L(error.message);$('.editor-error').hidden=false;}};
    overlay.hidden=false;enter(dialog,'gentle',8,.98);$('.product-nav').inert=true;$('.chat-shell').inert=true;$('.workspace-view').inert=true;
    const fields=()=> [...dialog.querySelectorAll('button,input,textarea,select,a[href]')].filter(n=>!n.disabled&&!n.hidden);
    (dialog.querySelector('input,textarea,select')??cancel).focus();
    overlay.onkeydown=event=> {
      if(event.isComposing||event.keyCode===229)return;
      if(event.key==='Escape'){event.preventDefault();event.stopPropagation();close();}
      if(event.key==='Tab'){const list=fields(),i=list.indexOf(document.activeElement);if((event.shiftKey&&i<=0)||(!event.shiftKey&&i===list.length-1)){event.preventDefault();list[event.shiftKey?list.length-1:0]?.focus();}}
    };
    return close;
  }
  function field(label,value='',type='text',onInput) {
    const control=el(type==='textarea'?'textarea':'input');if(type!=='textarea')control.type=type;control.value=value;control.setAttribute('aria-label',L(label));
    if(onInput)control.addEventListener('input',()=>onInput(control.value));return {element:el('label',null,null,el('span',null,label),control),control};
  }
  function choose(label,choices,value,onChange) {
    const select=el('select');select.setAttribute('aria-label',L(label));
    choices.forEach(choice=> {const option=el('option',null,typeof choice==='string'?choice:choice.label);option.value=typeof choice==='string'?choice:choice.value;select.append(option);});
    select.value=String(value);select.addEventListener('change',()=>onChange(select.value));return select;
  }
  function saveConversation() {const current=conversations.find(c=>c.id===state.conversation);if(current){current.messages=state.messages.map(m=>({...m}));current.draft=input.value;current.engine=state.engine;current.model=state.model;}}
  function openConversation(id) {
    if(state.busy){toast('Stop the current turn first.');return;}
    saveConversation();const next=conversations.find(c=>c.id===id);if(!next)return;
    state.conversation=id;state.messages=next.messages.map(m=>({...m}));input.value=next.draft;state.engine=next.engine??'Claude Code';state.model=next.model??'Default model';paintConversation();paintRecent();remember();
  }
  function paintRecent() {
    const host=$('.room-recent');host.replaceChildren(el('h3',null,'Conversations'),button('New','quiet-control',()=>newConversation()));
    const finder=search('Search',value=> {views.recentQuery=value;paintList();});finder.querySelector('input').value=views.recentQuery;host.append(finder);
    const list=el('div');host.append(list);
    function paintList(){list.replaceChildren();conversations.filter(c=>L(c.title).toLowerCase().includes(views.recentQuery.toLowerCase())).forEach(c=> {const b=button(c.title,'',()=>openConversation(c.id));b.setAttribute('aria-pressed',String(c.id===state.conversation));list.append(b);});}
    paintList();host.append(button('Memory','quiet-control',()=>setArea('memory','room')));
  }
  function newConversation() {
    if(state.busy){toast('Stop the current turn first.');return;}
    saveConversation();const id='conversation-'+(conversations.length+1);conversations.unshift({id,title:'New conversation',messages:[],draft:''});state.conversation=id;input.value='';showExample('empty');paintRecent();remember();
  }
  function prepareChat(text) {setArea('chat','room');if(input.value.trim())input.value+='\n'+L(text);else input.value=L(text);input.dispatchEvent(new Event('input'));input.focus();}
  function openFolder(){const select=choose('Project',['Vults','frontend','docs','api'],settings.chatFolder??'Vults',()=>{});openEditor('Project context',el('label',null,null,el('span',null,'Project'),select),()=>{settings.chatFolder=select.value;$('.folder-trigger').textContent=select.value;return true;});}
  function askSession(project){prepareChat(phrase('What is {project} doing now?',{project}));}
  function openPalette() {
    const content=el('div'), list=el('div');let close;
    const paint=query=> {list.replaceChildren();[['flock','Flock'],['chat','Chat'],['tasks','Tasks'],['activity','Activity'],['connections','Connections'],['settings','Settings'],['memory','Memory'],['review','Review & releases']].filter(([,label])=>L(label).toLowerCase().includes(query.toLowerCase())).forEach(([area,label])=>list.append(button(label,'',()=> {close();setArea(area,'room');})));};
    const query=field('Search','','search',paint);content.append(query.element,list);paint('');close=openEditor('Search',content);
  }
  function renderFlockSummary(host,onSelected=()=>{}) {
    if(stageState(host,'No sessions right now.'))return;
    const stage=el('div','flock-stage'),targets=el('div','lab-bird-targets');host.append(stage);let redrawBirdTargets=()=>{};
    const flock=mountFlock(stage,false,sessions,(scene,shown)=>{
      redrawBirdTargets=()=>{targets.replaceChildren();for(const slot of scene.slots()){
        const session=shown.find(s=>s.agent+':'+s.id===slot.key);if(!session)continue;
        const b=key(button(null,'lab-bird-target cursor-interaction',()=>select(session)), 'bird-'+session.id);
        b.style.left=Math.max(0,slot.x-8)+'px';b.style.width=slot.width+16+'px';b.setAttribute('aria-label',phrase('Select {project}: {status}',{project:session.project,status:L(sessionBrief(session))}));b.setAttribute('aria-pressed',String(views.sessionSelected===session.id));b.setAttribute('data-tooltip',session.project);bindBirdFocus(b,scene,session);targets.append(b);
      }};redrawBirdTargets();
    });stage.append(targets);
    if(sessions.length>5)host.append(el('p','pane-note',phrase('{n} sessions; select one below. Only the visible birds use the wire.',{n:sessions.length})));
    const list=el('div'), detail=el('article','session-detail'), searcher=search('Find a session…',q=>{views.sessionQuery=q;paint();remember();});searcher.querySelector('input').value=views.sessionQuery;
    const filter=segment(['All sessions','Needs you','In progress'],value=> {views.sessionFilter=value;paint();remember();});
    [...filter.children].forEach(b=>b.setAttribute('aria-pressed',String(b.textContent===L(views.sessionFilter))));
    host.append(el('div','workspace-toolbar',null,filter,searcher),el('div','session-grid',null,list,detail));
    function paint() {
      list.replaceChildren();const found=sessions.filter(s=>(s.project+' '+labelFor(s.agent)).toLowerCase().includes(views.sessionQuery.toLowerCase())&&(views.sessionFilter==='All sessions'||views.sessionFilter==='Needs you'&&['approval','question'].includes(s.status)||views.sessionFilter==='In progress'&&['working','thinking'].includes(s.status)));
      found.forEach(s=> {const b=key(button(null,'session-row',()=>select(s)),s.id);b.setAttribute('aria-pressed',String(s.id===views.sessionSelected));bindBirdFocus(b,flock,s);b.append(A.icon(s.status==='approval'?'person':s.status==='failed'?'retry':'terminal',16),el('span','session-copy',null,el('strong',null,s.project),el('small',null,labelFor(s.agent))),el('span','status '+(s.status==='approval'?'working':s.status==='failed'?'state-error':'success'),statusFor(s.status)));list.append(b);});
      if(!found.length)list.append(el('p','pane-note','Nothing here matches your search.'),button('Clear filters','quiet-control',()=>{views.sessionQuery='';views.sessionFilter='All sessions';renderAgain();}));
      const s=sessions.find(s=>s.id===views.sessionSelected)??sessions[0];detail.replaceChildren(el('h3',null,s.project),el('span','task-meta',labelFor(s.agent)+' · '+s.branch),el('p','pane-note',statusFor(s.status)));
      if(s.status==='approval'||s.status==='question')detail.append(sessionRequest(s,()=>{paint();resizeFlock();onSelected(s);}));
      if(state.scenario==='expired'&&s.id==='session-1')detail.append(el('section','permission-preview',null,el('h3',null,'Request expired'),el('p','pane-note','This request can no longer be answered.')));
      if(s.status==='failed')detail.append(el('p','state-error','The example run failed. Review its output before continuing.'));
      detail.append(el('div','inline-actions',null,button('Open terminal','quiet-control',()=>openEditor('Open terminal',el('div',null,null,el('p','pane-note','Terminal actions are simulated.'),el('code','mono',s.cwd+'\n'+s.step)))),button('Ask Zeca','quiet-control',()=>askSession(s.project))));
      redrawBirdTargets();
    }
    function select(s){const focused=document.activeElement?.getAttribute('data-key');views.sessionSelected=s.id;paint();onSelected(s);if(focused)host.querySelector('[data-key="'+focused+'"]')?.focus({preventScroll:true});remember();}
    function renderAgain(){host.replaceChildren();stopScenes();renderFlockSummary(host,onSelected);remember();}paint();
  }
  function editTask(row) {
    const draft=row?{title:row.title,repo:row.repo,criteria:row.criteria.join('\n'),engine:row.engine}:taskDraft;
    const title=field('Goal',draft.title,'text',value=>{draft.title=value;remember();}),project=field('Project',draft.repo,'text',value=>{draft.repo=value;remember();}),criteria=field('Criteria, one per line',draft.criteria,'textarea',value=>{draft.criteria=value;remember();});
    const content=el('div',null,null,title.element,project.element,criteria.element,el('p','pane-note','Creating a task does not start an agent.'));
    openEditor(row?'Edit task':'New task',content,()=>{
      if(!draft.title.trim())throw new Error('Enter a goal first.');
      if(!draft.repo.trim())throw new Error('Choose a project first.');
      const next={title:draft.title.trim(),repo:draft.repo.trim(),criteria:draft.criteria.split('\n').map(s=>s.trim()).filter(Boolean),engine:draft.engine};
      if(row){Object.assign(row,next);row.checked=next.criteria.map((_,i)=>row.checked[i]??false);row.done=row.checked.filter(Boolean).length;}
      else {tasks.unshift({...next,id:'task-'+Date.now(),checked:next.criteria.map(()=>false),done:0,status:'Ready to continue',workspace:false,issue:''});taskDraft={title:'',repo:'Vults',criteria:'',engine:'Codex'};}
      state.scenario='ready';setArea('tasks','room');toast('Saved in this prototype');return true;
    },row?'Save':'Create task');
  }
  function renderTasks(host) {
    host.append(pageHeading('Tasks','The next useful action, before maintenance.',key(button('New task','primary',()=>editTask()),'new-task')));
    if(stageState(host,'No tasks yet.',button('New task','quiet-control',()=>editTask())))return;
    const list=el('div'), finder=search('Search',value=>{views.taskQuery=value;paint();remember();});finder.querySelector('input').value=views.taskQuery;
    const filter=segment(['All tasks','In progress','For review'],word=>{views.taskFilter=word;paint();remember();});[...filter.children].forEach(b=>b.setAttribute('aria-pressed',String(b.textContent===L(views.taskFilter))));
    host.append(el('div','workspace-toolbar',null,filter,finder),list);
    function paint() {
      list.replaceChildren();const found=tasks.filter(row=>(views.taskFilter==='All tasks'||row.status===views.taskFilter)&&(L(row.title)+' '+row.repo).toLowerCase().includes(views.taskQuery.toLowerCase()));
      found.forEach(row=> {
        const action=button(row.status==='In progress'?'Stop':row.status==='For review'?'Review':'Continue','quiet-control',()=>{
          if(row.status==='For review'){prepareChat(phrase('Review the task: {task}',{task:L(row.title)}));return;}
          row.status=row.status==='In progress'?'Ready to continue':'In progress';row.workspace=true;paint();remember();toast('Simulated execution only');
        });
        const engine=choose('Agent',['Codex','Claude Code'],row.engine,value=>{row.engine=value;remember();});engine.disabled=row.status==='In progress';engine.className='engine-select';
        const item=key(el('article','task-row',null,el('div','task-top',null,A.icon('tasks',17),el('div','task-identity',null,el('h3',null,row.title),el('div','task-meta',null,el('span',null,row.repo),engine,el('span','status '+(row.status==='In progress'?'working':'success'),row.status))),action)),row.id);
        const d=el('details','criteria'),summary=el('summary',null,null,A.icon('chevron',12),el('span','counter',row.checked.filter(Boolean).length+'/'+row.criteria.length),el('span',null,'Criteria')),body=el('div','criteria-body');
        row.criteria.forEach((text,i)=> {const c=el('input');c.type='checkbox';c.checked=!!row.checked[i];c.addEventListener('change',()=>{row.checked[i]=c.checked;row.done=row.checked.filter(Boolean).length;summary.children[1].textContent=row.done+'/'+row.criteria.length;remember();});body.append(el('label',null,null,c,el('span',null,text)));});
        if(!row.criteria.length)body.append(el('p','pane-note','No criteria yet. Add them in Edit task.'));
        d.append(summary,body);item.append(d);
        const actions=el('div','inline-actions',null,button('Edit task','',()=>editTask(row)),button(row.issue?'Linked issue':'Link issue','',()=>{
          const issue=field('GitHub issue URL',row.issue.startsWith('https:')?row.issue:'','url');openEditor('Link issue',issue.element,()=>{if(!/^https:\/\/github\.com\/[^/]+\/[^/]+\/issues\/\d+$/.test(issue.control.value))throw new Error('Enter a GitHub issue URL.');row.issue=issue.control.value;paint();return true;});
        }),button('Review with Zeca','',()=>prepareChat(row.title)));
        const remove=button('Delete task','',()=>confirm('Delete task','Only this example task is removed.',()=>{tasks.splice(tasks.indexOf(row),1);paint();remember();}));remove.disabled=row.status==='In progress';actions.append(remove);
        const workspace=button('Remove workspace','',()=>confirm('Remove workspace','Only the example workspace is removed. The task stays.',()=>{row.workspace=false;paint();remember();}));workspace.disabled=!row.workspace||row.status==='In progress';actions.append(workspace);
        const more=el('details','run-details',null,el('summary',null,null,A.icon('chevron',12),el('span',null,'More actions')),actions);item.append(more);list.append(item);
      });
      if(!found.length)list.append(el('p','pane-note','Nothing here matches your search.'),button('Clear filters','quiet-control',()=>{views.taskQuery='';views.taskFilter='All tasks';host.replaceChildren();renderTasks(host);remember();}));
    }paint();
  }
  function renderConnections(host) {
    host.append(pageHeading('Connections','Checks and review, without hunting through details.',button('Refresh','quiet-control',()=>{toast('Sample data refreshed');remember();})));
    if(!settings.github){host.append(el('section','empty-state',null,el('h3',null,'GitHub is off'),button('Open connector settings','quiet-control',()=>{state.category='connectors';setArea('settings','room');})));return;}
    if(stageState(host,'No connection news yet.'))return;
    const list=el('div'),detail=el('article','connection-detail'),finder=search('Search',value=>{views.connectionQuery=value;paint();remember();});finder.querySelector('input').value=views.connectionQuery;
    const tabs=segment(['Pull requests','Releases'],word=>{views.connectionSection=word;paint();remember();});[...tabs.children].forEach(b=>b.setAttribute('aria-pressed',String(b.textContent===L(views.connectionSection))));
    host.append(el('div','workspace-toolbar',null,tabs,finder),el('div','connection-columns',null,list,detail));
    function paint() {
      list.replaceChildren();
      if(views.connectionSection==='Releases'){list.append(el('article','content-row',null,el('h3',null,'Vults · release preview'),el('p','pane-note','Release notes and links appear here.')));detail.replaceChildren(el('h3',null,'Release preview'),el('p','pane-note','No release was published by this prototype.'));return;}
      const found=prs.filter(row=>(L(row.title)+' '+row.repo+' '+row.id).toLowerCase().includes(views.connectionQuery.toLowerCase()));
      found.forEach(row=> {const b=key(button(null,'connection-row',()=>{views.connectionSelected=row.id;paint();enter(detail,'gentle',4,1);remember();}),row.id);b.setAttribute('aria-pressed',String(row.id===views.connectionSelected));b.append(A.icon('pull',16),el('div',null,null,el('strong',null,row.title),el('small',null,`${row.repo} ${row.id}`)));list.append(b);});
      if(!found.length){list.append(el('p','pane-note','Nothing here matches your search.'));detail.replaceChildren();return;}
      const selected=found.find(row=>row.id===views.connectionSelected)??found[0];views.connectionSelected=selected.id;
      detail.replaceChildren(el('span','status '+(selected.checks?'success':'working'),selected.merged?'Merged in prototype':selected.checks?'Checks passed':'Needs attention'),el('h3',null,selected.title),el('span','task-meta',`${selected.repo} ${selected.id} · feature/ui`),el('p',null,'Review changes before merging.'));
      const checks=details('Checks',selected.checks?['TypeScript','Rust','UI']:['TypeScript','Rust','UI check failed']);checks.className='run-details';checks.open=!selected.checks;detail.append(checks);
      const merge=button('Squash and merge','quiet-control',()=>confirm('Squash and merge','This only merges the example item. No GitHub action is sent.',()=>{selected.merged=true;paint();remember();toast('Merged in prototype');}));merge.disabled=!selected.checks||selected.merged;
      detail.append(el('div','detail-actions',null,button('Review with Zeca','primary',()=>prepareChat(selected.title)),merge));
    }paint();
  }
  const auditRows=[{time:'10:42',project:'Vults',agent:'Codex',answer:'Allowed',command:'npm run build'},{time:'10:31',project:'frontend',agent:'Claude Code',answer:'Denied',command:'git push origin main'}];
  function recordAudit(answer,command){auditRows.unshift({time:new Date().toLocaleTimeString('en-GB',{hour:'2-digit',minute:'2-digit'}),project:'Vults',agent:'Codex',answer,command});}
  function renderActivity(host) {
    host.append(pageHeading('Activity','Your work, with useful context.'));
    const tabs=el('nav','activity-tabs');tabs.setAttribute('aria-label',L('Activity'));
    const sections=[['week','Week'],['projects','By project'],['history','History'],['cost','Cost'],['traces','Traces'],['audit','Audit']];
    sections.forEach(([id,title])=>{const b=key(button(title,'',()=>{state.activity=id;views.recordQuery='';setArea('activity',state.mode);}),id);b.setAttribute('aria-pressed',String(state.activity===id));tabs.append(b);});host.append(tabs);
    if(stageState(host,'No activity yet.'))return;
    if(settings.recordsCleared&&state.activity!=='projects'&&!(state.activity==='audit'&&auditRows.length)){host.append(el('section','empty-state',null,el('h3',null,'No activity yet.')));return;}
    if(!settings.history&&['week','history','traces','audit'].includes(state.activity)){host.append(el('section','empty-state',null,el('h3',null,'History is off'),button('Open activity settings','quiet-control',()=>{state.category='activity';setArea('settings','room');})));return;}
    const panel=el('div','activity-content');host.append(panel);
    if(state.activity==='week')renderWeek(panel);
    else if(state.activity==='projects')renderProjects(panel);
    else if(state.activity==='cost')renderCost(panel);
    else renderRecords(panel,state.activity);
  }
  function renderWeek(host) {
    const heading=el('h3',null,views.week===0?'This week':'Previous week'), line=el('div','inline-actions',null,button('Previous week','',()=>{views.week--;render();remember();}),heading,button('Next week','',()=>{if(views.week<0)views.week++;render();remember();}));
    host.append(line);let content=el('div');host.append(content);render();
    function render(){heading.textContent=views.week===0?L('This week'):phrase('Week {n}',{n:views.week});content.replaceChildren();
      const values=views.week===0?[2,4,3,6,5,0,0]:[3,2,5,4,2,1,0];
      content.append(el('div','week-summary',null,...[[String(values.reduce((a,b)=>a+b,0)),'Agent turns'],['4','Projects'],['2','Needs you']].map(([value,label])=>el('div',null,null,el('strong',null,value),el('span',null,label)))));
      const chart=el('div','week-chart');chart.setAttribute('role','group');chart.setAttribute('aria-label',L('Agent turns by day'));
      ['Mon','Tue','Wed','Thu','Fri','Sat','Sun'].forEach((day,i)=>{const b=button(null,'week-day'+(views.day===i?' selected':''),()=>{views.day=i;render();remember();});b.setAttribute('aria-pressed',String(views.day===i));b.setAttribute('aria-label',phrase('{day}: {n} turns',{day:L(day),n:values[i]}));const bar=el('span','day-bar');bar.style.height=(values[i]*14)+'px';b.append(el('span','counter',String(values[i])),bar,el('span',null,day));chart.append(b);});content.append(chart);
      content.append(el('h3',null,phrase('{day}: {n} turns',{day:L(['Mon','Tue','Wed','Thu','Fri','Sat','Sun'][views.day]),n:values[views.day]})),el('p','pane-note','All numbers in this prototype are sample data.'),el('article','content-row',null,el('h3',null,'Vults'),el('p',null,'Refine the chat composer'),button('Open project','quiet-control',()=>{state.activity='projects';setArea('activity',state.mode);})),button('Export example recap','quiet-control',()=>download('vults-example-recap.json',JSON.stringify({sample:true,week:views.week,turns:values},null,2))));
    }
  }
  function renderProjects(host) {
    const list=el('div'), finder=search('Search',value=>{views.recordQuery=value;paint();remember();});finder.querySelector('input').value=views.recordQuery;host.append(el('div','workspace-toolbar',null,finder),list);
    function paint(){list.replaceChildren();['Vults','frontend','docs','api'].filter(project=>project.toLowerCase().includes(views.recordQuery.toLowerCase())).forEach((project,i)=>{
      const item=el('article','content-row',null,el('header',null,null,el('h3',null,project),el('span','status '+(i===0?'working':'success'),i===0?'Needs you':'In progress')),el('p','task-meta',`/projects/${project} · feature/ui`));
      item.append(details('Recent changes',['Refine the chat composer','Preserve keyboard navigation']),el('div','inline-actions',null,button('Open session','quiet-control',()=>{views.sessionSelected=sessions.find(s=>s.project===project)?.id??'session-1';setArea('flock','room');}),button('Ask Zeca','',()=>askSession(project))));list.append(item);
    });if(!list.children.length)list.append(el('p','pane-note','Nothing here matches your search.'));}paint();
  }
  function renderCost(host) {
    settings.dailyBudget??=10;
    host.append(el('h3',null,'Daily budget'),el('p','pane-note','Recorded API estimates are separate from subscription usage.'));
    const amount=el('strong','counter','$2.40 / $'+Number(settings.dailyBudget).toFixed(2)),progress=el('progress','budget-progress');progress.value=2.4;progress.max=settings.dailyBudget;progress.setAttribute('aria-label',L('Daily budget'));progress.classList.toggle('warning',2.4/settings.dailyBudget>=.7);progress.classList.toggle('critical',2.4/settings.dailyBudget>=.9);
    host.append(amount,progress,button('Edit budget','quiet-control',()=>{const budget=field('Daily budget',String(settings.dailyBudget),'number');budget.control.min='.01';budget.control.step='.01';openEditor('Daily budget',budget.element,()=>{const n=Number(budget.control.value);if(!Number.isFinite(n)||n<=0)throw new Error('Enter a positive amount.');settings.dailyBudget=n;setArea('activity',state.mode);return true;});}));
    const table=el('table','record-table'), head=el('thead',null,null,el('tr',null,null,...['Project','Agent','Recorded estimate'].map(word=>el('th',null,word)))),body=el('tbody');
    [['Vults','Codex','$1.80'],['frontend','Claude Code','$0.60'],['docs','Codex','Not reported']].forEach(values=>body.append(el('tr',null,null,...values.map(value=>el('td','counter',value)))));table.append(head,body);host.append(table,el('p','pane-note','Amber at 70% · red at 90%'));
  }
  function renderRecords(host,kind) {
    const rows=kind==='audit'?auditRows:[{time:'10:42',project:'Vults',agent:'Codex',answer:'Finished',command:'npm run build'},{time:'10:31',project:'frontend',agent:'Claude Code',answer:'Failed',command:'UI check failed'},{time:'10:20',project:'docs',agent:'Codex',answer:'Finished',command:'Read project context'}];
    const list=el('div'), finder=search('Search',value=>{views.recordQuery=value;paint();remember();});finder.querySelector('input').value=views.recordQuery;host.append(el('div','workspace-toolbar',null,finder,button('Clear filters','',()=>{views.recordQuery='';finder.querySelector('input').value='';paint();remember();})),list);
    function paint(){list.replaceChildren();const found=rows.filter(row=>Object.values(row).join(' ').toLowerCase().includes(views.recordQuery.toLowerCase()));
      if(!found.length){list.append(el('p','pane-note','Nothing here matches your search.'));return;}
      if(kind==='traces'){found.forEach(row=>{const trace=details(row.project+' · '+row.time,[row.command,'Read project context','Local trace example']);trace.open=false;list.append(trace);});return;}
      const table=el('table','record-table'),body=el('tbody');table.append(el('thead',null,null,el('tr',null,null,...['Time','Project','Agent',kind==='audit'?'Answer':'Status'].map(word=>el('th',null,word)))),body);
      found.forEach(row=>{const b=button(row.project,'',()=>openEditor(kind==='audit'?'Audit':'History',el('div',null,null,el('h3',null,row.project),el('p','pane-note',row.agent+' · '+L(row.answer)),el('pre','diff-preview',row.command))));body.append(el('tr',null,null,el('td','counter',row.time),el('td',null,null,b),el('td',null,row.agent),el('td',null,row.answer)));});list.append(table);
    }paint();
  }
  function renderMemory(host) {
    host.append(pageHeading('Memory','Review what Zeca keeps for future conversations.',button('Back to Chat','quiet-control',()=>setArea('chat','room'))));
    if(!settings.memory){host.append(el('section','empty-state',null,el('h3',null,"Zeca's memory is off."),button('Open memory settings','quiet-control',()=>{state.category='chat';setArea('settings','room');})));return;}
    if(stageState(host,'Nothing yet.'))return;
    const tabs=segment(['Waiting for you','What Zeca remembers','History'],value=>{views.memoryTab=value;setArea('memory','room');});[...tabs.children].forEach(b=>b.setAttribute('aria-pressed',String(b.textContent===L(views.memoryTab))));host.append(tabs);
    if(views.memoryTab==='Waiting for you'){
      if(!memory.pending.length)host.append(el('section','empty-state',null,el('h3',null,'Nothing waits.')));
      memory.pending.forEach(proposal=>{
        const act=accepted=>{memory.pending.splice(memory.pending.indexOf(proposal),1);if(accepted)memory.facts.push({...proposal});memory.history.unshift({...proposal,accepted});setArea('memory','room');toast('Saved in this prototype');};
        host.append(key(el('article','content-row',null,el('h3',null,proposal.place),el('div','memory-change',null,el('small',null,'Proposed memory'),el('p',null,proposal.text)),el('div','inline-actions',null,button('Turn down','quiet-control',()=>act(false)),button('Remember','primary',()=>act(true)))),proposal.id));
      });
    }else if(views.memoryTab==='What Zeca remembers'){
      memory.facts.forEach(fact=>host.append(el('article','content-row',null,el('h3',null,fact.place),el('p',null,fact.text),button('Remove','',()=>confirm('Remove memory','Only this example memory is removed.',()=>{memory.facts.splice(memory.facts.indexOf(fact),1);setArea('memory','room');})))));
      if(!memory.facts.length)host.append(el('p','pane-note','Nothing yet.'));
    }else{
      memory.history.forEach(item=>host.append(el('article','content-row',null,el('h3',null,item.accepted?'Remembered':'Turned down'),el('p',null,item.text),button('Undo','quiet-control',()=>{memory.history.splice(memory.history.indexOf(item),1);memory.facts=memory.facts.filter(f=>f.id!==item.id);memory.pending.push({...item});setArea('memory','room');}))));
      if(!memory.history.length)host.append(el('p','pane-note','Nothing yet.'));
    }
  }
  const CATEGORIES=[['general','General','Experience'],['chat','Chat & voice','Experience'],['flock','Flock & Zeca','Experience'],['agents','Agents','Work'],['approvals','Approvals','Work'],['projects','Projects','Work'],['activity','Activity','System'],['connectors','Connectors','System'],['phone','Phone','System'],['about','About','System']];
  function changedSetting(name,value) {
    settings[name]=value;toast('Saved in this prototype');
    if(name==='animations'){prefs.motion=value&&!matchMedia('(prefers-reduced-motion: reduce)').matches;root.classList.toggle('motion-off',!prefs.motion);if(!prefs.motion)settleMotion();for(const s of activeScenes)s.setActive(prefs.motion&&!document.hidden);}
    if(name==='zeca')A.setZeca(value);
    if(name==='species')A.setZecaSpecies(value);
    if(name==='outfit')A.setZecaLook(value==='none'?null:value);
    if(['species','outfit','zeca'].includes(name)){paintBird();for(const scene of activeScenes)scene.refresh();paintNav();}
    paintChrome();
    remember();
  }
  function makeSwitch(name,label) {
    const b=button(null,'toggle',()=>{const on=!settings[name];b.setAttribute('aria-checked',String(on));changedSetting(name,on);enter(b,'snappy',0,.98);},label);b.setAttribute('role','switch');b.setAttribute('aria-checked',String(settings[name]));b.append(el('span'));return b;
  }
  function optionRow(name,title,about,choices){return setting(title,about,choose(title,choices,settings[name],value=>changedSetting(name,typeof settings[name]==='number'?Number(value):value)));}
  function switchRow(name,title,about){return setting(title,about,makeSwitch(name,title));}
  function settingsGroup(title,...rows){return el('section','settings-group',null,el('h3',null,title),...rows);}
  function demoSetup(name) {
    const content=el('div',null,null,el('p','pane-note','Review this example change. No configuration file is written.'),el('pre','diff-preview',null,el('span','diff-add','+ Vults hooks for '+name+'\n'),el('span',null,'  Existing hooks stay in place.\n')),el('p','pane-note','The real installer makes a dated backup before applying a reviewed diff.'));
    openEditor('Review setup',content,()=>{settings['agent-'+name]=true;setArea('settings',state.mode);toast('Setup simulated');return true;},'Apply example');
  }
  function renderSettings(host) {
    host.append(pageHeading('Settings','Essential controls first.'));
    if(stageState(host,'No settings loaded.'))return;
    const categories=el('nav','settings-categories'),content=el('div','settings-content');categories.setAttribute('aria-label',L('Settings'));let last;
    CATEGORIES.forEach(([id,label,group])=>{if(last!==group){categories.append(el('small',null,group));last=group;}const b=key(button(label,'',()=>{state.category=id;paint();remember();}),id);categories.append(b);});
    host.append(el('div','settings-grid',null,categories,content));paint();
    function paint(){content.replaceChildren();for(const b of categories.querySelectorAll('button'))b.setAttribute('aria-pressed',String(b.dataset.key===state.category));
      const title=CATEGORIES.find(([id])=>id===state.category)?.[1]??'General';content.append(el('h3',null,title));
      if(state.category==='general'){
        const languageSelect=choose('Language',[{value:'pt-BR',label:'Portugu\u00eas (Brasil)'},{value:'en',label:'English'},{value:'es',label:'Espa\u00f1ol'},{value:'zh',label:'\u7b80\u4f53\u4e2d\u6587'}],language,value=>{language=value;A.setLang(value);settings.language=value;translateStatic();paintConversation();setArea('settings',state.mode);});
        const volume=el('input');volume.type='range';volume.min='0';volume.max='100';volume.value=settings.volume;volume.setAttribute('aria-label',L('Volume'));const number=el('span','counter',String(settings.volume)+'%');volume.addEventListener('input',()=>{number.textContent=volume.value+'%';settings.volume=Number(volume.value);remember();});
        content.append(settingsGroup('Language & sound',setting('Language','The language used across Vults.',languageSelect),switchRow('sounds','Sounds','Short cues for attention and completed work.'),setting('Volume','Preview controls do not play audio.',el('div','control-group',null,number,volume)),switchRow('media','Now playing','Show local media controls when enabled.')),
          settingsGroup('Island behavior',optionRow('presence','Presence','Choose how Vults stays visible.',['Island','Panel','Quiet','Paused']),optionRow('fold','Fold the island','Seconds after the pointer leaves.',[5,10,15,30,60].map(n=>({value:String(n),label:n+' s'}))),switchRow('hover','Open on hover','Open the island when the pointer rests on it.'),switchRow('autostart','Start with the desktop','Open Vults when you log in.')),
          settingsGroup('Attention',switchRow('notifications','Notifications','Desktop notifications in Panel mode.'),switchRow('usage','Codex usage','Read plan usage only when enabled.')),
          details('Keyboard shortcuts',['Ctrl+K · Search','Alt+1–5 · Workspace tabs','Escape · Close the current overlay']));
      }else if(state.category==='chat'){
        const access=choose('Who answers Zeca\'s cards',['Asks first','Your policies','Full access'],settings.access,value=>{
          if(value==='Full access'){access.value=settings.access;confirm('Full access','This preview changes only the example access mode. Destructive commands still ask.',()=>{changedSetting('access',value);access.value=value;});}
          else changedSetting('access',value);
        });
        content.append(settingsGroup('Conversation',switchRow('native','Your CLI setup','Changes apply to the next conversation.'),setting("Who answers Zeca's cards",'Every answer is audited. Destructive commands always ask.',access)),
          settingsGroup('Memory',switchRow('memory',"Zeca's memory",'Keep a local journal. Recalled facts join the conversation.'),switchRow('summary','A summary of each day','Uses your subscription for a daily summary; up to 8 requests a day.'),switchRow('suggestions','Suggest what to remember','Proposals wait for your review; up to 20 requests a day.'),switchRow('sleep','Let Zeca sleep on it','Proposed merges wait for you; up to 4 requests a day.'),switchRow('embeddings','Recall by meaning','Uses your local embedding model.'),setting('Review memories','Inspect proposals and saved facts.',button('Open Memory','quiet-control',()=>setArea('memory','room')))),
          settingsGroup('Voice',switchRow('voice','Talk to the chat','Transcription is local. This prototype simulates microphone activity.'),setting('Voice model','Example local model, already available.',button('Show voice example','quiet-control',()=>{setArea('chat','room');if(!state.voice)mic.click();}))),
          settingsGroup('API & web',switchRow('web','Web for Zeca','External content never grants permission. No requests leave this prototype.'),setting('Provider','CLI subscription or your API provider.',choose('Provider',['CLI','OpenRouter','Local model'],settings.provider??'CLI',value=>changedSetting('provider',value))),setting('API key','The real app stores keys in the OS keyring.',button('View key setup','quiet-control',()=>openEditor('API key',el('div',null,null,el('p','pane-note','Key entry belongs to the real app. This prototype uses no credentials.')))))));
      }else if(state.category==='flock'){
        const preview=el('div','settings-bird');const bird=new A.Scene(A.FOCUS_SCENE);activeScenes.push(bird);preview.append(bird.canvas);bird.update([],null,{clip:'idle',agent:'codex'});bird.setActive(!document.hidden);if(!prefs.motion)window.setTimeout(()=>bird.setActive(false),0);
        content.append(preview,settingsGroup('Companions',switchRow('zeca','Zeca','Off, the flock and decisions remain available.'),optionRow('pool','The flock draws from','Use the existing species and proportions.',['Brazil','The Americas','The world']),switchRow('visitors','Rare visitors','Occasional visitors cross the sky.'),optionRow('species','Species','The production sprite renderer previews your selection.',A.SPECIES.map(s=>({value:s.id,label:s.name}))),optionRow('outfit',"Zeca's look",'Use the existing accessories.',[{value:'none',label:'None'},{value:'santa-hat',label:'Santa hat'},{value:'sunglasses',label:'Sunglasses'},{value:'headphones',label:'Headphones'}])),settingsGroup('Motion',switchRow('animations','Animations','Reduced motion from your system always takes priority.')));
      }else if(state.category==='agents'){
        content.append(el('p','pane-note','Each setup shows a diff before applying. Capability labels are sample states.'));
        A.AGENTS.filter(a=>a.setup!=='none'&&a.setup!=='remove-only').forEach(a=>content.append(el('article','content-row',null,el('header',null,null,el('h3',null,a.name),el('span','status '+(settings['agent-'+a.name]?'success':''),settings['agent-'+a.name]?'Connected in prototype':'Not configured')),el('p','pane-note','Watch · Answer · Drive'),button('Review setup','quiet-control',()=>demoSetup(a.name)))));
      }else if(state.category==='approvals'){
        settings.rules??=[{command:'npm run build',project:'Vults'}];
        content.append(el('p','pane-note','A rule applies only to the exact command and project you review.'));
        settings.rules.forEach(rule=>content.append(el('article','content-row',null,el('h3',null,rule.project),el('code','mono',rule.command),el('div','inline-actions',null,button('Revoke rule','quiet-control',()=>confirm('Revoke rule','Future matching requests ask again in the example.',()=>{settings.rules.splice(settings.rules.indexOf(rule),1);paint();remember();}))))));
        content.append(button('Add example rule','quiet-control',()=>{const command=field('Exact command'),project=field('Project','Vults');openEditor('Add example rule',el('div',null,null,command.element,project.element),()=>{if(!command.control.value.trim())throw new Error('Enter a command first.');settings.rules.push({command:command.control.value.trim(),project:project.control.value.trim()});paint();return true;});}),details('Policies',['Deny overrides allow.','Destructive actions always ask.','Every answer is audited.']));
      }else if(state.category==='projects'){
        ['Vults','frontend','docs'].forEach(project=>{const name='pin-'+project;settings[name]??=project==='Vults';content.append(settingsGroup(project,switchRow(name,'Pin this project','Keep this project near the start of the flock.'),switchRow('mute-'+project,'Mute this project','Keep decisions visible without attention sounds.')));});
      }else if(state.category==='activity'){
        content.append(settingsGroup('Local records',switchRow('history','Keep a history','Choose what stays on this computer.'),setting('Recaps and records','View your activity without duplicating it here.',button('Open Activity','quiet-control',()=>setArea('activity','room')))),settingsGroup('Maintenance',setting('Clear history','This removes only example records in this prototype.',button('Clear history…','quiet-control',()=>confirm('Clear history','Only the example conversation, journal and audit records are cleared.',()=>clearExampleHistory())))));
      }else if(state.category==='connectors'){
        content.append(settingsGroup('GitHub',switchRow('github','GitHub','The toggle affects only sample data. No service is contacted.'),setting('Connection status',settings.github?'Enabled in this prototype':'Off',button('Open Connections','quiet-control',()=>setArea('connections','room'))),setting('Polling budget','The real app records requests through its network gate.',el('span','counter','2 / 200'))));
      }else if(state.category==='phone'){
        content.append(el('p','pane-note','Pairing requires the Android companion and a configured relay in the real app.'),settingsGroup('Paired devices',setting('Example phone',settings.phone?'Paired in prototype':'Not paired',button(settings.phone?'Remove phone':'Show pairing example','quiet-control',()=>{
          if(settings.phone)confirm('Remove phone','Only the example phone is removed.',()=>{settings.phone=false;paint();remember();});
          else openEditor('Pairing example',el('div',null,null,el('p','pane-note','This code is illustrative and cannot pair a device.'),el('strong','counter','DEMO 123 456')),()=>{settings.phone=true;paint();return true;},'Pair example phone');
        }))));
      }else content.append(el('article','content-row',null,el('h3',null,'Vults'),el('p',null,'A friendly flock watching your coding agents. MIT licensed; no telemetry.'),el('p','pane-note','Interactive design prototype · sample data'),details('Local paths',['Settings: ~/.config/vults','Data: ~/.local/share/vults']),button('Review & releases','quiet-control',()=>setArea('review','room'))));
      enter(content,'gentle',4,1);
    }
  }
  const SCREEN_REVIEW=[
    {id:'chat',label:'Chat Compact',reference:'ChatComposer.tsx · ChatMessageBubbles.tsx · ModelSwitcherWithTokenGauge.tsx',scope:'Keep the approved Compact composition in one stable window across tabs. Validate bounded menus and internal scrolling.'},
    {id:'connections',label:'Connections',reference:'ThinkingAccordion.tsx · CdsButton.tsx · ConfirmationDialog.tsx',scope:'Keep the approved list and detail composition. Validate search, failed checks and explicit merge confirmation.'},
    {id:'tasks',label:'Tasks',reference:'CdsButton.tsx · ThinkingAccordion.tsx · ConfirmationDialog.tsx',scope:'Goal and next action first. Review creation, criteria, execution controls and separate removal actions.'},
    {id:'flock',label:'Flock',reference:'StatusIndicators.tsx · ContainerResponsiveBar.tsx',scope:'Preserve the birds and habitat. Review session selection, activity, changes and permission cards.'},
    {id:'activity',label:'Activity',reference:'SegmentedControl.tsx · ScrollFadeContainer.tsx',scope:'Review Week, Projects, History, Cost, Traces and Audit with useful empty and error states.'},
    {id:'memory',label:'Memory',reference:'ThinkingAccordion.tsx · ConfirmationDialog.tsx',scope:'Review proposals before saved facts. Keep accept, reject, remove and undo explicit.'},
    {id:'settings',label:'Settings',reference:'CdsButton.tsx · SegmentedControl.tsx · ConfirmationDialog.tsx',scope:'Keep full settings in Room. Align the ten existing categories and remove repeated navigation.'},
    {id:'motion',label:'Motion & continuity',reference:'ClaudeMotionPresets.ts · useClaudeScrollPinning.ts · useClaudeRovingFocus.ts',scope:'Review reduced motion, keyboard use, scroll position and draft continuity across layouts.'},
  ];
  const RELEASES=[
    {title:'Release A · approved foundations',areas:'Chat Compact · Connections · Tasks',body:'Apply the approved visual direction to existing components, then validate the real chat, model menu, criteria and connection actions.',gate:'User review of Tasks; production renderer checks; native Compact and Room validation.',scope:'Existing UI work · UX4–UX6'},
    {title:'Release B · complete workspaces',areas:'Flock · Activity · Memory · Settings',body:'Apply the reviewed hierarchy to the remaining existing views. Preserve the current assets, permissions, retention and network controls.',gate:'Screen-by-screen approval; empty/error states; keyboard and short-window acceptance.',scope:'Existing UI work · UX4–UX6'},
    {title:'Release C · native continuity',areas:'Mini · Compact · Room · Full',body:'Validate native geometry, input regions, zoom and saved transitions. A browser prototype cannot prove these behaviors.',gate:'Real Linux desktop matrix, multiple monitor scales, unchanged drafts and selections.',scope:'Native feature work · remaining UX6 and UX7'},
    {title:'Later releases · separate features',areas:'Agents workspace · Studio · Investigations',body:'Keep new workspaces under their own capability contracts. This visual review does not claim those future features are implemented.',gate:'Feature specification, capability review and a separate approved prototype.',scope:'Future work · UX8–UX10'},
  ];
  function renderReview(host) {
    host.append(pageHeading('Review & releases','Visual approval is separate from production implementation.',button('Export review','primary',()=>download('vults-ui-review.json',JSON.stringify({prototype:'vults-ui-motion-proposal',sample:true,date:new Date().toISOString(),layout:{preferredSizes:A.SURFACE_SIZES,roomTabs:['flock','chat','tasks','activity','connections'],compactSize:'fixed across tabs'},decisions,notes,releases:RELEASES,references:SCREEN_REVIEW},null,2)))));
    const body=el('div'),tabs=segment(['Screens','Release sequence'],value=>paint(value));host.append(tabs,body);paint('Screens');
    function paint(tab){body.replaceChildren();
      if(tab==='Release sequence'){
        body.append(el('p','pane-note','Release names are working groups, not published versions or dates. Existing roadmap dependencies remain in force.'));
        RELEASES.forEach(release=>body.append(el('article','release-step',null,el('h3',null,release.title),el('span','status',release.scope),el('p',null,release.areas),el('p',null,release.body),el('p','pane-note',release.gate))));return;
      }
      body.append(el('p','pane-note','Chat Compact and Connections keep the visual direction you approved. Other screens still need your review.'));
      const grid=el('div','review-grid');body.append(grid);
      SCREEN_REVIEW.forEach(screen=>{
        const row=key(el('article','review-row',null,el('h3',null,screen.label),el('span','status '+(decisions[screen.id]==='approved'?'success':''),decisionLabel(decisions[screen.id])),el('p',null,screen.scope)),screen.id);
        const textarea=el('textarea');textarea.value=notes[screen.id]??'';textarea.placeholder=L('What should change?');textarea.setAttribute('aria-label',phrase('Notes for {screen}',{screen:L(screen.label)}));textarea.addEventListener('input',()=>{notes[screen.id]=textarea.value.slice(0,800);remember();});
        const open=()=>{if(screen.id==='motion'){setArea('chat','room');showExample('thinking');}else setArea(screen.id,screen.id==='chat'?'quick':'room');};
        row.append(el('div','inline-actions',null,button('Open screen','quiet-control',open),button('Keep this direction','',()=>{decisions[screen.id]='approved';row.querySelector('.status').textContent=decisionLabel('approved');remember();}),button('Needs refinement','',()=>{decisions[screen.id]='refine';row.querySelector('.status').textContent=decisionLabel('refine');remember();})),textarea,details('Reference components',[screen.reference]));grid.append(row);
      });
    }
  }
  function decisionLabel(value){return L(value==='approved'?'Direction approved':value==='refine'?'Needs refinement':'Waiting for review');}
  function clearExampleHistory(){clearTimeout(replyTimer);state.busy=false;state.voice=false;state.messages=[];state.files=[];input.value='';input.hidden=false;$('.voice-preview').hidden=true;mic.setAttribute('aria-pressed','false');state.conversation='current';conversations.splice(0,conversations.length,{id:'current',title:'Current conversation',messages:[],draft:''});memory.pending=[];memory.facts=[];memory.history=[];auditRows.splice(0);settings.recordsCleared=true;paintFiles();paintConversation();paintRecent();remember();toast('Example history cleared');}
  function download(name,text,type='application/json'){
    if(typeof Blob==='undefined'||!URL.createObjectURL){toast('Export is not available in this viewer.');return;}
    const url=URL.createObjectURL(new Blob([text],{type})),a=el('a');a.href=url;a.download=name;document.body.append(a);a.click();a.remove();window.setTimeout(()=>URL.revokeObjectURL(url),1000);toast('Review exported');
  }
  function translateStatic(){for(const node of $$('[data-word]'))node.textContent=L(node.dataset.word);for(const node of $$('[data-label]'))node.setAttribute('aria-label',L(node.dataset.label));input.placeholder=L('Ask Zeca…');input.setAttribute('aria-label',L('Ask Zeca…'));}
  function paintNav() {
    const nav=$('.product-nav'),old=nav.querySelector('.nav-current')?.getBoundingClientRect(),focused=document.activeElement?.getAttribute('data-key');nav.replaceChildren();let selected;
    const items=state.mode==='quick'?[['flock','Flock','flock'],['chat','Chat','chat'],['bars','Overview','summary'],['gear','Settings','settings']]:[['flock','Flock','flock'],['chat','Chat','chat'],['tasks','Tasks','tasks'],['bars','Activity','activity'],['plug','Connections','connections'],['gear','Settings','settings']];
    items.forEach(([glyph,label,target])=>{
      const nextMode=state.mode==='quick'&&['flock','chat','summary'].includes(target)?'quick':state.mode==='full'?'full':'room';
      const name=state.mode==='quick'&&target==='settings'?'Open settings in Room':label;
      const b=key(button(null,'',()=>setArea(target,nextMode),name),'nav-'+target);b.append(A.icon(glyph,15),el('span','nav-label',label));b.disabled=target==='chat'&&!settings.zeca;b.setAttribute('aria-pressed',String(target===state.area));
      if(target===state.area){b.classList.add('selected-area');selected=el('span','nav-current');selected.setAttribute('aria-hidden','true');b.prepend(selected);}nav.append(b);
      if(focused==='nav-'+target&&!restoring)b.focus({preventScroll:true});
    });
    if(old&&selected&&prefs.motion&&!restoring){const next=selected.getBoundingClientRect();selected.animate([{transform:'translateX('+(old.left-next.left)+'px)'},{transform:'translateX(0)'}],{duration:180,easing:'cubic-bezier(0.16,1,0.3,1)'});}
  }
  function paintChrome(){const sound=$('.product-head-actions button');sound.replaceChildren(A.icon(settings.sounds?'sound':'mute',15));sound.setAttribute('aria-pressed',String(settings.sounds));const mark=$('.room-footer span');if(mark)mark.textContent=L(settings.zeca?'Zeca is on':'Zeca is off');}
  function setMode(mode){
    if(!['mini','quick','room','full'].includes(mode))return;
    const area=mode==='mini'?'flock':state.mode==='mini'?(mode==='quick'?views.quickArea:views.roomArea):mode==='quick'&&!['chat','flock','summary'].includes(state.area)?(['activity','connections'].includes(state.area)?'summary':views.quickArea):state.area;
    setArea(area,mode);
  }
  function setArea(area,mode=state.mode) {
    if(!AREAS.includes(area))return;
    captureLabPosition();captureLabPosition=()=>{};
    if(mode==='quick'&&['activity','connections'].includes(area))area='summary';
    if(area==='summary'&&['room','full'].includes(mode))area='activity';
    if(!settings.zeca&&area==='chat')area='flock';
    if(!['mini','quick','room','full'].includes(mode))mode='room';
    if(['settings','tasks','memory','review'].includes(area)&&['mini','quick'].includes(mode))mode='room';
    const previous=state.area,previousMode=state.mode;if(previous==='chat'&&previousMode!=='mini')views.chatScroll=log.scrollTop;
    if(previousMode==='quick')views.quickArea=previous;else if(['room','full'].includes(previousMode))views.roomArea=previous;
    state.area=area;state.mode=mode;closeModels(false,false);closeMenu(history,historyTrigger,false,false);stopScenes();
    if(area!=='flock'&&['approval','destructive','question','expired','many','read','edit','run','done','rest','fail','think'].includes(state.scenario))state.scenario='ready';
    $$('[data-flock-example]').forEach(option=>option.hidden=area!=='flock');
    const mini=mode==='mini',room=['room','full'].includes(mode),chat=area==='chat';
    windowEl.hidden=mini;$('.mini-strip').hidden=!mini;windowEl.classList.toggle('workspace',room);windowEl.classList.toggle('full',mode==='full');windowEl.classList.toggle('review-mode',area==='review');
    $('.chat-shell').hidden=!chat||mini;chatEl.hidden=!chat||mini;$('.workspace-view').hidden=chat||mini;$('.room-recent').hidden=!room;$('.room-footer').hidden=!room||area==='review';
    $('.review-states').hidden=!chat||mini;$('.prototype-controls').hidden=false;$('.scenario-select').disabled=mini;$('.scenario-select').value=state.scenario;
    $('.surface-name').textContent=area==='review'?L('Design review'):mode==='quick'?'Compact':mode==='full'?L('Full'):L('Room');
    $('.product-size').hidden=area==='review';$('.return-compact').hidden=!room||area==='review';$('.product-nav').hidden=area==='review';
    windowEl.setAttribute('aria-label',area==='review'?'Vults design review':'Vults '+(mode==='quick'?'Compact':mode==='full'?'Full':'Room'));
    $$('[data-area]').forEach(b=>b.setAttribute('aria-pressed',String(b.dataset.area===area&&(!b.dataset.mode||b.dataset.mode===mode))));
    $$('[data-size]').forEach(b=>{b.classList.toggle('selected',b.dataset.size===mode);b.setAttribute('aria-pressed',String(b.dataset.size===mode));});
    layoutSurface(!restoring&&previousMode!==mode);
    const workspace=$('.workspace-view');workspace.replaceChildren();workspace.classList.remove('compact-flock','compact-summary','lab-flock');paintNav();
    if(mini){$('.mini-strip').replaceChildren();mountFlock($('.mini-strip'),true);}
    else if(chat){
      paintRecent();paintConversation();log.scrollTop=views.chatScroll;mic.hidden=!settings.voice;
      if(state.scenario==='error')log.prepend(el('div','pane-note state-error','Could not update this view.'),button('Try again','quiet-control',()=>{state.scenario='ready';setArea('chat',state.mode);}));
    }
    else if(mode==='quick'&&area==='flock')renderCompactFlock(workspace);
    else if(mode==='quick'&&area==='summary')renderCompactSummary(workspace);
    else ({flock:renderFlock,tasks:renderTasks,activity:renderActivity,connections:renderConnections,settings:renderSettings,memory:renderMemory,review:renderReview})[area]?.(workspace);
    const foot=$('.room-footer');foot.replaceChildren(el('span',null,settings.zeca?'Zeca is on':'Zeca is off'),el('span',null,'Sample data'),button('Back to Compact','',()=>setMode('quick')));
    $('.access-trigger').textContent=L(settings.access);$('.folder-trigger').textContent=settings.chatFolder??'Vults';
    paintChrome();
    if(!chat)input.hidden=state.voice?true:false;
    paintBird();paintSuggestions();translateStatic();if(previous!==area&&!restoring)enter(chat?chatEl:workspace,'gentle',4,1);remember();
  }
  function setScenario(value){
    state.scenario=value;
    if(state.area==='flock'){
      views.flockDetail='session';
      sessions.splice(5);const s=sessions[0],pose={read:['working','read','docs/guide.md'],edit:['working','edit','ui/src/chat.ts'],run:['working','run','npm run build'],done:['finished',null,'npm run build'],rest:['idle',null,'npm run build'],fail:['failed','run','npm run build'],think:['thinking','think','Review the layout']}[value];
      s.step=pose?pose[2]:value==='destructive'?'git clean -fd':'npm run build';s.status=pose?pose[0]:value==='question'?'question':value==='expired'?'idle':'approval';s.activity=pose?pose[1]:'run';s.card=s.waiting=['approval','question'].includes(s.status);s.steps=['Read project context',s.step];views.sessionSelected=s.id;
      if(value==='many')for(let i=0;i<10;i++)sessions.push({...sessions[i%5],id:'extra-'+i,project:'project-'+(i+1),status:i%3===0?'working':'idle',card:false,waiting:false});
      A.assignSpecies(sessions);
    }
    if(state.area==='chat'){
      if(value==='empty')showExample('empty');
      else if(value==='loading')showExample('thinking');
    }
    setArea(state.area,state.mode);
  }
  /* COMPACT_LOGIC */
  /* LAB_LOGIC */
  const media=matchMedia('(prefers-reduced-motion: reduce)');media.addEventListener?.('change',()=>{prefs.motion=settings.animations&&!media.matches;root.classList.toggle('motion-off',!prefs.motion);if(!prefs.motion)settleMotion();paintBird();for(const scene of activeScenes)scene.setActive(prefs.motion&&!document.hidden);});
  root.addEventListener('keydown',event=>{
    if(event.defaultPrevented||event.isComposing||event.keyCode===229)return;
    if(!$('.editor-overlay').hidden||!$('.dialog-overlay').hidden)return;
    if((event.ctrlKey||event.metaKey)&&event.key.toLowerCase()==='k'){event.preventDefault();openPalette();return;}
    if(event.altKey&&['1','2','3','4','5'].includes(event.key)){event.preventDefault();setArea(['flock','chat','tasks','activity','connections'][Number(event.key)-1],'room');}
  });
