  const settings = { theme:'jaragua', zeca:true, sounds:false, volume:50, notifications:true, presence:'Island', fold:15, hover:false, autostart:false, media:false, usage:true, history:true, memory:true, summary:false, suggestions:true, sleep:false, embeddings:false, voice:true, voiceModel:'Small', voiceLanguage:'Automatic', access:'Asks first', native:true, web:false, webEngine:'Brave', github:true, visitors:true, animations:prefs.motion, species:'atratus', outfit:'auto', pool:'Brazil', language:'pt-BR', dailyBudget:10, phone:false, transport:'Relay', provider:'CLI', chatFolder:'Vults', recordsCleared:false, dnd:'Off', screen:'Follow the pointer', widget:'Off',
    dictMode:'Hold + double tap', dictSpace:true, dictSession:true, dictAutoSend:false, dictSpokenCode:true, dictAgentsAsk:false, dictMic:'System default', dictSound:true, dictRepeats:true, dictPlanB:true, dictTerminals:true, dictClipboard:true, dictCleanup:false, dictCleanupModel:'Chat & voice · claude-haiku-4-5', dictStyle:'Natural', dictWriteIn:'The spoken language', dictSessionsEnglish:false, dictSummary:false, dictKeep:'30', dictHint:true, dictProject:true, dictSayCancel:true, dictDuck:'Lower to 25%', dictCaption:true, dictCommands:true, dictCommandElse:'Say so on the island',
    zecaVoice:false, zecaVoiceModel:'OpenAI · gpt-realtime', zecaBudget:'$1', zecaTurn:'Balanced', zecaBarge:true, zecaFollowUp:false,
    rules:[{agent:'Codex',tool:'Bash',target:'npm run build',cwd:'~/Projects/Vults'},{agent:'Claude Code',tool:'Read',target:'docs/**',cwd:'~/Projects/docs'}],
    policies:[{id:'global',scope:'Every folder',text:'deny command "git push --force*"\nask class network\nallow tool Read'},{id:'vults',scope:'~/Projects/Vults',text:'allow command "npm run build"\nallow command "cargo test*"\nask class write outside "src/**"'}],
    keys:{anthropic:true,openrouter:true,openai:false}, githubSeen:{} };
  const views = { detailOpen:{}, listScroll:{}, taskQuery:'', taskFilter:'Open', taskSelected:'task-2', taskDetail:'run', panels:{}, skipped:[], showFinished:false, connectionQuery:'', connectionSection:'pulls', connectionFace:'you', connectionSelected:'#364', sessionQuery:'', sessionFilter:'All sessions', sessionGroup:'state', sessionSelected:'session-1', flockDetail:'session', flockStep:0, flockFile:'all', recordQuery:'', week:0, day:3, memoryTab:'Waiting for you', recentQuery:'', chatScroll:0, quickArea:'chat', roomArea:'flock' };
  // Sessions you opened in a terminal: the flock watches them.
  const sessions = [
    {id:'session-1',agent:'codex',project:'Vults',status:'approval',activity:'run',why:'Build the project to check the Now inbox before the review.',step:'npm run build',step_count:12,subagents:0,branch:'feat/compact-now',cost:0.42,steps:['Read docs/dev/plan-ui.md','Edit ui/src/island/now.ts','Edit ui/src/island/render.ts','Run npm run build']},
    {id:'session-2',agent:'claude',project:'frontend',status:'working',activity:'edit',step:'ui/src/chat.ts',step_count:8,subagents:1,branch:'fix/chat-scroll',cost:1.10,steps:['Read ui/src/chat.ts','Search "scrollTop"','Subagent: check the scroll tests','Edit ui/src/chat.ts']},
    {id:'session-3',agent:'codex',project:'docs',status:'finished',activity:null,step:'docs/guide/flock.md',step_count:6,subagents:0,branch:'docs/flock-badges',cost:0.18,rest:24,steps:['Read docs/guide/flock.md','Edit docs/guide/flock.md','Edit docs/pt-br/guide/flock.md']},
    {id:'session-4',agent:'opencode',project:'api',status:'failed',activity:'run',step:'cargo test -p api',step_count:4,subagents:0,branch:'fix/retry-flake',cost:null,steps:['Read crates/api/src/retry.rs','Edit crates/api/src/retry.rs','Run cargo test -p api'],output:'test retry::backs_off ... FAILED\nassertion failed: elapsed >= 200ms (was 140ms)'},
    {id:'session-5',agent:'claude',project:'design',status:'thinking',activity:'think',step:'Compare the Lab layouts',step_count:3,subagents:0,branch:'main',cost:0.31,steps:['Read design/ui-motion-proposal/scope.json','Think about the Lab layout']},
    {id:'session-6',agent:'qwen',project:'infra',status:'working',activity:null,step:'Waiting for input',step_count:2,subagents:0,branch:'chore/ci-cache',cost:null,quiet:16,steps:['Read .github/workflows/ci.yml','Waiting for input']},
  ].map(s => ({...s,cwd:'~/Projects/'+s.project,attention:s.status==='approval'?'needs-you':'quiet',card:s.status==='approval',waiting:s.status==='approval',raise:true,note:null,editor:s.agent==='claude'?'kitty':'tmux',bird_chosen:false,roost:s.project,started:'Watched in a terminal',pending:s.status==='approval'?2:0,restSince:s.rest?Date.now()-s.rest*1000:null}));
  A.assignSpecies(sessions);
  const github = {
    synced:2, requests:41, budget:3000, refreshedAt:0,
    inbox:[
      {id:'inbox-1',kind:'Review requested',title:'Pin the flock order',repo:'frontend',ref:'#358',author:'ana-lima',age:'1 h',isNew:true},
      {id:'inbox-2',kind:'Mentioned',title:'Badge shapes in the docs',repo:'docs',ref:'#91',author:'joao-p',age:'3 h',isNew:true},
    ],
    pulls:[
      {id:'#364',title:'Show the Now inbox in Compact',repo:'Vults',branch:'feat/compact-now',author:'you',size:'+412 −96',checks:'passed',checkList:[['TypeScript','passed'],['Rust','passed'],['UI integration','passed']],merged:false,isNew:false},
      {id:'#363',title:'Keep the chat scroll while a reply streams',repo:'frontend',branch:'fix/chat-scroll',author:'you',size:'+58 −21',checks:'running',checkList:[['TypeScript','passed'],['Scroll tests','running']],merged:false,isNew:true},
      {id:'#361',title:'Fix the flaky retry test',repo:'api',branch:'fix/retry-flake',author:'you',size:'+12 −4',checks:'failed',checkList:[['cargo test','failed'],['clippy','passed']],merged:false,isNew:true},
    ],
    actions:[
      {id:'act-1',title:'main',repo:'Vults',checks:'passed',checkList:[['CI','passed'],['Docs','passed']],age:'20 min'},
      {id:'act-2',title:'main',repo:'frontend',checks:'failed',checkList:[['e2e','failed'],['lint','passed']],age:'45 min',isNew:true},
      {id:'act-3',title:'main',repo:'docs',checks:'passed',checkList:[['Build','passed']],age:'2 h'},
    ],
    releases:[
      {id:'rel-1',title:'v0.1.4',repo:'Vults',age:'2 days',notes:'Saved Nest close, model list retry, wider Quick chat.'},
      {id:'rel-2',title:'v2.3.0',repo:'frontend',age:'5 days',notes:'Streaming replies keep their place.'},
    ],
  };
  // What Zeca may read or do with each connector: its own switch, budget and trace (rule 4).
  const zecaTools = [
    {id:'snapshot',name:'github_snapshot',kind:'Read',about:'Your PRs, their checks and review requests, from the last sync.',on:true,used:'10:42',calls:6},
    {id:'issue',name:'open_issue',kind:'Act',about:'Opens an issue for a task. Asks you first, every time.',on:true,used:'Yesterday',calls:1},
    {id:'merge',name:'merge_pull_request',kind:'Act',about:'Squash and merge a PR with green checks. Always asks you.',on:false,used:'Never',calls:0},
  ];
  const memory = {
    pending:[
      {id:'proposal-1',place:'Vults',text:'Run npm run build before asking for a review.',current:null},
      {id:'proposal-2',place:'About you',text:'Prefers replies in Brazilian Portuguese.',current:null},
      {id:'proposal-3',place:'frontend',text:'Tests run on Vitest.',current:'Tests run on Jest.'},
    ],
    facts:[{id:'fact-1',place:'Vults',text:'Merge with a merge commit, never squash.'},{id:'fact-2',place:'About you',text:'Reviews every PR before merging.'}],
    files:[['about-you.md',34],['vults.md',61],['frontend.md',22]],
    history:[], debrief:'Overnight, Zeca merged two notes about the Vults build and proposes one change.' };
  const decisions = { chat:'approved', connections:'approved', island:'pending', flock:'pending', now:'pending', activity:'pending', settings:'pending', motion:'pending' };
  const notes = {};
  const conversations = [{id:'current',title:'Current conversation',messages:[],draft:''},{id:'recent',title:'What changed recently?',messages:[{who:'you',text:'What changed recently?'},SAMPLE_REPLY()],draft:''},{id:'older',title:'Why did the api tests fail?',messages:[],draft:''}];
  const auditRows=[
    {time:'10:42',project:'Vults',agent:'Codex',answer:'Allowed',by:'You',command:'npm run build'},
    {time:'10:31',project:'frontend',agent:'Claude Code',answer:'Denied',by:'You',command:'git push --force origin main'},
    {time:'10:12',project:'docs',agent:'Claude Code',answer:'Allowed',by:'Always rule',command:'Read docs/guide/flock.md'},
    {time:'09:58',project:'Vults',agent:'Codex',answer:'Allowed',by:'Phone',command:'cargo test -p vults-core'},
    {time:'09:40',project:'infra',agent:'Qwen Code',answer:'Asked',by:'Policy',command:'curl https://ci.example.dev/status'},
  ];
  let taskDraft={title:'',repo:'Vults',criteria:'',engine:'Codex'}, activeScenes=[], flockObserver, flockResize=() => {}, lastManualScroll=0;
  // Keep each surface's director alive so navigation does not replay arrivals.
  const flockScenes=new Map();
  let surfaceAnimation,contentAnimation,lastSurface;
  const surface=$('.surface-shell');
  const MODE_NAME={mini:'Island',quick:'Compact',room:'Lab',full:'Lab'};
  function layoutSurface(animate=false) {
    const stage=$('.review-stage'),box=stage.getBoundingClientRect(),preview=$('.viewport-select').value;
    const availableWidth=Math.max(0,(stage.clientWidth||box.width||1168)-24);
    const availableHeight=preview==='short'?360:Math.max(280,(window.innerHeight||1000)-Math.max(0,box.top)-20);
    const next=A.fitSurface(state.mode,preview==='narrow'?Math.min(480,availableWidth):availableWidth,availableHeight);
    const singlePane=next.width<=760||next.height<460;
    if(windowEl.classList.contains('single-pane')!==singlePane)captureLabPosition();
    const measured=surface.getBoundingClientRect(),previous={width:measured.width,height:measured.height,radius:Number.parseFloat(window.getComputedStyle?.(surface).borderTopLeftRadius)||lastSurface?.radius||next.radius};
    surfaceAnimation?.cancel();contentAnimation?.cancel();
    for(const name of ['width','height'])surface.style[name]=next[name]+'px';surface.style.borderRadius=next.radius+'px';surface.setAttribute('data-mode',state.mode);
    windowEl.style.width=Math.max(0,next.width-2)+'px';windowEl.style.height=Math.max(0,next.height-2)+'px';windowEl.style.left='50%';windowEl.style.transform='translateX(-50%)';
    windowEl.classList.toggle('tight',next.height<(state.mode==='quick'?380:460));windowEl.classList.toggle('very-short',next.height<360);surface.setAttribute('data-dimensions',next.width+'x'+next.height);
    if(animate&&lastSurface&&prefs.motion&&surface.animate&&(Math.abs(previous.width-next.width)>1||Math.abs(previous.height-next.height)>1)){
      surfaceAnimation=surface.animate(A.surfaceFrames(previous,next),{duration:200,easing:'cubic-bezier(0.16,1,0.3,1)',fill:'none'});
      const content=state.mode==='mini'?$('.mini-strip'):windowEl;
      contentAnimation=content.animate([{opacity:.65},{opacity:1}],{duration:160,fill:'none'});
    }
    windowEl.classList.toggle('single-pane',singlePane);
    lastSurface=next;positionOpenMenus();resizeFlock();
  }
  const menuAnimations=new Map();
  function positionMenu(panel,trigger) {
    if(panel.hidden)return;
    const rect=windowEl.getBoundingClientRect(),anchor=trigger.getBoundingClientRect();
    panel.style.maxHeight='312px';
    const placement=A.placePopover(rect,anchor,panel.scrollHeight,$('.product-nav').getBoundingClientRect().bottom,state.mode!=='quick');
    // A small menu stays small, even in a wide surface.
    if(panel===actionMenu&&placement.width>320){placement.left+=Math.max(0,Math.min(placement.width-320,anchor.left-rect.left-placement.left-160));placement.width=320;}
    for(const name of ['left','top','width','maxHeight'])panel.style[name]=placement[name]+'px';panel.style.transformOrigin=placement.origin;
  }
  let menuOwner=null;
  function positionOpenMenus(){positionMenu(popover,modelTrigger);positionMenu(history,historyTrigger);positionMenu(commands,input);if(menuOwner)positionMenu(actionMenu,menuOwner);}
  function openMenu(panel,trigger){
    menuAnimations.get(panel)?.cancel();panel.setAttribute('data-open','true');panel.hidden=false;panel.inert=false;trigger.setAttribute('aria-expanded','true');positionMenu(panel,trigger);enter(panel,'snappy',4,.99);
  }
  function closeMenu(panel,trigger,returnFocus=false,animate=true){
    menuAnimations.get(panel)?.cancel();panel.setAttribute('data-open','false');trigger?.setAttribute('aria-expanded','false');panel.inert=true;
    if(!panel.hidden&&prefs.motion&&animate&&panel.animate){const exit=panel.animate([{opacity:1,transform:'none'},{opacity:0,transform:'translateY(4px) scale(.99)'}],{duration:120,easing:'cubic-bezier(0.16,1,0.3,1)'});menuAnimations.set(panel,exit);exit.finished.then(()=>{if(panel.getAttribute('data-open')==='false')panel.hidden=true;}).catch(()=>{});}
    else panel.hidden=true;
    if(returnFocus)trigger?.focus({preventScroll:true});
  }
  // One small menu for every "⋯" (quick actions, quick settings): it never answers a card.
  function openActionMenu(trigger,title,rows){
    if(menuOwner===trigger&&!actionMenu.hidden){closeActionMenu(true);return;}
    closeActionMenu(false);menuOwner=trigger;
    actionMenu.replaceChildren(el('div','model-popover-head',null,el('strong',null,title),iconButton('close','Close',()=>closeActionMenu(true))));
    for(const row of rows){
      if(row.group){actionMenu.append(el('small','menu-group',row.group));continue;}
      if(row.control){actionMenu.append(el('div','menu-control',null,el('span',null,row.label),row.control));continue;}
      const b=key(button(null,'menu-item',()=>{closeActionMenu(false);row.action();}),row.key??String(row.label).toLowerCase().replace(/[^a-z0-9]+/g,'-'));b.disabled=!!row.disabled;b.append(A.icon(row.icon??'chevron',14),el('span',null,row.label),...(row.hint?[el('small',null,row.hint)]:[]));actionMenu.append(b);
    }
    actionMenu.setAttribute('aria-label',L(title));openMenu(actionMenu,trigger);actionMenu.querySelector('.menu-item,.menu-control select')?.focus({preventScroll:true});
  }
  function closeActionMenu(returnFocus=false){if(actionMenu.hidden&&!menuOwner)return;const owner=menuOwner;menuOwner=null;closeMenu(actionMenu,owner,returnFocus,false);}
  function settleMotion(){for(const animation of surface.getAnimations?.({subtree:true})??[])animation.cancel();surfaceAnimation?.cancel();contentAnimation?.cancel();noticeAnimation?.cancel();$('.notice-slot').hidden=!state.notice;for(const panel of [popover,history,commands,actionMenu]){menuAnimations.get(panel)?.cancel();panel.hidden=panel.getAttribute('data-open')!=='true';}}
  window.addEventListener('resize',()=>{layoutSurface(false);resizeCompactFlock();});
  const key = (node,value) => {node.setAttribute('data-key',value);return node;};
  const labelFor = agent => raw(({claude:'Claude Code',codex:'Codex',opencode:'OpenCode',qwen:'Qwen Code',zeca:'Zeca'})[agent]??agent);
  const statusFor = s => ({approval:'Needs you',question:'Needs you',working:'Working',thinking:'Thinking…',finished:'Finished',failed:'Failed',idle:'Idle'})[s]??s;
  // One status vocabulary across the app; the shapes match the island's badges.
  const toneFor = s => s.quiet?'quiet':['approval','question'].includes(s.status)?'needs':s.status==='failed'?'failed':s.status==='finished'?'done':['working','thinking'].includes(s.status)?'working':'quiet';
  function chip(tone,label,shape=tone){const c=el('span','status tone-'+tone,null,el('span','shape shape-'+shape),el('span',null,label));return c;}
  const sessionChip = s => chip(toneFor(s),s.quiet?phrase('Quiet {n} min',{n:s.quiet}):statusFor(s.status),s.status==='question'?'question':toneFor(s));
  // A filled sentence is already translated; mark it so a second L() does not count it as missing.
  const phrase = (text,vars) => {let out=L(text);for(const [name,value] of Object.entries(vars))out=out.replaceAll('{'+name+'}',String(value));return raw(out);};
  function snapshot() {
    const data={settings,views,decisions,notes,mode:state.mode,category:state.category,activity:state.activity,conversation:state.conversation,draft:input.value.slice(0,2000),files:state.files,messages:state.messages.filter(m=>m.who!=='card').slice(-6).map(m => ({who:m.who,text:String(m.text).slice(0,500)})),memory:{pending:memory.pending,facts:memory.facts,history:memory.history},taskDraft};
    return JSON.stringify(data).length<14000?data:{settings,views,decisions,notes,mode:state.mode,draft:data.draft,category:state.category,activity:state.activity};
  }
  function restorePrototype(saved) {
    const data=saved?.privateContent, nav=saved?.modelContent;
    if (nav && AREAS.includes(nav.area)) state.area=nav.area;
    if (nav?.mode && ['mini','quick','room','full'].includes(nav.mode)) state.mode=nav.mode;
    if (nav?.engine && ENGINES.some(e=>e.id===nav.engine)) state.engine=nav.engine;
    if (typeof nav?.model==='string') state.model=nav.model;
    if (!data || typeof data!=='object') return;
    if (data.settings && typeof data.settings==='object') for(const name of Object.keys(settings)) if (typeof data.settings[name]===typeof settings[name]&&!Array.isArray(settings[name])) settings[name]=data.settings[name];
    if (Array.isArray(data.settings?.rules)) settings.rules=data.settings.rules.filter(r=>typeof r?.target==='string').slice(0,20);
    if (data.settings) for(const [name,value] of Object.entries(data.settings)) if((name.startsWith('agent-')||name.startsWith('pin-')||name.startsWith('mute-')||name.startsWith('hide-'))&&typeof value==='boolean')settings[name]=value;
    if (data.views && typeof data.views==='object') for(const name of Object.keys(views)) if (typeof data.views[name]===typeof views[name]) views[name]=data.views[name];
    // Old Compact routes (activity, connections, settings) open the Now inbox.
    views.quickArea=['flock','chat','summary'].includes(views.quickArea)?views.quickArea:'summary';
    if (!['you','zeca'].includes(views.connectionFace)) views.connectionFace='you';
    if (typeof data.draft==='string') { input.value=data.draft; fitInput(); }
    if (Array.isArray(data.files)) state.files=data.files.filter(name=>typeof name==='string').slice(0,4);
    if (Array.isArray(data.messages)) state.messages=data.messages.filter(m => ['you','zeca'].includes(m.who)&&typeof m.text==='string').slice(-8);
    if (data.decisions) for (const area of Object.keys(decisions)) if (['approved','pending','refine'].includes(data.decisions[area])) decisions[area]=data.decisions[area];
    if (data.notes) for(const [area,note] of Object.entries(data.notes)) if(typeof note==='string' && area in decisions) notes[area]=note.slice(0,800);
    if (data.memory && ['pending','facts','history'].every(name=>Array.isArray(data.memory[name]))) Object.assign(memory,data.memory);
    if (data.taskDraft && typeof data.taskDraft.title==='string') taskDraft={...taskDraft,...data.taskDraft};
    if (CATEGORIES.some(([id])=>id===data.category)) state.category=data.category;
    if (['week','projects','history','cost','traces','audit'].includes(data.activity)) state.activity=data.activity;
    applyTheme(settings.theme);
    language=LANGUAGES.includes(settings.language)?settings.language:'pt-BR';A.setLang(language);$('.language-select').value=language;
    prefs.motion=settings.animations&&!matchMedia('(prefers-reduced-motion: reduce)').matches;
    A.setZeca(settings.zeca);A.setZecaSpecies(settings.species);A.setZecaLook(['none','auto'].includes(settings.outfit)?null:settings.outfit);
    root.classList.toggle('motion-off',!prefs.motion);
  }
  log.addEventListener('wheel',() => lastManualScroll=Date.now(),{passive:true});
  log.addEventListener('touchstart',() => lastManualScroll=Date.now(),{passive:true});
  log.addEventListener('pointerdown',() => lastManualScroll=Date.now());
  function flockVisible(){return !document.hidden&&!(state.area==='flock'&&['room','full'].includes(state.mode)&&windowEl.classList.contains('single-pane'));}
  function resizeFlock(){flockResize();}
  function stopScenes() {for(const scene of activeScenes) scene.setActive(false);activeScenes=[];flockObserver?.disconnect();flockResize=() => {};}
  function mountFlock(host,kind='lab',source=sessions,onLayout) {
    const base=kind==='mini'?A.COMPACT_SCENE:A.HABITAT_SCENE;
    const layout={...base,flights:base.flights&&prefs.motion},cacheKey=kind+'-'+prefs.motion;
    let scene=flockScenes.get(cacheKey);
    if(!scene){scene=new A.Scene(layout,['mini','lab'].includes(kind)?skyOwns(kind):undefined);flockScenes.set(cacheKey,scene);}
    activeScenes.push(scene);host.append(scene.canvas);scene.canvas.setAttribute('role','img');scene.canvas.setAttribute('aria-label',L('Live session birds'));
    const update=() => {
      const width=Math.floor(host.getBoundingClientRect().width||layout.width);
      const scale=layout.vults.scale, room=kind==='mini'?8:Math.max(1,Math.floor(width/(scale*32)));
      const shown=source.slice(0,Math.min(source.length,room)), gutter=kind==='mini'?48:0, available=width-gutter;
      scene.resize(width,shown.length,i=>({x:Math.round(gutter+(i+.5)*available/shown.length-scale*12),wire:layout.vults.at(0).wire}));
      scene.update(shown,null,kind==='mini'&&settings.zeca?{clip:'idle',agent:'codex'}:null);scene.setActive(flockVisible()&&(kind==='mini'||!windowEl.classList.contains('tight')));
      onLayout?.(scene,shown,width);
      still(scene);
    };
    update();flockResize=update;
    if (typeof ResizeObserver!=='undefined') {flockObserver=new ResizeObserver(update);flockObserver.observe(host);}
    return scene;
  }
  document.addEventListener('visibilitychange',()=> {for(const scene of activeScenes)scene.setActive(flockVisible()&&prefs.motion);});
  // Empty chat: what Zeca suggests depends on the flock. A suggestion fills the draft, it never sends.
  function paintSuggestions() {
    const suggestions=$('.room-suggestions');suggestions.replaceChildren();
    suggestions.hidden=state.mode==='mini'||state.messages.length>0||state.busy||!settings.zeca;
    if(suggestions.hidden)return;
    const room=state.mode!=='quick';
    if(room)suggestions.append(el('p','suggest-title','Ask about your agents, your projects or anything on your desktop.'));
    companionSuggestions().slice(0,room?4:2).forEach(c=>suggestions.append(key(button(c.label,'',()=>{if(c.go){c.go();return;}input.value=L(c.send);input.dispatchEvent(new Event('input'));input.focus();}),c.id)));
  }
  const EMPTY_HINTS={flock:'Start an agent in a terminal with the Vults hooks installed; its bird lands here.',summary:'Cards, failures, checks and reviews that ask for an action show up here.',connections:'Turn on GitHub in Settings to see your pull requests and checks.',activity:'Turns appear here as your agents work. History is kept on this computer.',dictation:'Hold Ctrl+Alt+V in any app and speak. What you dictate shows here, only on this computer.',chat:'Ask Zeca about your agents, your projects or your desktop.'};
  function stageState(host,emptyTitle,emptyAction) {
    if(!['empty','loading','error'].includes(state.scenario))return false;
    const panel=el('section','empty-state');panel.setAttribute('role',state.scenario==='error'?'alert':'status');
    if(state.scenario==='loading')panel.append(el('h3',null,'Loading'),el('div','loading-line'),el('div','loading-line'));
    else if(state.scenario==='error')panel.append(A.icon('retry',20),el('h3','state-error','Could not update this view.'),el('p',null,'Your draft and selections stay here.'),details('Technical details',['vults-store: database is locked (retrying in 2 s)']),button('Try again','quiet-control',()=> {state.scenario='ready';setArea(state.area,state.mode);}));
    else panel.append(A.icon(({flock:'flock',connections:'plug',activity:'bars',chat:'chat',dictation:'mic'})[state.area]??'list',22),el('h3',null,emptyTitle),el('p',null,EMPTY_HINTS[state.area]??'Nothing here yet.'),emptyAction??button('Show sample data','quiet-control',()=> {state.scenario='ready';setArea(state.area,state.mode);}));
    host.append(panel);return true;
  }
  function openEditor(title,content,onSave,saveLabel='Save') {
    const overlay=$('.editor-overlay'), dialog=$('.editor-dialog'), form=dialog.querySelector('form'), previous=document.activeElement;
    dialog.querySelector('h2').textContent=L(title);$('.editor-content').replaceChildren(content);$('.editor-error').hidden=true;
    const close=()=> {overlay.hidden=true;windowEl.querySelector('.product-nav').inert=false;$('.chat-shell').inert=false;$('.workspace-view').inert=false;previous?.isConnected?previous.focus():$('.product-nav button')?.focus();};
    const cancel=button(onSave?'Cancel':'Close','quiet-control',close), save=button(saveLabel,'primary',()=>{});save.type='submit';
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
  /* Conversations: saved locally, newest first; rename and delete from the list. */
  function saveConversation() {const current=conversations.find(c=>c.id===state.conversation);if(current){current.messages=state.messages.map(m=>({...m}));current.draft=input.value;current.engine=state.engine;current.model=state.model;}}
  function openConversation(id) {
    if(state.busy){toast('Stop the current turn first.');return;}
    saveConversation();const next=conversations.find(c=>c.id===id);if(!next)return;
    state.conversation=id;state.messages=next.messages.map(m=>({...m}));input.value=next.draft;fitInput();state.engine=next.engine??'claude';state.model=next.model??'Default model';paintConversation();paintRecent();remember();
  }
  function conversationRow(c,before=()=>{}) {
    const row=el('div','conversation-row');
    const open=button(c.title,'conversation-open',()=>{before();openConversation(c.id);});open.setAttribute('aria-pressed',String(c.id===state.conversation));
    const more=iconButton('pencil','Rename',()=>{const name=field('Name',L(c.title));openEditor('Rename conversation',name.element,()=>{if(!name.control.value.trim())throw new Error('Enter a name first.');c.title=raw(name.control.value.trim());paintRecent();if(!history.hidden)paintHistoryMenu();return true;});});
    const drop=iconButton('close','Delete',()=>confirm('Delete conversation','Only this conversation is deleted. Zeca\'s memory stays.',()=>{conversations.splice(conversations.indexOf(c),1);if(state.conversation===c.id){state.conversation=conversations[0]?.id??'current';if(!conversations.length)conversations.push({id:'current',title:'Current conversation',messages:[],draft:''});openConversation(state.conversation);}paintRecent();},'default','Delete'));
    row.append(open,el('span','row-tools',null,more,drop));return row;
  }
  function paintRecent() {
    const host=$('.room-recent');host.replaceChildren(el('div','recent-head',null,el('h3',null,'Conversations'),iconButton('plus','New conversation',()=>newConversation())));
    const finder=search('Search conversations',value=> {views.recentQuery=value;paintList();});finder.querySelector('input').value=views.recentQuery;host.append(finder);
    const list=el('div','recent-list');host.append(list);
    function paintList(){list.replaceChildren();const found=conversations.filter(c=>L(c.title).toLowerCase().includes(views.recentQuery.toLowerCase()));if(found.length)list.append(el('small','menu-group','Today'));found.forEach(c=>list.append(conversationRow(c)));if(!found.length)list.append(el('p','pane-note','Nothing here matches your search.'));}
    paintList();
    if(['room','full'].includes(state.mode))flexible($('.chat-shell'),host,'recent',{def:230,min:170,max:380,label:'Conversations',head:fold=>{const h=host.querySelector('.recent-head');h.append(fold);return h;},rail:()=>{const r=el('div','rail-marks');conversations.slice(0,6).forEach(c=>{const b=button(null,'rail-dot',()=>openConversation(c.id),c.title);b.setAttribute('data-tooltip',L(c.title));b.append(A.icon('chat',13));r.append(b);});return r;}});
  }
  function newConversation() {
    if(state.busy){toast('Stop the current turn first.');return;}
    saveConversation();const id='conversation-'+(conversations.length+1);conversations.unshift({id,title:'New conversation',messages:[],draft:''});state.conversation=id;input.value='';fitInput();showExample('empty');paintRecent();remember();
  }
  function prepareChat(text,mode) {setArea('chat',mode??(state.mode==='quick'?'quick':'room'));if(input.value.trim())input.value+='\n'+L(text);else input.value=L(text);input.dispatchEvent(new Event('input'));input.focus();}
  function openFolder(){
    const folders=[...new Set(sessions.map(s=>s.project))].map(p=>({value:p,label:p})).concat([{value:'Empty folder',label:'An empty folder'}]);
    const select=choose('Folder',folders,settings.chatFolder??'Vults',()=>{});
    openEditor('Folder for this conversation',el('div',null,null,el('label',null,null,el('span',null,'Folder'),select),el('p','pane-note','The front session\'s folder comes first. Zeca reads only what you share.')),()=>{settings.chatFolder=select.value;$('.folder-trigger').textContent=L(select.value);paintConversation();return true;});
  }
  // The chat chip can only lower access; raising it needs Settings.
  function openAccess(){
    const levels=['Asks first','Your policies','Full access'],at=levels.indexOf(settings.access);
    openActionMenu($('.access-trigger'),"Who answers Zeca's cards",[...levels.slice(0,at+1).map(level=>({label:level,icon:level===settings.access?'check':'person',action:()=>{settings.access=level;$('.access-trigger').textContent=L(level);remember();}})),{label:'Change in Settings',icon:'gear',action:()=>openSettings('chat')}]);
  }
  function askSession(project){prepareChat(phrase('What is {project} doing now?',{project}));}
  function openPalette() {
    const content=el('div','palette'), list=el('div','palette-list');let close;
    const rows=[
      ...sessions.map(s=>({group:'Sessions',label:s.project+' · '+labelFor(s.agent),hint:L(sessionBrief(s)),run:()=>{views.sessionSelected=s.id;views.detailOpen.flock=true;setArea('flock','room');}})),
      ...flockFacts().needs.map(s=>({group:'Cards',label:phrase('Review the card from {project}',{project:s.project}),hint:'Opens it; answering stays a click',run:()=>{views.sessionSelected=s.id;views.detailOpen.flock=true;views.flockDetail='session';setArea('flock','room');}})),
      ...COMMANDS.map(([name,about,insert])=>({group:'Commands',label:raw(name),hint:about,run:()=>prepareChat(insert)})),
      ...[['flock','Flock'],['chat','Chat'],['connections','Connections'],['activity','Activity'],['dictation','Dictation'],['memory','Memory'],['settings','Settings']].map(([area,label])=>({group:'Go to',label,hint:'Lab',run:()=>area==='settings'?openSettings():area==='memory'?openMemory():setArea(area,'room')})),
    ];
    const paint=query=> {list.replaceChildren();let group;rows.filter(row=>(L(row.label)+' '+L(row.hint)).toLowerCase().includes(query.toLowerCase())).slice(0,14).forEach(row=>{if(row.group!==group){group=row.group;list.append(el('small','menu-group',group));}const b=button(null,'menu-item',()=> {close();row.run();});b.append(el('span',null,row.label),el('small',null,row.hint));list.append(b);});};
    const query=field('Search sessions, cards, commands and places','','search',paint);content.append(query.element,list);paint('');close=openEditor('Search',content);
  }
  function openSettings(category){if(category)state.category=category;setArea('settings',state.mode==='full'?'full':'room');}
  function openMemory(){state.memoryPanel=true;views.memoryTab=memory.pending.length?'Waiting for you':'What Zeca remembers';setArea('chat',state.mode==='full'?'full':'room');roomForMemory();}
  // Opening Memory in a narrow Lab folds the conversation list once, so the chat keeps its width.
  function roomForMemory(){if(windowEl.getBoundingClientRect().width>=1100)return;if(typeof views.panels!=='object'||!views.panels)views.panels={};const recent=views.panels.recent??={w:230,folded:false};if(!recent.folded){recent.folded=true;paintRecent();}}
  function toggleMemoryPanel(force){state.memoryPanel=force??!state.memoryPanel;if(state.memoryPanel)roomForMemory();paintMemoryPanel();remember();}
  /* Every side panel in the Lab is flexible, the same way: a header with its name and a fold button that is
     always visible, a divider you drag (arrow keys work on it, double-click resets), and a narrow rail when
     folded, with the panel's own marks (`rail`) so it still says something. Widths are kept per panel. */
  function flexible(container,panel,id,{side='left',min=180,max=480,def=280,label='Panel',head=null,rail=null}={}){
    if(typeof views.panels!=='object'||!views.panels)views.panels={};
    const p=views.panels[id]??={w:def,folded:false};
    for(const old of [...panel.children])if(old.classList.contains('flex-handle')||old.classList.contains('flex-head')||old.classList.contains('flex-rail'))old.remove();
    panel.classList.add('flexible','flex-'+side);
    const fold=key(button(null,'icon-action flex-fold',()=>{p.folded=!p.folded;apply();remember();}),'fold-'+id);
    const header=head?.(fold)??el('div','flex-head',null,el('strong',null,label),fold);
    if(!head)panel.prepend(header);
    const railNode=el('div','flex-rail');
    const handle=el('div','flex-handle');handle.setAttribute('role','separator');handle.setAttribute('aria-orientation','vertical');handle.setAttribute('aria-label',phrase('Resize {panel}',{panel:L(label)}));handle.setAttribute('aria-valuemin',String(min));handle.setAttribute('aria-valuemax',String(max));handle.tabIndex=0;handle.setAttribute('data-tooltip',L('Drag to resize · double-click to reset'));
    panel.append(railNode,handle);
    function apply(){
      container.style.setProperty('--'+id+'-w',(p.folded?48:p.w)+'px');panel.classList.toggle('folded',p.folded);handle.setAttribute('aria-valuenow',String(p.w));
      const name=p.folded?phrase('Show {panel}',{panel:L(label)}):phrase('Fold {panel}',{panel:L(label)});fold.setAttribute('aria-label',name);fold.setAttribute('data-tooltip',name);
      fold.replaceChildren(A.icon(p.folded===(side==='left')?'sideRight':'sideLeft',15));
      railNode.replaceChildren();if(p.folded){railNode.append(fold);if(rail)railNode.append(rail());}else header.append(fold);
    }
    // The grid caps a list at 48% of the Lab; the drag stops at the same place.
    const resize=width=>{const cap=Math.min(max,Math.round((container.clientWidth||Infinity)*.48));p.w=Math.max(min,Math.min(cap,Math.round(width)));p.folded=false;apply();};
    handle.addEventListener('pointerdown',event=>{
      const start=event.clientX,from=p.w;handle.setPointerCapture?.(event.pointerId);handle.classList.add('dragging');document.body?.classList?.add('vp-resizing');
      const move=e=>resize(from+(e.clientX-start)*(side==='left'?1:-1));
      const up=()=>{handle.classList.remove('dragging');document.body?.classList?.remove('vp-resizing');handle.removeEventListener('pointermove',move);handle.removeEventListener('pointerup',up);handle.removeEventListener('pointercancel',up);remember();};
      handle.addEventListener('pointermove',move);handle.addEventListener('pointerup',up);handle.addEventListener('pointercancel',up);
    });
    handle.addEventListener('keydown',event=>{
      if(['ArrowLeft','ArrowRight'].includes(event.key)){event.preventDefault?.();resize(p.w+((event.key==='ArrowRight')===(side==='left')?20:-20));remember();}
      if(event.key==='Enter'){p.folded=!p.folded;apply();remember();}
    });
    handle.addEventListener('dblclick',()=>{p.w=def;p.folded=false;apply();remember();});
    apply();
    return p;
  }
  function listDetail(host,list,detail,selected,label){
    const area=state.area;
    host.classList.add('list-detail');list.classList.add('detail-list');detail.classList.add('detail-content');
    host.classList.toggle('detail-open',!!views.detailOpen[area]);
    const back=key(button(null,'list-back',()=>{
      views.detailOpen[area]=false;setArea(area,state.mode);
      host.querySelector('[data-key="'+selected+'"]')?.focus({preventScroll:true});
    }),'back-to-list');back.append(A.icon('chevron',14),el('span',null,label));detail.prepend(back);
    list.scrollTop=views.listScroll[area]??0;
    const captureDetail=captureLabPosition;
    captureLabPosition=()=>{
      const single=windowEl.classList.contains('single-pane');
      if(!single||host.classList.contains('detail-open'))captureDetail();
      if(!single||!host.classList.contains('detail-open'))views.listScroll[area]=list.scrollTop;
    };
  }
  function focusDetail(host,selected){
    host.querySelector(windowEl.classList.contains('single-pane')?'[data-key="back-to-list"]':'[data-key="'+selected+'"]')?.focus({preventScroll:true});
  }
  /* Themes are generated from sp-night.json: each flavour's tokens fill the roles Vults reads. Black is
     today's app theme (themes.css), kept as the fallback. */
  const THEMES=[...Object.entries(SP_NIGHT.flavors).map(([value,f])=>({value,label:raw(f.label)})),{value:'black',label:'Black'}];
  const mixHex=(a,b,t)=>'#'+[1,3,5].map(i=>Math.round(parseInt(a.slice(i,i+2),16)*(1-t)+parseInt(b.slice(i,i+2),16)*t).toString(16).padStart(2,'0')).join('');
  function flavorVars(c){
    const r=k=>c[SP_NIGHT.roles[k]];
    return {'--bg':r('bg'),'--bg-deep':r('bg_deep'),'--chrome':r('bg_deep'),'--float':mixHex(r('bg_deep'),r('bg'),.5),'--input':mixHex(r('bg_deep'),r('bg'),.4),
      '--panel':r('panel'),'--raised':r('raised'),'--border':mixHex(r('panel'),r('raised'),.5),'--chrome-line':mixHex(r('panel'),r('raised'),.5),
      '--control-border':r('border_strong'),'--border-hover':mixHex(r('border_strong'),r('fg_muted'),.2),'--wire':r('border_strong'),
      '--line':r('fg')+'12','--hairline':r('fg')+'0a','--fg-bright':r('fg_bright'),'--fg':r('fg'),'--fg-dim':r('fg_dim'),'--fg-muted':r('fg_muted'),
      '--soft':r('fg_dim'),'--chrome-icon':mixHex(r('fg_dim'),r('fg'),.4),'--accent':r('fg_bright'),'--on-accent':r('bg_deep'),'--mark':r('fg'),
      '--ok':r('ok'),'--warn':r('warn'),'--error':r('error'),'--error-text':r('error_text'),'--info':r('info'),'--tab-accent':r('accent'),
      '--focus-border':mixHex(r('accent'),r('border_strong'),.55),'--focus-shadow':'0 0 0 2px '+r('bg')+',0 0 0 4px '+r('accent')+'47'};
  }
  let themeVars=[];
  function applyTheme(value){
    settings.theme=THEMES.some(t=>t.value===value)?value:'jaragua';
    for(const name of themeVars)root.style.removeProperty(name);themeVars=[];
    const flavor=SP_NIGHT.flavors[settings.theme];
    if(flavor)for(const [name,v] of Object.entries(flavorVars(flavor.colors))){root.style.setProperty(name,v);themeVars.push(name);}
    root.setAttribute('data-flavor',settings.theme);for(const pick of $$('.theme-select,.settings-theme'))pick.value=settings.theme;
    // The scenes cache their wire and agent colors: redraw them in the new flavour.
    for(const scene of activeScenes){scene.colors=null;scene.refresh?.();}
    document.body?.style&&(document.body.style.background=flavor?flavor.colors[SP_NIGHT.roles.bg]:'#000');
  }
  /* MODULES */
  const SCREEN_REVIEW=[
    {id:'island',label:'Island',open:()=>setMode('mini'),scope:'See only: badges by shape, the pulse, usage hairlines. A click opens Compact.'},
    {id:'chat',label:'Compact · Chat',open:()=>setArea('chat','quick'),scope:'Approved direction. Now with engines, commands, tools and Zeca\'s own cards.'},
    {id:'flock',label:'Compact · Flock and the card',open:()=>setArea('flock','quick'),scope:'One card in three densities: Always, 1 of N, countdown and question choices.'},
    {id:'now',label:'Compact · Now',open:()=>setArea('summary','quick'),scope:'One inbox for everything that asks for an action. Quick settings in the gear.'},
    {id:'connections',label:'Lab · Connections',open:()=>setArea('connections','room'),scope:'Approved direction. Now with a face for you and a face for Zeca.'},
    {id:'activity',label:'Lab · Activity',open:()=>setArea('activity','room'),scope:'Week, projects, history, cost, traces and audit with the app\'s real figures.'},
    {id:'settings',label:'Lab · Settings',open:()=>openSettings('general'),scope:'Every setting the app has, behind the gear.'},
    {id:'motion',label:'Motion & continuity',open:()=>{setArea('chat','room');showExample('thinking');},scope:'Reduced motion, keyboard use, scroll position and drafts across Island, Compact and Lab.'},
  ];
  function renderReview(host) {
    host.append(pageHeading('Review','Mark each screen. Your notes stay in this prototype.',button('Export review','primary',()=>download('vults-ui-review.json',JSON.stringify({prototype:'vults-ui-motion-proposal',sample:true,date:new Date().toISOString(),levels:{island:'see',compact:'act',lab:'lead'},sizes:A.SURFACE_SIZES,decisions,notes},null,2)))));
    const grid=el('div','review-grid');host.append(el('p','pane-note','Island sees, Compact acts, Lab leads. Each level shows more of the same thing, never a copy.'),grid);
    SCREEN_REVIEW.forEach(screen=>{
      const row=key(el('article','review-row',null,el('h3',null,screen.label),chip(decisions[screen.id]==='approved'?'done':decisions[screen.id]==='refine'?'needs':'quiet',decisionLabel(decisions[screen.id])),el('p',null,screen.scope)),screen.id);
      const textarea=el('textarea');textarea.value=notes[screen.id]??'';textarea.placeholder=L('What should change?');textarea.setAttribute('aria-label',phrase('Notes for {screen}',{screen:L(screen.label)}));textarea.addEventListener('input',()=>{notes[screen.id]=textarea.value.slice(0,800);remember();});
      const mark=value=>{decisions[screen.id]=value;row.querySelector('.status').replaceWith(chip(value==='approved'?'done':'needs',decisionLabel(value)));remember();};
      row.append(el('div','inline-actions',null,button('Open screen','quiet-control',screen.open),button('Keep this direction','',()=>mark('approved')),button('Needs refinement','',()=>mark('refine'))),textarea);grid.append(row);
    });
  }
  function decisionLabel(value){return value==='approved'?'Direction approved':value==='refine'?'Needs refinement':'Waiting for review';}
  function clearExampleHistory(){clearTimeout(replyTimer);state.busy=false;state.voice=false;state.messages=[];state.files=[];input.value='';input.hidden=false;$('.voice-preview').hidden=true;mic.setAttribute('aria-pressed','false');state.conversation='current';conversations.splice(0,conversations.length,{id:'current',title:'Current conversation',messages:[],draft:''});auditRows.splice(0);settings.recordsCleared=true;paintFiles();paintConversation();paintRecent();remember();toast('History cleared. Memory, policies, phones and costs stay.');}
  function download(name,text,type='application/json'){
    if(typeof Blob==='undefined'||!URL.createObjectURL){toast('Export is not available in this viewer.');return;}
    const url=URL.createObjectURL(new Blob([text],{type})),a=el('a');a.href=url;a.download=name;document.body.append(a);a.click();a.remove();window.setTimeout(()=>URL.revokeObjectURL(url),1000);toast('Exported');
  }
  function translateStatic(){for(const node of $$('[data-word]'))node.textContent=L(node.dataset.word);for(const node of $$('[data-label]'))node.setAttribute('aria-label',L(node.dataset.label));input.placeholder=L(['room','full'].includes(state.mode)?'Ask Zeca…':'Ask Zeca… / for commands, @ for a session');input.setAttribute('aria-label',L('Message'));for(const node of $$('[data-size]'))node.textContent=MODE_NAME[node.dataset.size]==='Island'?L('Island'):MODE_NAME[node.dataset.size];}
  function setLanguage(value){language=LANGUAGES.includes(value)?value:'pt-BR';settings.language=language;A.setLang(language);$('.language-select').value=language;translateStatic();paintConversation();setArea(state.area,state.mode);remember();}
  // Compact: Flock · Chat · Now. Lab: Flock · Chat · Connections · Activity · Dictation. Settings is the gear in both.
  const COMPACT_NAV=[['flock','Flock','flock'],['chat','Chat','chat'],['bars','Now','summary']];
  const LAB_NAV=[['flock','Flock','flock'],['chat','Chat','chat'],['plug','Connections','connections'],['bars','Activity','activity'],['mic','Dictation','dictation']];
  function navCount(target){
    if(target==='flock')return flockFacts().needs.length+flockFacts().failed.length;
    if(target==='summary')return nowRows().filter(r=>r.urgent).length;
    if(target==='connections')return settings.github?unseenCount():0;
    return 0;
  }
  function paintNav() {
    const nav=$('.product-nav'),old=nav.querySelector('.nav-current')?.getBoundingClientRect(),focused=document.activeElement?.getAttribute('data-key');nav.replaceChildren();let selected;
    const items=state.mode==='quick'?COMPACT_NAV:LAB_NAV, current=state.area==='memory'?'chat':state.area;
    items.forEach(([glyph,label,target])=>{
      const nextMode=state.mode==='quick'?'quick':state.mode==='full'?'full':'room';
      const b=key(button(null,'',()=>{views.detailOpen[target]=false;setArea(target,nextMode);},label),'nav-'+target);b.append(A.icon(glyph,15),el('span','nav-label',label));
      const count=navCount(target);if(count)b.append(el('span','nav-count counter',null,document.createTextNode(String(count))));
      b.disabled=target==='chat'&&!settings.zeca;b.setAttribute('aria-pressed',String(target===current));
      if(target===current){b.classList.add('selected-area');selected=el('span','nav-current');selected.setAttribute('aria-hidden','true');b.prepend(selected);}nav.append(b);
      if(focused==='nav-'+target&&!restoring)b.focus({preventScroll:true});
    });
    if(old&&selected&&prefs.motion&&!restoring){const next=selected.getBoundingClientRect();selected.animate([{transform:'translateX('+(old.left-next.left)+'px)'},{transform:'translateX(0)'}],{duration:180,easing:'cubic-bezier(0.16,1,0.3,1)'});}
  }
  function paintChrome(){
    const actions=$('.product-head-actions');actions.replaceChildren();
    if(settings.media)actions.append(el('span','now-playing',null,A.icon('note',12),el('span',null,raw('Lo-fi radio'))));
    if(settings.dnd!=='Off')actions.append(key(iconButton('moon',phrase('Do not disturb until {time}',{time:'11:30'}),()=>{settings.dnd='Off';paintChrome();toast('Do not disturb is off');}),'dnd'));
    const sound=iconButton(settings.sounds?'sound':'mute','Sounds',()=>changedSetting('sounds',!settings.sounds));sound.setAttribute('aria-pressed',String(settings.sounds));
    const gear=key(iconButton('gear',state.mode==='quick'?'Quick settings':'Settings',event=>state.mode==='quick'?openQuickSettings(gear):openSettings()),'head-settings');gear.setAttribute('aria-pressed',String(state.area==='settings'));
    actions.append(sound,iconButton('search','Search (Ctrl+K)',()=>openPalette()));
    if(state.mode!=='quick')actions.append(key(iconButton('fullscreen',state.mode==='full'?'Leave full screen (Esc)':'Full screen',()=>setMode(state.mode==='full'?'room':'full')),'head-full'));
    actions.append(gear);
  }
  function setMode(mode){
    if(!['mini','quick','room','full'].includes(mode))return;
    const area=mode==='mini'?'flock':state.mode==='mini'?(mode==='quick'?views.quickArea:views.roomArea):mode==='quick'&&!['chat','flock','summary'].includes(state.area)?views.quickArea:state.area;
    setArea(area,mode);
  }
  function setArea(area,mode=state.mode) {
    if(!AREAS.includes(area))return;
    captureLabPosition();captureLabPosition=()=>{};
    if(area==='memory'){state.memoryPanel=true;area='chat';if(['mini','quick'].includes(mode))mode='room';}
    if(!['mini','quick','room','full'].includes(mode))mode='room';
    if(['settings','review'].includes(area)&&['mini','quick'].includes(mode))mode='room';
    // Old Compact routes (activity, connections) land on Now, which links to them.
    if(mode==='quick'&&!['flock','chat','summary'].includes(area))area='summary';
    if(area==='summary'&&['room','full'].includes(mode))area='flock';
    if(!settings.zeca&&area==='chat')area='flock';
    const previous=state.area,previousMode=state.mode;if(previous==='chat'&&previousMode!=='mini')views.chatScroll=log.scrollTop;
    if(['room','full'].includes(mode)&&previousMode==='quick'&&area==='flock')views.detailOpen.flock=true;
    if(previousMode==='quick')views.quickArea=previous;else if(['room','full'].includes(previousMode)&&previous!=='review')views.roomArea=previous;
    state.area=area;state.mode=mode;closeModels(false,false);closeMenu(history,historyTrigger,false,false);closeCommands();closeActionMenu(false);stopScenes();
    $$('[data-flock-example]').forEach(option=>option.hidden=!['flock','summary'].includes(area)&&mode!=='mini');
    const mini=mode==='mini',room=['room','full'].includes(mode),chat=area==='chat';
    // The Lab has one bar: its tabs sit in the header between the name and the actions.
    const nav=$('.product-nav'),head=$('.product-head');
    if(room&&nav.parentElement!==head)$('.product-head-actions').before(nav);
    else if(!room&&nav.parentElement===head)head.after(nav);
    windowEl.hidden=mini;$('.mini-strip').hidden=!mini;windowEl.classList.toggle('workspace',room);windowEl.classList.toggle('review-mode',area==='review');
    $('.chat-shell').hidden=!chat||mini;chatEl.hidden=!chat||mini;$('.workspace-view').hidden=chat||mini;$('.room-recent').hidden=!room;
    memoryTrigger.hidden=!room;
    $('.scenario-select').value=state.scenario;
    $('.surface-name').textContent=area==='review'?L('Design review'):L(MODE_NAME[mode]);
    $('.product-size').hidden=area==='review';$('.product-nav').hidden=area==='review';
    windowEl.setAttribute('aria-label',area==='review'?'Vults design review':'Vults '+MODE_NAME[mode]);
    $$('[data-area]').forEach(b=>b.setAttribute('aria-pressed',String(b.dataset.area===area&&(!b.dataset.mode||b.dataset.mode===mode||(b.dataset.mode==='room'&&mode==='full')))));
    $$('[data-size]').forEach(b=>{const on=b.dataset.size===mode||(b.dataset.size==='room'&&mode==='full');b.classList.toggle('selected',on);b.setAttribute('aria-pressed',String(on));});
    layoutSurface(!restoring&&previousMode!==mode);
    const workspace=$('.workspace-view');workspace.replaceChildren();workspace.classList.remove('compact-flock','compact-summary','lab-flock','lab-split','with-sky','list-detail','detail-open','dx-view');paintNav();paintChrome();
    paintNeedsStrip();
    if(mini)renderIsland($('.mini-strip'));
    else if(chat){
      paintRecent();paintConversation();paintMemoryPanel();log.scrollTop=views.chatScroll;mic.hidden=!settings.voice;
      if(state.scenario==='error')log.prepend(el('div','pane-note state-error','Could not update this view.'),button('Try again','quiet-control',()=> {state.scenario='ready';setArea('chat',state.mode);}));
    }
    else if(mode==='quick'&&area==='flock')renderCompactFlock(workspace);
    else if(mode==='quick'&&area==='summary')renderNow(workspace);
    else ({flock:renderFlock,activity:renderActivity,connections:renderConnections,dictation:renderDictation,settings:renderSettings,review:renderReview})[area]?.(workspace);
    $('.access-trigger').textContent=L(settings.access);$('.folder-trigger').textContent=L(settings.chatFolder??'Vults');
    if(!chat)input.hidden=state.voice?true:false;
    paintBird();paintSuggestions();translateStatic();if(previous!==area&&!restoring)enter(chat?chatEl:workspace,'gentle',4,1);remember();
  }
  // In the Lab, whatever needs you shows above every tab except Flock, where it is the list itself.
  function paintNeedsStrip(){
    const strip=$('.needs-strip');strip.replaceChildren();
    const room=['room','full'].includes(state.mode);
    const rows=[...flockFacts().needs.map(s=>({s,shape:s.status==='question'?'question':'needs',label:phrase('{project}: {brief}',{project:s.project,brief:L(sessionBrief(s))}),open:()=>{views.sessionSelected=s.id;views.detailOpen.flock=true;views.flockDetail='session';setArea('flock',state.mode);}})),
      ...flockFacts().failed.map(s=>({s,shape:'failed',label:phrase('{project}: {brief}',{project:s.project,brief:L(sessionBrief(s))}),open:()=>{views.sessionSelected=s.id;views.detailOpen.flock=true;views.flockDetail='activity';setArea('flock',state.mode);}}))].filter(r=>!isSkipped(r.s));
    strip.hidden=!room||state.area==='flock'||state.area==='review'||!rows.length;
    if(strip.hidden)return;
    strip.append(el('span','needs-title',null,chip('needs','Needs you')));
    const shown=rows.slice(0,3),rest=rows.slice(3);
    shown.forEach(r=>{
      const open=key(button(null,'needs-open',r.open),'needs-'+r.s.id);open.append(el('span','shape shape-'+r.shape),el('span',null,r.label));
      strip.append(el('span','needs-chip',null,open,key(iconButton('close','Skip',()=>{skip(r.s);paintNeedsStrip();}),'skip-'+r.s.id)));
    });
    if(rest.length){const more=key(button(null,'needs-more',()=>openActionMenu(more,'Needs you',rest.map(r=>({label:r.label,icon:'person',action:r.open})))),'needs-more');more.append(document.createTextNode('+'+rest.length));strip.append(more);}
    strip.append(key(button('Skip all','needs-skip-all',()=>{rows.forEach(r=>skip(r.s));paintNeedsStrip();}),'skip-all'));
  }
  function setScenarioQuiet(value){state.scenario=value;applyScenario(value);}
  function applyScenario(value){
    sessions.splice(6);sessions.forEach(s=>s.pending=s.status==='approval'?2:0);
    const s=sessions[0],pose={read:['working','read','docs/dev/plan-ui.md'],edit:['working','edit','ui/src/island/now.ts'],run:['working','run','npm run build'],done:['finished',null,'npm run build'],rest:['idle',null,'npm run build'],fail:['failed','run','npm run build'],think:['thinking','think','Plan the Now inbox']}[value];
    s.step=pose?pose[2]:value==='destructive'?'git clean -fdx':value==='question'||value==='question-multi'?'Which checks should run?':'npm run build';
    s.status=pose?pose[0]:['question','question-multi'].includes(value)?'question':value==='expired'?'idle':'approval';s.activity=pose?pose[1]:'run';
    s.multi=value==='question-multi';s.card=s.waiting=['approval','question'].includes(s.status);s.pending=s.status==='approval'?2:0;
    if(value==='many')for(let i=0;i<10;i++)sessions.push({...sessions[i%5],id:'extra-'+i,project:raw('project-'+(i+1)),status:i%3===0?'working':'idle',card:false,waiting:false,quiet:null,pending:0,task:null});
    A.assignSpecies(sessions);
  }
  function setScenario(value){
    state.scenario=value;
    if(['flock','summary'].includes(state.area)||state.mode==='mini'){views.flockDetail='session';views.sessionSelected='session-1';applyScenario(value);}
    if(state.area==='chat'){if(value==='empty')showExample('empty');else if(value==='loading')showExample('thinking');}
    setArea(state.area,state.mode);
  }
  root.addEventListener('keydown',event=>{
    if(event.defaultPrevented||event.isComposing||event.keyCode===229)return;
    if(!$('.editor-overlay').hidden||!$('.dialog-overlay').hidden)return;
    if((event.ctrlKey||event.metaKey)&&event.key.toLowerCase()==='k'){event.preventDefault();openPalette();return;}
    if(event.altKey&&['1','2','3','4','5','6'].includes(event.key)){event.preventDefault();setArea(LAB_NAV[Number(event.key)-1][2],state.mode==='full'?'full':'room');return;}
    // Ctrl+Alt+Y / N answer the card on screen, as the app's global shortcuts do; never one that is not shown.
    if(event.ctrlKey&&event.altKey&&['y','n'].includes(event.key?.toLowerCase())){const card=$('.request-card');const b=card?.querySelector('[data-key="'+(event.key.toLowerCase()==='y'?'allow':'deny')+'"]');if(b){event.preventDefault();b.click();}return;}
    if(event.ctrlKey&&event.altKey&&['1','2','3'].includes(event.key)){event.preventDefault();setMode(['mini','quick','room'][Number(event.key)-1]);}
  });
  const media=matchMedia('(prefers-reduced-motion: reduce)');media.addEventListener?.('change',()=>{prefs.motion=settings.animations&&!media.matches;root.classList.toggle('motion-off',!prefs.motion);if(!prefs.motion)settleMotion();paintBird();for(const scene of activeScenes)scene.setActive(prefs.motion&&flockVisible());});
