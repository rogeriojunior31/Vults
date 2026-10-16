(() => {
  const root = document.getElementById('vults-ui-proposal');
  if (!root) return;
  const A = VultsProposalAssets;
  const copy = /* EXTRA_COPY */;
  const SP_NIGHT = /* SP_NIGHT */;
  const LANGUAGES = ['en','pt-BR','es','zh'];
  let language = 'pt-BR';
  A.setLang(language);
  // Sample data (projects, commands, paths, model names) is shown as is; everything else must be translated.
  const missing = new Set();
  const RAW = /^(?:(?:npm|git|cargo|gh|curl|pnpm) .*|LM Studio|Large v3 Turbo|Portugu.s \(Brasil\)|[\w-]*[.@#/:~_+][\w.@#/:~+-]*|[\w-]*\d[\w-]*|[\w.-]+ · [\w.-]+|\$?[\d.,]+%?|\S*[/`$=|<>]\S*|v?\d[\d.]*)$/;
  const L = text => {
    if (text===undefined||text===null) return text;
    text = String(text);
    const index = ['pt-BR','es','zh'].indexOf(language);
    if (index<0) return text;
    const found = copy[text]?.[index];
    if (found) return found;
    const shared = A.t(text);
    if (shared!==text) return shared;
    if (/[A-Za-z]{2}/.test(text) && !RAW.test(text) && !NAMES.has(text) && !rawText.has(text)) missing.add(text);
    return text;
  };
  const rawText = new Set();
  // Names stay as they are in every language.
  const NAMES = new Set(['Vults','Zeca','Compact','Lab','GitHub','Codex','Claude Code','OpenCode','Qwen Code','Copilot CLI','Kimi Code','Cursor','Factory Droid','Antigravity','Gemini CLI','Anthropic','OpenAI','OpenRouter','Ollama','Google','Groq','DeepSeek','Mistral','xAI','Brave','Tavily','SearXNG','Sonnet','Opus','Haiku','Markdown','PDF','Bash','Read','frontend','docs','api','infra','design','Relay','Direct','English','Portugu\u00eas','Espa\u00f1ol','tmux','kitty']);
  const raw = text => { rawText.add(String(text)); return String(text); };
  const $ = selector => root.querySelector(selector);
  const $$ = selector => [...root.querySelectorAll(selector)];
  const el = (tag, cls, text, ...children) => {
    const n = document.createElement(tag);
    if (cls) n.className = cls;
    if (text !== undefined && text !== null) n.textContent = L(text);
    n.append(...children.filter(Boolean));
    return n;
  };
  const button = (text, cls, action, name) => {
    const b = el('button', cls, text); b.type = 'button';
    if (name) { b.setAttribute('aria-label', L(name));b.setAttribute('data-label',name); }
    b.addEventListener('click', action); return b;
  };
  const iconButton = (name, label, action) => {
    const b = button(null, 'icon-action', action, label); b.append(A.icon(name, 15)); b.setAttribute('data-tooltip',L(label)); return b;
  };
  const kbd = text => el('kbd',null,null,document.createTextNode(text));
  const prefs = { motion: !matchMedia('(prefers-reduced-motion: reduce)').matches };
  function enter(node, preset='snappy', dy=6, scale=.98) {
    if (!prefs.motion || !node?.animate) return;
    node.animate([{opacity:0,transform:`translateY(${dy}px) scale(${scale})`},{opacity:1,transform:'none'}], { duration:preset==='gentle'?200:160, easing:'cubic-bezier(0.16,1,0.3,1)', fill:'none' });
  }
  const windowEl = $('.product-window'), chatEl = $('.chat-view'), log = $('.transcript');
  const input = $('textarea'), popover = $('.model-popover'), history = $('.history-popover'), commands = $('.command-popover'), actionMenu = $('.action-menu');
  const state = { area:'chat', mode:'quick', scenario:'ready', activity:'week', category:'general', conversation:'current', example:'empty', engine:'claude', model:'Default model', effort:'Default effort', files:[], messages:[], queued:[], busy:false, voice:false, notice:false, memoryPanel:false, lastSent:'' };
  let restoring = true;
  const AREAS=['summary','flock','chat','activity','connections','dictation','settings','memory','review'];
  const original = window.openai?.widgetState;
  if (original?.modelContent?.area && AREAS.includes(original.modelContent.area)) state.area=original.modelContent.area;
  function remember() {
    if (restoring) return;
    window.openai?.setWidgetState?.({ modelContent:{ area:state.area, mode:state.mode, activity:state.activity, category:state.category, example:state.example, engine:state.engine, model:state.model }, privateContent:snapshot() }).catch(() => {});
  }
  // A 3x scene divisible by three keeps the same director crisp at 1x, 2x and 3x.
  const scene = new A.Scene({...A.FOCUS_SCENE,width:114,zeca:{...A.FOCUS_SCENE.zeca,x:21}});
  scene.canvas.setAttribute('aria-hidden','true'); $('.portrait').append(scene.canvas);
  // A paused scene still needs one drawn frame, so reduced motion shows the birds standing still.
  function still(target){ if (!prefs.motion) window.setTimeout(() => target.setActive(false),60); }
  /* Zeca is a companion: his clip follows the conversation (listening while you type, searching while a
     tool runs, thinking, speaking as the reply lands, waiting with you on a card). */
  let moodTimer, typingTimer;
  const waitingCard = () => state.messages.some(m=>m.who==='card'&&!m.answer);
  function zecaClip() {
    if (state.mood) return state.mood;
    if (state.busy) return state.toolPhase?'search':'think';
    if (state.voice||state.typing) return 'listen';
    if (waitingCard()) return 'approval';
    return 'idle';
  }
  function zecaDo(clip,ms=1400) { clearTimeout(moodTimer); state.mood=clip; paintBird(); moodTimer=window.setTimeout(()=>{ state.mood=null; paintBird(); },ms); }
  function paintBird(clip) {
    scene.update([],null,{ clip:clip ?? zecaClip(),agent:engineInfo().agent });
    scene.setActive(settings.zeca && state.area==='chat' && state.mode!=='mini' && !windowEl.classList.contains('very-short') && !(windowEl.classList.contains('workspace')&&windowEl.classList.contains('tight')) && !document.hidden);
    still(scene);
  }
  $('.portrait').addEventListener('click',() => { if (prefs.motion && !state.busy && !state.voice) scene.react('hello'); });
  $('.portrait').addEventListener('pointerenter',() => { if (prefs.motion) scene.lookAt('front'); });
  $('.portrait').addEventListener('pointerleave',() => { if (prefs.motion) scene.lookAt(null); });
  document.addEventListener('visibilitychange',() => paintBird());

  /* Engines: your own CLI setup first, then the Vults harness with an API key (Settings › Chat & voice). */
  const ENGINES = [
    {id:'claude',group:'Your setup',name:'Claude Code',agent:'claude',status:'Signed in',models:['Default model','Sonnet','Opus','Haiku'],usage:42},
    {id:'codex',group:'Your setup',name:'Codex',agent:'codex',status:'Installed',models:['Default model','GPT-6.1-Sol','GPT-6-Astra','GPT-6-Sol'],usage:76},
    {id:'anthropic',group:'API',name:'Anthropic',agent:'claude',status:'Key saved',models:['Opus','Sonnet','Haiku']},
    {id:'openrouter',group:'API',name:'OpenRouter',agent:'other',status:'Key saved',models:['deepseek-v4','qwen3-coder','kimi-k2.5']},
    {id:'ollama',group:'API',name:'Ollama',agent:'other',status:'Local',models:['qwen3-coder:30b','gpt-oss:20b']},
    {id:'openai',group:'API',name:'OpenAI',agent:'codex',status:'No key',models:[]},
  ];
  const engineInfo = (id=state.engine) => ENGINES.find(e=>e.id===id)??ENGINES[0];
  const EFFORTS = ['Default effort','Low','Medium','High'];

  for (const node of $$('[data-word]')) node.textContent=L(node.dataset.word);
  const modelTrigger = button(null,'model-trigger',() => toggleModels(), 'Model and effort');
  modelTrigger.setAttribute('aria-expanded','false'); modelTrigger.setAttribute('aria-controls','vults-proposal-model-menu');
  popover.id='vults-proposal-model-menu';
  const attach = iconButton('clip','Attach a file',() => addFile('design-notes.md'));
  const mic = iconButton('mic','Voice (Ctrl+Alt+V)',() => toggleVoice());
  const send = button(null,'send-action',() => submit(), 'Send (Enter)'); send.append(A.icon('chevron',15));
  // Model, folder and access live in one toolbar inside the composer.
  $('.composer-left').append(attach,modelTrigger,$('.context-folder'),$('.context-access')); $('.composer-right').append(mic,send);
  const historyTrigger = iconButton('list','Conversations',() => toggleHistory());
  history.id='vults-proposal-history';historyTrigger.setAttribute('aria-expanded','false');historyTrigger.setAttribute('aria-controls',history.id);
  const memoryTrigger = iconButton('sideRight','Memory',() => toggleMemoryPanel());memoryTrigger.classList.add('memory-trigger');
  $('.chat-actions').append(historyTrigger, memoryTrigger, iconButton('plus','New conversation',() => newConversation()));
  $('.context-folder').append(A.icon('folder',12),button('Vults','folder-trigger',() => openFolder(),'Folder for this conversation'),A.icon('chevronDown',10));
  $('.context-access').append(A.icon('person',12),button('Asks first','access-trigger',() => openAccess(),"Who answers Zeca's cards"));
  // The composer resizes from one place, also when a draft or a conversation is loaded.
  function fitInput(){ input.style.height='auto'; input.style.height=Math.min(state.mode==='quick'?90:220,input.scrollHeight)+'px'; paintSend(); }
  input.placeholder=L('Ask Zeca… / for commands, @ for a session'); input.setAttribute('aria-label',L('Message'));
  input.addEventListener('input',() => { if (input.value.trim()&&(!state.typing||state.mood)) { state.typing=true; state.mood=null; clearTimeout(moodTimer); paintBird(); } clearTimeout(typingTimer); typingTimer=window.setTimeout(()=>{ state.typing=false; paintBird(); },1500); input.style.height='auto'; input.style.height=Math.min(state.mode==='quick'?90:220,input.scrollHeight)+'px'; paintSend(); paintCommands(); remember(); });
  input.addEventListener('keydown',event => {
    if (event.isComposing || event.keyCode===229) return;
    if (!commands.hidden && ['ArrowDown','ArrowUp','Enter','Tab','Escape'].includes(event.key)) { commandKey(event); return; }
    if (event.key==='ArrowUp' && !input.value && state.lastSent) { event.preventDefault(); input.value=state.lastSent; input.dispatchEvent(new Event('input')); return; }
    if (event.key==='Enter'&&!event.shiftKey) { event.preventDefault(); submit(true); }
  });
  const composer = $('.composer-surface');
  composer.addEventListener('dragover',event => { event.preventDefault?.(); composer.classList.add('dropping'); });
  composer.addEventListener('dragleave',() => composer.classList.remove('dropping'));
  composer.addEventListener('drop',event => { event.preventDefault?.(); composer.classList.remove('dropping'); addFile(event.dataTransfer?.files?.[0]?.name ?? 'screenshot.png'); });
  function addFile(name) {
    if (!settings.zeca) { toast('Turn Zeca on to share files.'); return; }
    if (!state.files.includes(name)) state.files.push(raw(name));
    state.files=state.files.slice(0,4); paintFiles(); paintBird('swallow'); window.setTimeout(() => paintBird(),1400); remember();
  }
  function details(title, rows, cls='run-details') {
    const d=el('details',cls), s=el('summary',null,null,A.icon('chevron',12),el('span',null,title));
    const body=el('div','run-body');
    rows.forEach(row => body.append(typeof row==='string'?el('div',null,null,A.icon('check',12),el('span',null,row)):row));
    d.append(s,body); d.addEventListener('toggle',() => { if (d.open) enter(body,'gentle',-3,1); });
    return d;
  }
  function toolRow(name,args,result){ return el('div','tool-row',null,el('code',null,raw(name+'('+args+')')),el('span',null,result)); }
  function replyTools(m) {
    const rows = m.tools ?? [toolRow('flock_list','',phrase('{n} sessions',{n:sessions.length})),toolRow('recall','"recent changes"','1 note from Vults')];
    return details(phrase(rows.length===1?'Used 1 tool · {s} s':'Used {n} tools · {s} s',{n:rows.length,s:m.seconds??'0.4'}),rows,'run-details tool-details');
  }
  /* In the Lab chat Zeca has his own place: large above the suggestions when nothing is said yet,
     then perched on the composer's edge, his wire on its border, reacting as the conversation goes. */
  function placeZeca() {
    const p=$('.portrait'),room=['room','full'].includes(state.mode),populated=state.messages.length>0||state.busy;
    p.classList.toggle('zeca-hero',room&&!populated);p.classList.toggle('zeca-perch',room&&populated);
    if (room&&populated) { if (p.parentElement!==composer) composer.prepend(p); }
    else if (room) { if (p.nextElementSibling!==$('.room-suggestions')) $('.room-suggestions').before(p); }
    else if (p.parentElement!==$('.chat-intro')) $('.chat-intro').prepend(p);
  }
  function paintConversation() {
    const readingTop=log.scrollTop;
    chatEl.classList.toggle('populated',state.messages.length>0||state.busy);
    const title=$('.intro-copy h2'), caption=$('.intro-caption');
    title.textContent=state.messages.length||state.busy?'Zeca':greeting();
    caption.textContent=state.messages.length||state.busy?phrase('{engine} · {folder}',{engine:engineInfo().name,folder:settings.chatFolder}):flockLine();
    log.replaceChildren();
    // Follow-ups stay on the latest reply, even when a note from Zeca came after it.
    const lastReply=state.messages.map((m,i)=>m.who==='zeca'?i:-1).reduce((a,b)=>Math.max(a,b),-1);
    state.messages.forEach((m,index) => {
      if (m.who==='you') { log.append(el('div','message you',m.text)); return; }
      if (m.who==='card') { log.append(chatCard(m)); return; }
      if (m.tools!==false) log.append(replyTools(m));
      const reply=el('div','message reply',null,el('div','message-signature',null,el('span','agent-dot '+engineInfo().agent),el('span',null,'Zeca'),el('small',null,raw(m.model??modelLabel()))),el('p',null,m.text));
      if (m.list) { const list=el('ul'); m.list.forEach(text => list.append(el('li',null,text))); reply.append(list); }
      if (m.link) reply.append(el('div','inline-actions',null,button(m.link.label,'quiet-control',m.link.action)));
      if (m.memory) { const fact=memory.facts.find(f=>f.id===m.memory); if (fact) reply.append(button(null,'memory-note',()=>openMemory()));reply.querySelector('.memory-note')?.append(A.icon('sideRight',11),el('span',null,phrase('From memory: {fact}',{fact:L(fact.text)}))); }
      if (m.chips&&index===lastReply&&!state.busy) reply.append(el('div','reply-chips',null,...m.chips.map(c=>key(button(c.label,'reply-chip',()=>followUp(c)),'chip-'+c.id))));
      const tools=el('div','message-tools',null,button('Copy','',() => copyText(m.text)));
      if (index===state.messages.length-1) tools.append(button('Try again','',() => retry()));
      reply.append(tools); log.append(reply);
    });
    if (state.busy) {
      const live=el('details','run-details tool-details live',null,el('summary',null,null,el('span','live-dot'),el('span',null,state.liveTools?.length?phrase(state.liveTools.length===1?'Using 1 tool…':'Using {n} tools…',{n:state.liveTools.length}):'Thinking…')),el('div','run-body',null,...(state.liveTools??[]).map(t=>toolRow(t.name,t.args,t.result??'running…'))));
      live.open=true;live.addEventListener('toggle',()=>{if(state.busy)live.open=true;});log.append(live);
    }
    const queue=$('.queue-strip');queue.replaceChildren(...state.queued.map(text=>el('span','queued',null,A.icon('list',11),el('span',null,phrase('Queued: {text}',{text})))));
    paintSend(); log.scrollTop=readingTop;
    placeZeca(); paintBird(); paintModelTrigger(); paintSuggestions();
  }
  function paintSend() {
    const empty=!input.value.trim()&&!state.files.length;
    send.disabled=!state.busy&&empty;
    const stop=state.busy&&empty;
    send.setAttribute('aria-label',L(stop?'Stop':state.busy?'Queue this message':'Send (Enter)'));
    send.replaceChildren(A.icon(stop?'stop':'chevron',15));
  }
  function copyText(text) {
    const done=() => toast('Copied');
    try { navigator.clipboard.writeText(L(text)).then(done,() => toast('Select the text to copy it.')); } catch { toast('Select the text to copy it.'); }
  }
  function paintFiles() {
    const host=$('.attachment-strip'); host.replaceChildren();
    state.files.forEach(name => {
      const kind=name.endsWith('.png')?'Image':name.endsWith('.pdf')?'PDF':'Markdown';
      const item=el('div','attachment',null,A.icon('file',18),el('div',null,null,button(name,'attachment-open',() => openEditor(name,el('pre','diff-preview',raw('# '+name+'\n\n- Keep the Now inbox to actionable rows.\n- Compact answers, Lab investigates.')))),el('small',null,kind)));
      item.append(iconButton('close','Remove',() => { state.files=state.files.filter(f=>f!==name); paintFiles(); }));
      host.append(item); enter(item,'bouncy',0,.92);
    });
    paintSend();
  }
  const modelLabel = () => { const e=engineInfo(); const model=state.model==='Default model'?e.name:state.model; return state.effort==='Default effort'?model:model+' · '+L(state.effort); };
  function ring(percent) {
    const ns='http://www.w3.org/2000/svg', svg=document.createElementNS(ns,'svg');svg.setAttribute('viewBox','0 0 16 16');svg.setAttribute('class','usage-ring '+(percent>=90?'critical':percent>=70?'warning':''));svg.setAttribute('width','14');svg.setAttribute('height','14');
    const track=document.createElementNS(ns,'circle'),value=document.createElementNS(ns,'circle');
    for (const c of [track,value]) { c.setAttribute('cx','8');c.setAttribute('cy','8');c.setAttribute('r','6');c.setAttribute('fill','none');c.setAttribute('stroke-width','2'); }
    track.setAttribute('class','ring-track');value.setAttribute('class','ring-value');value.setAttribute('stroke-dasharray',(percent/100*37.7).toFixed(1)+' 37.7');value.setAttribute('transform','rotate(-90 8 8)');
    svg.append(track,value);return svg;
  }
  function paintModelTrigger() {
    const e=engineInfo();
    modelTrigger.replaceChildren(el('span','agent-dot '+e.agent),el('span','model-text',null,document.createTextNode(modelLabel())),...(e.usage!==undefined?[ring(e.usage)]:[]),A.icon('chevronDown',11));
    modelTrigger.setAttribute('data-tooltip',e.usage!==undefined?phrase('{name} plan: {n}% used',{name:e.name,n:e.usage}):L('Billed to your API key'));
  }
  function modelChoices() {
    const list=$('.model-options'); list.replaceChildren();
    const locked=state.messages.length>0||state.busy;
    if (locked) {
      list.append(el('p','pinned-model',"A conversation keeps its engine and model; New starts another."),button('New conversation','quiet-control',() => { closeModels(); newConversation(); toggleModels(true); }));
      $('.model-popover-foot').hidden=true;return;
    }
    $('.model-popover-foot').hidden=false;
    for (const group of ['Your setup','API']) {
      const grid=el('div','engine-grid');list.append(el('small','menu-group',group),grid);
      ENGINES.filter(e=>e.group===group).forEach(e => {
        const choice=key(button(null,'engine-option',() => { state.engine=e.id; state.model=e.models[0]; modelChoices(); paintModelTrigger(); paintBird(); positionOpenMenus(); remember(); }),'engine-'+e.id);
        choice.disabled=!e.models.length;choice.setAttribute('aria-pressed',String(state.engine===e.id));
        choice.append(el('span','agent-dot '+e.agent),el('span',null,raw(e.name)),...(e.usage!==undefined?[el('small','counter',raw(e.usage+'%'))]:[]));choice.setAttribute('data-tooltip',L(e.status));
        grid.append(choice);
      });
    }
    if (engineInfo().group==='API' && settings.provider==='CLI') list.append(el('p','pane-note','The Vults harness bills your API key. Zeca keeps the same tools.'));
    list.append(el('small','menu-group','Model'));
    engineInfo().models.forEach(name => {
      const choice=button(null,'model-option',() => { state.model=name; paintModelTrigger(); closeModels(); remember(); });
      choice.setAttribute('aria-pressed',String(state.model===name));
      choice.append(el('span','check-space',null,...(state.model===name?[A.icon('check',13)]:[])),el('span',null,name));
      if (name==='Default model') choice.append(el('small',null,'Your CLI default'));
      list.append(choice);
    });
  }
  function toggleModels(force) {
    const open=force ?? modelTrigger.getAttribute('aria-expanded')!=='true';
    closeMenu(history,historyTrigger,false,false);closeCommands();
    if (!open) { closeModels(); return; }
    modelChoices();
    openMenu(popover,modelTrigger);
    $('.model-close').focus({preventScroll:true});
  }
  function closeModels(returnFocus=true,animate=true) { closeMenu(popover,modelTrigger,returnFocus,animate); }
  $('.model-close').append(A.icon('close',13)); $('.model-close').setAttribute('aria-label',L('Close'));
  $('.model-close').addEventListener('click',() => closeModels());
  EFFORTS.forEach(word => { const o=el('option',null,word);o.value=word;$('.effort-select').append(o); });
  $('.effort-select').addEventListener('change',event => { state.effort=event.target.value; paintModelTrigger(); remember(); });
  popover.addEventListener('keydown',event => {
    if (event.isComposing || event.keyCode===229 || event.target.tagName==='SELECT') return;
    if (!['ArrowDown','ArrowUp','Home','End'].includes(event.key)) return;
    const choices=[...popover.querySelectorAll('.model-option')]; if (!choices.length) return;
    event.preventDefault(); const i=choices.indexOf(document.activeElement);
    choices[event.key==='Home'?0:event.key==='End'?choices.length-1:(i+(event.key==='ArrowDown'?1:-1)+choices.length)%choices.length].focus();
  });

  /* Slash commands and @ mentions. A command never answers a card. */
  const COMMANDS = [
    ['/continue','Resume a stopped session (within 10 minutes)','/continue'],
    ['/remember','Save a fact to Zeca\'s memory','/remember '],
    ['/forget','Remove a fact from Zeca\'s memory','/forget '],
  ];
  let commandIndex=0, commandRows=[];
  function paintCommands() {
    const value=input.value, slash=/^\/\w*$/.exec(value), mention=/(?:^|\s)@(\w*)$/.exec(value);
    if (!slash && !mention) { closeCommands(); return; }
    commandRows = slash
      ? COMMANDS.filter(([name])=>name.startsWith(value)).map(([name,about,insert])=>({label:name,about,apply:() => insert}))
      : sessions.filter(s=>s.project.toLowerCase().startsWith(mention[1].toLowerCase())).slice(0,6).map(s=>({label:'@'+s.project,about:raw(labelFor(s.agent)+' · '+L(sessionBrief(s))),apply:() => value.slice(0,value.length-mention[1].length-1)+'@'+s.project+' '}));
    if (!commandRows.length) { closeCommands(); return; }
    commandIndex=Math.min(commandIndex,commandRows.length-1);
    commands.replaceChildren(el('small','menu-group',slash?'Commands':'Sessions'),...commandRows.map((row,i) => {
      const b=button(null,'command-option',() => pickCommand(i));b.setAttribute('aria-selected',String(i===commandIndex));b.setAttribute('role','option');
      b.append(el('code',null,raw(row.label)),el('small',null,row.about));return b;
    }));
    commands.setAttribute('role','listbox');
    if (commands.hidden) openMenu(commands,input); else positionMenu(commands,input);
  }
  function pickCommand(i) { const row=commandRows[i]; if (!row) return; input.value=row.apply(); closeCommands(); input.focus(); input.dispatchEvent(new Event('input')); }
  function commandKey(event) {
    event.preventDefault();
    if (event.key==='Escape') { closeCommands(); return; }
    if (event.key==='Enter'||event.key==='Tab') { pickCommand(commandIndex); return; }
    commandIndex=(commandIndex+(event.key==='ArrowDown'?1:-1)+commandRows.length)%commandRows.length;paintCommands();
  }
  function closeCommands() { if (!commands.hidden) closeMenu(commands,input,false,false); commandIndex=0; }

  function toggleHistory() {
    closeModels(false,false);
    if (history.getAttribute('data-open')==='true') {closeMenu(history,historyTrigger,true);return;}
    paintHistoryMenu();openMenu(history,historyTrigger);
  }
  function paintHistoryMenu() {
    history.replaceChildren(el('div','model-popover-head',null,el('strong',null,'Conversations'),iconButton('close','Close',() => closeMenu(history,historyTrigger,true))));
    saveConversation();conversations.slice(0,8).forEach(c=>history.append(conversationRow(c,() => closeMenu(history,historyTrigger,false,false))));
    history.append(button('New conversation','quiet-control',()=>{closeMenu(history,historyTrigger,false,false);newConversation();}));
  }
  root.addEventListener('scroll',event => { if (!actionMenu.hidden&&!actionMenu.contains(event.target)) closeActionMenu(false); },true);
  document.addEventListener('pointerdown',event => {
    if (!popover.hidden&&!popover.contains(event.target)&&!modelTrigger.contains(event.target)) closeModels(false);
    if (!history.hidden&&!history.contains(event.target)&&!historyTrigger.contains(event.target)) closeMenu(history,historyTrigger,false);
    if (!commands.hidden&&!commands.contains(event.target)&&event.target!==input) closeCommands();
    if (!actionMenu.hidden&&!actionMenu.contains(event.target)&&!menuOwner?.contains(event.target)) closeActionMenu(false);
  });

  /* Chat examples. Each one is a state the real chat reaches. */
  const sampleQuestion = () => raw(L('What changed recently?'));
  function showExample(example) {
    clearTimeout(replyTimer);
    state.voice=false; input.hidden=false; $('.voice-preview').hidden=true; mic.setAttribute('aria-pressed','false');
    state.example=example; state.busy=example==='thinking'; state.notice=example==='notification'; state.queued=[];
    state.messages=['conversation','thinking','notification'].includes(example)?[{who:'you',text:sampleQuestion()}]:[];
    if (example==='conversation'||example==='notification') state.messages.push(SAMPLE_REPLY());
    if (example==='permission'||example==='outside') {
      const outside=example==='outside';
      state.messages=[{who:'you',text:outside?'Install the linter the docs page recommends':'Show me the last commits in Vults'},
        {who:'zeca',text:outside?'The page recommends piping its install script to the shell. I need your answer first.':'I need to read the git log for that.',tools:[toolRow(outside?'web_fetch':'session_detail',outside?'"lint.example.dev/install"':'"Vults"',outside?'1 page read':'branch feat/compact-now')],seconds:outside?'1.2':'0.3'},
        {who:'card',id:'chat-card-'+example,tool:outside?'Bash':'Bash',command:outside?'curl -fsSL lint.example.dev/install | sh':'git log --oneline -5',cwd:'~/Projects/Vults',outside,answer:null}];
    }
    closeModels(false,false);closeMenu(history,historyTrigger,false,false);
    paintConversation(); paintNotice(); log.scrollTop=log.scrollHeight;
    $$('[data-example]').forEach(b => b.setAttribute('aria-pressed',String(b.dataset.example===example)));
    if (example==='models') toggleModels(true);
    remember();
  }
  const SAMPLE_REPLY = () => ({who:'zeca',chips:[{id:'diff',label:"Open frontend's changes",go:()=>{views.sessionSelected='session-2';views.flockDetail='diff';setArea('flock','room');}},{id:'card',label:"Show Vults' card",go:()=>{views.sessionSelected='session-1';setArea('flock',state.mode==='quick'?'quick':'room');}}],text:'Two sessions changed files in the last hour.',list:['frontend (Claude Code) edited ui/src/chat.ts: the reply keeps its scroll while it streams.','Vults (Codex) added the Now inbox and is waiting to run the build.'],tools:[toolRow('flock_list','','6 sessions'),toolRow('session_detail','"frontend"','8 steps, 1 subagent'),toolRow('session_detail','"Vults"','waiting for permission')],seconds:'0.6'});
  function chatCard(m) {
    const card=requestCard({id:m.id,agent:'zeca',project:'Vults',step:m.command,cwd:m.cwd,tool:m.tool,status:'approval',outside:m.outside,chat:true},'chat',answer => {
      m.answer=answer; zecaDo(answer==='Denied'?'preen':'done',1400);
      if (answer!=='Denied') state.messages.push({who:'zeca',text:m.outside?'Installed. The script added lint-pro to your PATH.':'The last five commits are on feat/compact-now; the newest adds the Now inbox.',tools:[toolRow('bash','"'+m.command+'"',m.outside?'exit 0':'5 lines')],seconds:'0.2'});
      paintConversation();remember();
    });
    if (m.answer) return el('div','card-outcome',null,A.icon(m.answer==='Denied'?'close':'check',13),el('span',null,phrase('{answer}: {command}',{answer:L(m.answer),command:m.command})));
    return card;
  }
  function followUp(c) {
    if (c.go) { c.go(); return; }
    input.value=L(c.send); input.dispatchEvent(new Event('input')); submit();
  }
  function greeting() {
    const hour=new Date().getHours(),part=hour<5?'Good evening':hour<12?'Good morning':hour<18?'Good afternoon':'Good evening';
    return settings.firstName?phrase(part+', {name}',{name:settings.firstName}):L(part);
  }
  function flockLine() {
    const f=flockFacts();
    if (f.needs.length&&f.failed.length) return phrase('{project} is waiting for you and {failed}\'s checks failed.',{project:f.needs[0].project,failed:f.failed[0].project});
    if (f.needs.length) return phrase('{project} is waiting for you.',{project:f.needs[0].project});
    if (f.failed.length) return phrase('{project}\'s checks failed.',{project:f.failed[0].project});
    if (f.working.length) return phrase('{n} agents are working. I am keeping an eye on them.',{n:f.working.length});
    return L('The flock is quiet. What are we doing?');
  }
  // Suggestions follow the flock: a card waiting, a failure, then the day.
  function companionSuggestions() {
    const f=flockFacts(),out=[];
    if (f.needs[0]) out.push({id:'s-card',label:phrase('What does {project} want to run?',{project:f.needs[0].project}),send:phrase('What does {project} want to run?',{project:f.needs[0].project})});
    if (f.failed[0]) out.push({id:'s-fail',label:phrase('Why did {project} fail?',{project:f.failed[0].project}),send:phrase('Why did {project} fail?',{project:f.failed[0].project})});
    out.push({id:'s-day',label:'What changed today?',send:'What changed today?'});
    if (memory.pending.length) out.push({id:'s-memory',label:'What do you want to remember?',go:()=>openMemory()});
    return out;
  }
  let replyTimer;
  function retry() {
    const last=[...state.messages].reverse().find(m=>m.who==='you');if (!last||state.busy) return;
    const at=state.messages.lastIndexOf(last);state.messages=state.messages.slice(0,at);input.value=last.text;submit();
  }
  function submit(fromKeyboard=false) {
    const text=input.value.trim();
    if (state.busy) {
      if (text) { state.queued.push(raw(text)); input.value=''; paintConversation(); remember(); return; }
      if (fromKeyboard) return;
      clearTimeout(replyTimer); state.busy=false; paintConversation(); return;
    }
    if (!text&&!state.files.length) return;
    closeModels(false); closeCommands();
    const files=state.files.length?' ('+state.files.join(', ')+')':'';
    state.messages.push({who:'you',text:raw((text||L('Here is a file'))+files)}); state.lastSent=text; state.busy=true;
    input.value=''; input.style.height='auto'; state.files=[]; paintFiles(); paintConversation();log.scrollTop=log.scrollHeight;
    const reply=commandReply(text);
    if (/\b(thanks|thank you|obrigad|gracias|valeu)/i.test(text)) { reply.text='Anytime. I am here.'; reply.tools=false; reply.thanks=true; }
    state.toolPhase=reply.tools!==false; state.liveTools=[]; paintBird();
    // Each tool call lands as it happens: first without a result, then with it.
    (Array.isArray(reply.tools)?reply.tools:[]).forEach((row,i)=>{
      const name=row.querySelector?.('code')?.textContent??'', at=name.indexOf('('), call={name:name.slice(0,at),args:name.slice(at+1,-1),result:null};
      window.setTimeout(()=>{ if (!state.busy) return; state.liveTools.push(call); paintConversation(); log.scrollTop=log.scrollHeight; },250+i*300);
      window.setTimeout(()=>{ if (!state.busy) return; call.result=row.querySelector?.('span')?.textContent??''; paintConversation(); },550+i*300);
    });
    window.setTimeout(()=>{ state.toolPhase=false; if (state.busy) paintBird(); },700+300*(Array.isArray(reply.tools)?reply.tools.length:0));
    replyTimer=window.setTimeout(() => {
      const top=log.scrollTop, follow=log.scrollHeight-log.scrollTop-log.clientHeight<40 && Date.now()-lastManualScroll>=150;
      state.busy=false; state.liveTools=[]; state.messages.push(reply); paintConversation();enter(log.lastElementChild,'gentle');log.scrollTop=follow?log.scrollHeight:top;
      zecaDo(reply.thanks?'dance':'speak',reply.thanks?2200:1600);
      if (state.queued.length) { input.value=state.queued.shift(); submit(); }
      remember();
    },1400);
    remember();
  }
  function commandReply(text) {
    const [name,...rest]=text.split(' '), arg=rest.join(' ').trim();
    if (name==='/continue') return {who:'zeca',text:'No stopped Claude or Codex session in the last 10 minutes.',tools:false};
    if (name==='/remember'&&arg) { memory.facts.push({id:'fact-'+Date.now(),place:settings.chatFolder,text:raw(arg)}); return {who:'zeca',text:phrase('Remembered for {place}.',{place:settings.chatFolder}),tools:false,link:{label:'Open Memory',action:() => openMemory()}}; }
    if (name==='/forget'&&arg) { const before=memory.facts.length; memory.facts=memory.facts.filter(f=>!L(f.text).toLowerCase().includes(arg.toLowerCase())); return {who:'zeca',text:before>memory.facts.length?'Forgotten.':'Nothing in memory matches that.',tools:false}; }
    if (/@\w+/.test(text)) { const s=sessions.find(s=>text.includes('@'+s.project)); if (s) return {who:'zeca',text:phrase('{project} ({agent}): {brief}.',{project:s.project,agent:labelFor(s.agent),brief:L(sessionBrief(s))}),tools:[toolRow('session_detail','"'+s.project+'"',raw(L(sessionBrief(s))))],seconds:'0.2'}; }
    const failed=sessions.find(s=>s.status==='failed'&&text.includes(s.project));
    if (failed) return {who:'zeca',text:phrase('{project} stopped at {command}: {output}',{project:failed.project,command:failed.step,output:failed.output.split('\n').pop()}),tools:[toolRow('session_detail','"'+failed.project+'"',phrase('failed at step {n}',{n:failed.step_count}))],seconds:'0.5',chips:[{id:'f-open',label:'Open its output',go:()=>{views.sessionSelected=failed.id;views.flockDetail='activity';setArea('flock','room');}}]};
    return {who:'zeca',text:'This prototype answers with sample data. In the app, Zeca reads the flock, your memory and the connectors he may use.',seconds:'0.4',memory:'fact-1',chips:[{id:'more',label:'Tell me more',send:'Tell me more'}]};
  }
  let noticeAnimation;
  function paintNotice() {
    const slot=$('.notice-slot'),shown=slot.getAttribute('data-open')==='true';
    if(shown===state.notice)return;
    const before=slot.hidden?0:slot.getBoundingClientRect().height;noticeAnimation?.cancel();slot.setAttribute('data-open',String(state.notice));slot.inert=!state.notice;
    if(!prefs.motion||restoring){slot.hidden=!state.notice;return;}
    slot.hidden=false;slot.inert=!state.notice;const height=state.notice?Math.min(80,slot.scrollHeight):0;
    noticeAnimation=slot.animate([{height:before+'px',opacity:shown?1:0},{height:height+'px',opacity:state.notice?1:0}],{duration:180,easing:'cubic-bezier(0.16,1,0.3,1)'});
    noticeAnimation.finished.then(()=>{slot.hidden=!state.notice;slot.inert=!state.notice;}).catch(()=>{});
  }
  $('.notice-close').append(A.icon('close',12)); $('.notice-close').setAttribute('aria-label',L('Dismiss'));
  $('.notice-close').addEventListener('click',() => { state.notice=false;paintNotice(); });
  $('.notice-open').addEventListener('click',() => { state.notice=false;paintNotice();views.sessionSelected=flockFacts().needs[0]?.id??views.sessionSelected;setArea('flock',state.mode==='quick'?'quick':'room'); });
  function toggleVoice() {
    if (!settings.voice) { toast('Voice is off in Settings.'); return; }
    state.voice=!state.voice; input.hidden=state.voice; $('.voice-preview').hidden=!state.voice; mic.setAttribute('aria-pressed',String(state.voice));
    $('.voice-partial').textContent=state.voice?L('Review the lay…'):'';
    if(!state.voice&&!input.value.trim()){input.value=L('Review the layout of the Now inbox');input.dispatchEvent(new Event('input'));}
    paintBird();remember();
  }
  let toastTimer;
  function toast(text) { const host=$('.feedback');host.textContent=L(text);host.hidden=false;clearTimeout(toastTimer);toastTimer=setTimeout(() => host.hidden=true,1800); }
  // Only a destructive action gets the danger style; a merge or a removal of sample data does not.
  function confirm(title,about,action,variant='default',label=title) {
    const overlay=$('.dialog-overlay'), dialog=$('.confirmation'), previous=document.activeElement;
    dialog.querySelector('h2').textContent=L(title); dialog.querySelector('p').textContent=L(about);
    const cancel=button('Cancel','quiet-control',() => close());
    const yes=button(label,variant==='danger'?'danger':'primary',() => { action();close(); });
    function close() { overlay.hidden=true;previous?.isConnected?previous.focus():$('.product-nav button')?.focus(); }
    $('.confirmation-actions').replaceChildren(cancel,yes); overlay.hidden=false; enter(dialog,'gentle',8,.98);cancel.focus();
    overlay.onkeydown=event => {
      if (event.isComposing||event.keyCode===229) return;
      if (event.key==='Escape') { event.preventDefault(); event.stopPropagation(); close(); }
      if (event.key==='Tab') { event.preventDefault(); (document.activeElement===cancel?yes:cancel).focus(); }
    };
  }
  root.addEventListener('keydown',event => {
    if (event.isComposing||event.keyCode===229||event.defaultPrevented) return;
    if (event.key==='Escape') {
      if (!actionMenu.hidden) { event.preventDefault();closeActionMenu(true); }
      else if (!popover.hidden) { event.preventDefault();closeModels(); }
      else if (!history.hidden) { event.preventDefault();closeMenu(history,historyTrigger,true); }
      else if (state.voice) { state.voice=false;input.hidden=false;$('.voice-preview').hidden=true;mic.setAttribute('aria-pressed','false');paintBird(); }
      else if (state.mode==='full') { event.preventDefault();setMode('room'); }
    }
  });
  function pageHeading(title,about,...actions) { return el('div','page-title',null,el('div',null,null,el('h2',null,title),about?el('p',null,about):null),actions.length?el('div','page-actions',null,...actions):null); }
  function search(label,action) {
    const field=el('input');field.type='search';field.placeholder=L(label);field.setAttribute('aria-label',L(label));field.addEventListener('input',() => action(field.value));
    return el('label','search-field',null,A.icon('search',14),field);
  }
  // Filters: a segmented control. Navigation between views uses tabs() instead (two patterns, no more).
  function segment(items,onChange,current) {
    const host=el('div','segment');host.setAttribute('role','group');
    items.forEach((item,i) => { const [value,label]=Array.isArray(item)?item:[item,item]; const b=button(label,'',() => { for (const node of host.children) node.setAttribute('aria-pressed',String(node===b));enter(b,'snappy',0,.96);onChange(value); });b.dataset.value=value;b.setAttribute('aria-pressed',String(current===undefined?i===0:current===value));host.append(b); });
    return host;
  }
  function tabs(items,current,onChange,label,cls='') {
    const host=el('nav','tabs '+cls);host.setAttribute('role','tablist');host.setAttribute('aria-label',L(label));
    items.forEach(([id,text,count]) => {
      const b=key(button(null,'',() => onChange(id,false)),'tab-'+id);b.setAttribute('role','tab');b.setAttribute('aria-selected',String(id===current));b.tabIndex=id===current?0:-1;
      b.append(el('span',null,text));if (count) b.append(el('span','tab-count counter',null,document.createTextNode(String(count))));host.append(b);
    });
    host.addEventListener('keydown',event => {
      if (event.isComposing||!['ArrowLeft','ArrowRight','Home','End'].includes(event.key)) return;
      event.preventDefault();const at=items.findIndex(([id])=>id===current);
      const next=event.key==='Home'?0:event.key==='End'?items.length-1:(at+(event.key==='ArrowRight'?1:-1)+items.length)%items.length;onChange(items[next][0],true);
    });
    return host;
  }
  function setting(title,about,control) { return el('div','setting-row',null,el('div','setting-copy',null,el('strong',null,title),about?el('small',null,about):null),control); }
  /* WORKSPACE_LOGIC */
  $$('[data-area]').forEach(b => b.addEventListener('click',() => { if (b.dataset.scenario) setScenarioQuiet(b.dataset.scenario); setArea(b.dataset.area,b.dataset.mode ?? 'room'); }));
  $$('[data-size]').forEach(b => b.addEventListener('click',() => setMode(b.dataset.size)));
  $('.scenario-select').addEventListener('change',() => setScenario($('.scenario-select').value));
  $('.language-select').addEventListener('change',() => setLanguage($('.language-select').value));
  $('.theme-select').addEventListener('change',() => { applyTheme($('.theme-select').value); remember(); });
  $('.viewport-select').addEventListener('change',() => {const stage=$('.review-stage');stage.classList.toggle('narrow',$('.viewport-select').value==='narrow');stage.classList.toggle('short',$('.viewport-select').value==='short');layoutSurface(true);resizeCompactFlock();paintBird();});
  $('.prototype-tools').addEventListener('toggle',()=>{layoutSurface();resizeCompactFlock();paintBird();});
  $$('[data-example]').forEach(b => b.addEventListener('click',() => { if (state.area!=='chat') setArea('chat',state.mode==='mini'?'quick':state.mode); showExample(b.dataset.example); }));
  window.addEventListener('openai:set_globals',event => {
    const saved=event.detail?.globals?.widgetState?.modelContent;
    if (saved?.area && AREAS.includes(saved.area)) {
      restoring=true;restorePrototype(event.detail.globals.widgetState);setArea(state.area,state.mode);restoring=false;
    }
  });
  applyTheme(settings.theme);
  restorePrototype(original);paintFiles();paintConversation();setArea(state.area,state.mode);
  restoring=false;
  // verify.mjs drives the same entry points a reviewer clicks.
  window.__vultsPrototype={setArea,setMode,setScenario,showExample,setLanguage,missing:() => [...missing],state,sessions,views,settings,dictation:dx};
})();
