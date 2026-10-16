  /* Lab · Dictation (plan-dictation V0.7). Hold the talk key in any app, speak, let go: whisper.cpp transcribes on this
     computer, your words and snippets fix the text, and Vults pastes it where the cursor is. The area has its own side
     list, in three groups (Voice, Your words, System), and owns its settings; Settings › Chat & voice links to it. The
     island shows dictation over the flock, in grey, and a card always wins.
     Voice commands (V8) use their own key: understood on this computer with no model, they move around the flock and
     read what Vults already knows; they never type into an app and never answer a card. */
  const DX_PAGES=[['history','Dictation','mic','Voice'],['commands','Voice commands','flock','Voice'],['talk','Talk with Zeca','chat','Voice'],['meetings','Meetings','person','Voice'],['stats','Statistics','bars','Voice'],
    ['dictionary','Dictionary','file','Your words'],['snippets','Snippets','tag','Your words'],['styles','Styles','pencil','Your words'],['settings','Settings','gear','System'],['diagnostics','Diagnostics','list','System']];
  const DX_PHASE={history:'V1',commands:'V8',talk:'V6',meetings:'V5',stats:'V4',dictionary:'V2',snippets:'V2',styles:'V3',settings:'V1',diagnostics:'V4'};
  const dx={tab:'history',query:'',island:'idle',partial:'',caption:null,command:0,meeting:'m1',meetingTab:'transcript',recording:false,fresh:null,timers:[]};
  const dxWords=['Vults','Zeca','Codex','KWin','Wayland','Tauri','whisper.cpp'].map(raw);
  // Spoken forms that stay the same in every language, so the sample works in all four.
  const dxReplacements=[{from:'cloud code',to:'Claude Code'},{from:'pull request',to:'PR'},{from:'k win',to:'KWin'}].map(r=>({from:raw(r.from),to:raw(r.to)}));
  const dxSnippets=[{trigger:'my email',text:raw('you@example.com')},{trigger:'signature',text:'Cheers,\nAna'}];
  const DX_STYLES={verbatim:['As spoken','No model: only your words and snippets.'],natural:['Natural','Drops “uh”, “like” and repeats; keeps your voice.'],polished:['Polished','Full sentences and written punctuation.'],prompt:['Prompt','A clear instruction for the agent. It never answers or runs what you said.']};
  const dxApps=[['Sessions','prompt'],['Konsole','verbatim'],['VS Code','verbatim'],['Thunderbird','polished'],['Firefox','natural']].map(([app,style])=>({app:app==='Sessions'?app:raw(app),style}));
  const dxHistory=[
    {day:'Today',time:'15:10',app:'Codex',session:'api',heard:'uh, fix the failing parser test, like, only that one, and run cargo test again',secs:8,out:raw('Fix the failing parser test (only that one), then run cargo test again.'),translated:'English'},
    {day:'Today',time:'14:51',command:true,heard:'who is waiting on a card',result:'Two sessions wait on a card: api and web.',secs:2},
    {day:'Today',time:'14:32',app:'Firefox',heard:'open a pull request on cloud code and ask zeca to review it',secs:6},
    {day:'Today',time:'14:05',app:'Konsole',heard:'git commit dash m fix k win focus on wayland',secs:4},
    {day:'Today',time:'11:48',app:'Thunderbird',heard:'hi team, the tauri build passed, here is the link. signature',secs:9},
    {day:'Yesterday',time:'18:20',app:'Slack',heard:'codex finished the task, I will look at it tomorrow morning',secs:5},
    {day:'Yesterday',time:'09:41',app:'Firefox',heard:'my email',secs:1},
  ].map(h=>({...h,app:h.app&&raw(h.app)}));
  /* What Try a command plays, in order. The names resolve against the flock and Your words; one or two letters off
     still count, and when two fit it asks. "And X too" and "undo" reuse the last command for 90 s. */
  const DX_COMMANDS=[
    {heard:'open the api session',state:'commanded',result:'Opened api · Codex'},
    {heard:'who is waiting on a card',state:'answered',result:'Two sessions wait on a card: api and web.'},
    {heard:'approve it',state:'refused',result:'Cards are answered with a click or a shortcut, not by voice.'},
    {heard:'mute web',state:'commanded',result:'Muted web'},
    {heard:'and docs too',state:'commanded',result:'Muted docs, like web'},
    {heard:'undo',state:'commanded',result:'Unmuted docs'},
    {heard:'open the folder of apy',state:'asked',result:'Which one: api or api-gateway?'},
    {heard:'the gateway one',state:'commanded',result:'Opened the api-gateway folder'},
    {heard:'write a poem about birds',state:'unknown',result:'Not a command. Try “next session” or “who is waiting?”.'},
  ];
  const dxMeetings=[
    {id:'m1',title:'Vults sync',when:'Today · 10:00 · 32 min',speakers:['You','Person 2','Person 3'],
      lines:[[0,'The idea is to close dictation this week.'],[1,'Is whisper Small good enough in Portuguese?'],[0,'Yes. Turbo is better, but only with a GPU.'],[2,'And on Wayland, how is the text pasted?'],[0,'Through the RemoteDesktop portal; KDE asks once.']],
      summary:'Dictation closes this week with whisper Small as the default. On Wayland, pasting goes through the RemoteDesktop portal.',
      todos:[['Compare Small and Turbo on 20 sentences','You','Fri'],['Test the portal on KWin 6','Person 3','—']]},
    {id:'m2',title:'User interview',when:'Yesterday · 15:30 · 48 min',speakers:['You','Person 2'],
      lines:[[1,'I dictate more in the terminal than in the editor.'],[0,'And does Ctrl+Shift+V work there on its own?']],
      summary:'They dictate mostly in the terminal; pasting with Ctrl+Shift+V is essential.',todos:[['Detect terminals from the focused app','You','—']]},
  ];
  const DX_DAYS=[820,1240,460,0,1580,1720,940,1310,2010,620,0,1450,1890,1130];
  const DX_APPS=[['Firefox',38],['Konsole',24],['VS Code',17],['Thunderbird',12],['Slack',9]];

  /* The text pass: snippets, then replacements, then the dictionary's spelling. Whole words, any case, a space or a
     hyphen between parts. It returns segments, so the page can show what changed and why. */
  const dxEscape=s=>s.replace(/[.*+?^${}()|[\]\\]/g,'\\$&').replace(/[\s-]+/g,'[\\s-]+');
  function dxRules(){
    return [...dxSnippets.map(s=>({from:L(s.trigger),to:L(s.text),kind:'snippet',why:phrase('Snippet: “{trigger}”',{trigger:L(s.trigger)})})),
      ...dxReplacements.map(r=>({from:r.from,to:r.to,kind:'replace',why:phrase('Replacement: “{from}”',{from:r.from})})),
      ...dxWords.map(w=>({from:w,to:w,kind:'word',why:L('Your words')}))];
  }
  function dxApply(text){
    let segs=[{text}];
    for(const r of dxRules()){
      const re=new RegExp('(?<![\\p{L}\\p{N}])'+dxEscape(r.from)+'(?![\\p{L}\\p{N}])','giu');
      segs=segs.flatMap(s=>{
        if(s.why)return [s];
        const out=[];let last=0;
        for(const m of s.text.matchAll(re)){if(r.kind==='word'&&m[0]===r.to)continue;out.push({text:s.text.slice(last,m.index)},{text:r.to,why:r.why,kind:r.kind,was:m[0]});last=m.index+m[0].length;}
        out.push({text:s.text.slice(last)});return out.filter(x=>x.text);
      });
    }
    return segs;
  }
  // What a clean-up would do, faked for the sample: a capital letter and a full stop. Off by default (a request on your behalf).
  function dxTidy(segs,style){
    if(style==='verbatim')return segs;
    const out=segs.map(s=>({...s}));
    if(out[0]&&!out[0].why)out[0].text=out[0].text.replace(/^\s*(\p{L})/u,c=>c.toUpperCase());
    const end=out.at(-1);
    if(end&&!end.why&&!/[.!?。]\s*$/.test(end.text))end.text=end.text.replace(/\s*$/,'.');
    return out;
  }
  // Without the clean-up every line is as spoken; the sample's English prompt exists only with it on.
  const dxStyleOf=h=>!settings.dictCleanup?'verbatim':h.session?'prompt':dxApps.find(a=>a.app===h.app)?.style??(Object.entries(DX_STYLES).find(([,v])=>v[0]===settings.dictStyle)?.[0]??'natural');
  const dxFinal=h=>h.out&&settings.dictCleanup?dxApply(h.out):dxTidy(dxApply(L(h.heard)),dxStyleOf(h));
  const dxPlain=segs=>segs.map(s=>s.text).join('');
  const dxWordCount=s=>s.trim().split(/\s+/).filter(Boolean).length;
  function dxRender(segs){
    const p=el('p','dx-text');
    for(const s of segs){
      if(!s.why){p.append(document.createTextNode(s.text));continue;}
      const span=el('span','dx-hit',null,document.createTextNode(s.text));span.setAttribute('data-tooltip',s.why+(s.kind==='snippet'?'':' · '+phrase('heard: “{text}”',{text:s.was})));p.append(span);
    }
    return p;
  }
  const dxPhase=(node,phase)=>{node.querySelector('h2')?.append(el('span','dx-phase',null,document.createTextNode(phase)));return node;};
  const dxTile=(value,label)=>el('div','stat-tile',null,el('strong','counter',null,document.createTextNode(value)),el('small',null,label));

  const dxOpen=page=>{dx.tab=page;setArea('dictation',state.mode==='full'?'full':'room');};
  // Each page: its title, a line about it and its actions, then its body.
  const DX_HEAD={
    history:()=>['Dictation','Hold Ctrl+Alt+V in any app, speak and let go. The text goes where the cursor is.',key(button('Try it','quiet-control',()=>dxSimulate()),'dictation-try')],
    commands:()=>['Voice commands','Hold the command key, say what to do in Vults and let go. Understood on this computer with no model; it never types into an app.',...(settings.dictCommands?[key(button('Try a command','quiet-control',()=>dxCommand()),'dictation-try-command')]:[])],
    talk:()=>['Talk with Zeca','Off by default. On, your voice goes to the realtime model you choose, with your key in the keyring, a daily budget and a trace. Dictation stays on this computer.',...(settings.zecaVoice?[key(button('Start a conversation','quiet-control',()=>dxIsland('speaking')),'zeca-talk')]:[])],
    meetings:()=>['Meetings','Records your microphone and the computer\'s sound. No bot joins the call; the audio is deleted after the transcript.',dx.recording?key(button('Stop and transcribe','quiet-control',()=>{dx.recording=false;toast('Transcribing…');dxOpen('meetings');}),'meeting-stop'):key(button('Record a call','quiet-control',()=>{dx.recording=true;dxOpen('meetings');}),'meeting-record')],
    stats:()=>['Statistics','Only this computer. Erasing the history keeps the figures; a day with no dictation counts as unknown, not zero.'],
    dictionary:()=>['Dictionary','Names and terms Vults always spells right.'],
    snippets:()=>['Snippets','Say the short phrase and Vults pastes the whole text, unchanged.'],
    styles:()=>['Styles','An optional clean-up by a model, set per app. It is a filter, never an assistant.'],
    settings:()=>['Settings',null],
    diagnostics:()=>['Diagnostics',null,button('Test the microphone','quiet-control',()=>toast('Good: clear speech, low noise (−52 dB).'))],
  };
  function renderDictation(host){
    host.classList.add('dx-view');
    const nav=el('nav','settings-categories dx-pages'),page=el('div','settings-content dx-page');nav.setAttribute('aria-label',L('Dictation'));let last;
    for(const [id,label,glyph,group] of DX_PAGES){
      if(group!==last){nav.append(el('small',null,group));last=group;}
      const b=key(button(null,'',()=>dxOpen(id)),'dx-page-'+id);b.append(A.icon(glyph,14),el('span',null,label));
      if(id==='meetings'&&dx.recording)b.append(el('span','shape shape-working nav-count'));
      b.setAttribute('aria-pressed',String(dx.tab===id));nav.append(b);
    }
    host.append(el('div','settings-grid dx-grid',null,nav,page));
    if(stageState(page,'Nothing dictated yet.'))return;
    const [title,about,...actions]=DX_HEAD[dx.tab]();page.append(dxPhase(pageHeading(title,about,...actions),DX_PHASE[dx.tab]));
    ({history:dxHistoryPage,commands:dxCommandsPage,talk:dxTalkPage,meetings:dxMeetingsPage,stats:dxStatsPage,dictionary:dxWordsPage,snippets:dxSnippetsPage,styles:dxStylesPage,settings:dxSettingsPage,diagnostics:dxDiagnosticsPage})[dx.tab](page);
  }
  // A voice command in a list: what it did, what was heard on demand.
  // Where a line went, at a glance: a command, a flock session, or the app's first letter.
  function dxMark(h){
    const m=el('span','dx-mark'+(h.command?' command':h.session?' session':''));m.setAttribute('aria-hidden','true');
    if(h.command)m.append(A.icon('flock',14));else if(h.session)m.append(A.icon('terminal',14));else m.append(document.createTextNode(String(L(h.app)).slice(0,1).toUpperCase()));
    return m;
  }
  function dxCommandLine(h,refill){
    const heard=el('p','dx-heard',null,document.createTextNode(phrase('Heard: “{text}”',{text:L(h.heard)})));heard.hidden=true;
    const meta=el('div','dx-meta',null,el('time','counter',null,document.createTextNode(L(h.time))),el('span','dx-where',null,A.icon('flock',12),el('span',null,'Voice command')),el('span','counter',null,document.createTextNode(phrase('{n} s',{n:h.secs}))));
    return key(el('article','dx-entry'+(h===dx.fresh?' dx-new':''),null,dxMark(h),el('div',null,null,el('p','dx-text',null,document.createTextNode(L(h.result))),heard,meta),
      el('div','dx-actions',null,iconButton('hide','Show what was heard',()=>{heard.hidden=!heard.hidden;}),iconButton('close','Delete',()=>{dxHistory.splice(dxHistory.indexOf(h),1);toast('Deleted');refill();}))),'dictation-'+dxHistory.indexOf(h));
  }
  function dxHistoryPage(host){
    if(settings.dictKeep==='0'){host.append(el('section','empty-state',null,el('h3',null,'History is off'),el('p',null,'Only the last text stays in memory, for Paste again. Statistics still count.'),button('Dictation settings','quiet-control',()=>dxOpen('settings'))));return;}
    const today=dxHistory.filter(h=>h.day==='Today'&&!h.command).reduce((n,h)=>n+dxWordCount(dxPlain(dxFinal(h))),0)+1188;
    // The page's signature: the key itself, a voice line, and today's figures beside it.
    const wave=el('span','dx-wave');for(let i=0;i<36;i++){const b=el('i');b.style.height=(18+Math.round(Math.abs(Math.sin(i*1.7)*Math.cos(i*.45))*70))+'%';wave.append(b);}
    host.append(el('section','dx-hero',null,
      el('div','dx-hero-key',null,el('div','dx-keycaps',null,...['Ctrl','Alt','V'].map(k=>el('kbd',null,null,document.createTextNode(k)))),el('p',null,'In any app: the editor, the terminal, the browser, or straight into a flock session.'),wave),
      el('div','dx-hero-stats',null,dxTile(today.toLocaleString(language),'words today'),dxTile(phrase('{n} min',{n:14}),'saved today'),dxTile('152','words per minute'))));
    const finder=search('Search what you dictated',value=>{dx.query=value;fill();});finder.querySelector('input').value=dx.query;
    const kept=el('span','status',null,el('span','shape shape-quiet'),el('span',null,{'7':'Kept 7 days','30':'Kept 30 days','365':'Kept 1 year',all:'Kept always'}[settings.dictKeep]));
    const list=el('div','dx-list');host.append(el('div','workspace-toolbar dx-toolbar',null,finder,kept),list);fill();
    function fill(){
      list.replaceChildren();const q=dx.query.trim().toLowerCase();
      const rows=dxHistory.filter(h=>!q||(L(h.heard)+' '+(h.command?L(h.result):dxPlain(dxFinal(h)))).toLowerCase().includes(q));
      if(!rows.length){list.append(el('p','pane-note',q?'Nothing here matches your search.':'Nothing dictated yet.'));return;}
      let day;
      for(const h of rows){
        if(h.day!==day){day=h.day;list.append(el('small','menu-group',day));}
        if(h.command){list.append(dxCommandLine(h,fill));continue;}
        const segs=dxFinal(h),fixes=segs.filter(s=>s.why).length,style=dxStyleOf(h);
        const heard=el('p','dx-heard',null,document.createTextNode(phrase('Heard: “{text}”',{text:L(h.heard)})));heard.hidden=true;
        const where=h.session?el('span','dx-where',null,A.icon('flock',12),el('span',null,null,document.createTextNode(phrase('In session {session} · {app}',{session:h.session,app:h.app})))):el('span','dx-where',null,A.icon('openOut',12),el('span',null,null,document.createTextNode(phrase('Pasted in {app}',{app:h.app}))));
        const meta=el('div','dx-meta',null,el('time','counter',null,document.createTextNode(L(h.time))),where,el('span',null,null,document.createTextNode(L(DX_STYLES[style][0])+(h.translated&&settings.dictCleanup?' · '+phrase('in {language}',{language:L(h.translated)}):''))),el('span','counter',null,document.createTextNode(phrase('{n} s',{n:h.secs}))),fixes?el('span',null,null,document.createTextNode(phrase(fixes===1?'1 fix':'{n} fixes',{n:fixes}))):null);
        const actions=el('div','dx-actions',null,
          iconButton('retry','Paste again (Ctrl+Alt+B)',()=>toast('Pasted again in the focused app')),
          iconButton('hide','Show what was heard',()=>{heard.hidden=!heard.hidden;}),
          iconButton('copy','Copy',()=>toast('Copied')),
          iconButton('close','Delete',()=>{dxHistory.splice(dxHistory.indexOf(h),1);toast('Deleted');fill();}));
        list.append(key(el('article','dx-entry'+(h===dx.fresh?' dx-new':''),null,dxMark(h),el('div',null,null,dxRender(segs),heard,meta),actions),'dictation-'+dxHistory.indexOf(h)));
      }
      dx.fresh=null;
    }
  }
  function dxWordsPage(host){
    const chips=el('div','dx-chips');
    for(const w of dxWords)chips.append(el('span','dx-chip',null,el('span',null,null,document.createTextNode(w)),iconButton('close',phrase('Remove {word}',{word:w}),()=>{dxWords.splice(dxWords.indexOf(w),1);setArea('dictation',state.mode);})));
    host.append(settingsGroup(phrase('Words · {n}',{n:dxWords.length}),chips,dxAddRow('New word, e.g. Kubernetes',v=>{if(dxWords.some(w=>w.toLowerCase()===v.toLowerCase()))throw new Error('That word is already there.');dxWords.push(raw(v));})));
    const from=field('When I say',''),to=field('Write','');
    host.append(settingsGroup(phrase('Replacements · {n}',{n:dxReplacements.length}),el('p','pane-note','They also fix the names in voice commands.'),dxPairs(dxReplacements.map(r=>[r.from,r.to]),['When I say','Write'],i=>dxReplacements.splice(i,1)),
      el('div','dx-add',null,from.element,to.element,key(button('Add','quiet-control',()=>{if(!from.control.value.trim()||!to.control.value.trim()){toast('Fill in both sides.');return;}dxReplacements.push({from:raw(from.control.value.trim()),to:raw(to.control.value.trim())});setArea('dictation',state.mode);toast('Added');}),'add-replacement'))));
    host.append(settingsGroup('How it is applied',switchRow('dictHint','Help the model with these words','Your first words go to whisper as vocabulary. Only with Small, where it was measured to help.'),switchRow('dictProject','Project vocabulary',"Dictating into a session, or into a terminal or editor whose window title names a project, adds its file, crate and branch names."),el('p','pane-note','Whole words, any case; a space and a hyphen count the same. Nothing is learned on its own.')));
    const box=field('Try a sentence',L('open a pull request on cloud code and send it to my email'),'textarea'),out=el('div','dx-try-out');
    const run=()=>out.replaceChildren(el('small',null,'Becomes, before any clean-up:'),dxRender(dxApply(box.control.value)));
    box.control.addEventListener('input',run);run();host.append(key(el('section','dx-try',null,box.element,out),'dictation-try-sentence'));
  }
  function dxPairs(rows,heads,remove){const t=el('table','record-table dx-pairs'),b=el('tbody');t.append(el('thead',null,null,el('tr',null,null,...heads.map(h=>el('th',null,h)),el('th'))),b);rows.forEach(([a,c],i)=>b.append(el('tr',null,null,el('td',null,null,document.createTextNode(a)),el('td','dx-snippet',null,document.createTextNode(c)),el('td',null,null,iconButton('close','Delete',()=>{remove(i);setArea('dictation',state.mode);})))));return t;}
  function dxSnippetsPage(host){
    const trigger=field('When I say',''),text=field('Paste','','textarea');
    host.append(settingsGroup(phrase('Snippets · {n}',{n:dxSnippets.length}),dxPairs(dxSnippets.map(s=>[L(s.trigger),L(s.text)]),['When I say','Paste'],i=>dxSnippets.splice(i,1)),
      el('div','dx-add',null,trigger.element,text.element,key(button('Add','quiet-control',()=>{if(!trigger.control.value.trim()||!text.control.value.trim()){toast('Fill in both sides.');return;}dxSnippets.push({trigger:raw(trigger.control.value.trim()),text:raw(text.control.value)});setArea('dictation',state.mode);toast('Added');}),'add-snippet'))));
  }
  function dxAddRow(placeholder,onAdd){
    const f=el('input');f.placeholder=L(placeholder);f.setAttribute('aria-label',L(placeholder));
    const add=()=>{const v=f.value.trim();if(!v)return;try{onAdd(v);setArea('dictation',state.mode);toast('Added');}catch(error){toast(error.message);}};
    f.addEventListener('keydown',event=>{if(event.key==='Enter')add();});
    return el('div','dx-add',null,f,key(button('Add','quiet-control',add),'add-word'));
  }
  function dxMeetingsPage(host){
    if(dx.recording)host.append(el('div','dx-recording',null,el('span','shape shape-working'),el('span','counter',null,document.createTextNode('00:42')),el('p',null,'Recording. Tell the people on the call that you are recording.')));
    const list=el('div','dx-meeting-list'),detail=el('article','dx-meeting');
    for(const m of dxMeetings){const b=key(button(null,'connection-row',()=>{dx.meeting=m.id;setArea('dictation',state.mode);}),'meeting-'+m.id);b.setAttribute('aria-pressed',String(m.id===dx.meeting));b.append(el('div','connection-copy',null,el('strong',null,m.title),el('small',null,m.when)));list.append(b);}
    const m=dxMeetings.find(x=>x.id===dx.meeting)??dxMeetings[0];
    const exporter=choose('Export',[{value:'',label:'Export…'},{value:'md',label:'Markdown'},{value:'srt',label:raw('SRT')},{value:'vtt',label:raw('VTT')}],'',value=>{if(value)toast('Exported');exporter.value='';});
    detail.append(el('h3',null,m.title),el('span','task-meta',null,document.createTextNode([L(m.when),...m.speakers.map(s=>L(s))].join(' · '))),el('div','detail-actions',null,iconButton('copy','Copy as Markdown',()=>toast('Copied')),exporter),
      tabs([['transcript','Transcript'],['summary','Summary'],['todos','To-dos']],dx.meetingTab,id=>{dx.meetingTab=id;setArea('dictation',state.mode);},'Meeting'));
    if(dx.meetingTab==='transcript')for(const [who,line] of m.lines){
      const name=button(m.speakers[who],'dx-speaker',()=>{const f=field('Name',L(m.speakers[who]));openEditor('Rename speaker',f.element,()=>{if(!f.control.value.trim())throw new Error('Enter a name first.');m.speakers[who]=raw(f.control.value.trim());setArea('dictation',state.mode);toast('Renamed in the whole meeting');return true;});},'Rename speaker');
      detail.append(el('div','dx-line',null,name,el('span',null,line)));
    }
    if(dx.meetingTab==='summary')detail.append(settings.dictSummary?el('p',null,m.summary):el('section','empty-state',null,el('h3',null,'Summaries are off'),el('p',null,"They use the chat's model through the network gate, with a budget and a trace."),button('Dictation settings','quiet-control',()=>dxOpen('settings'))));
    if(dx.meetingTab==='todos')for(const [text,owner,due] of m.todos)detail.append(el('div','dx-todo',null,el('span','shape shape-quiet'),el('span',null,text),el('small','task-meta',null,document.createTextNode(L(owner)+' · '+L(due)))));
    host.append(el('div','dx-meetings',null,list,detail));
  }
  function dxStatsPage(host){
    host.append(el('div','stat-tiles',null,dxTile((48920).toLocaleString(language),'words dictated'),dxTile(phrase('{h} h {m} min',{h:5,m:26}),'saved'),dxTile('152','words per minute')));
    const max=Math.max(...DX_DAYS),chart=el('div','dx-chart');chart.setAttribute('role','img');chart.setAttribute('aria-label',L('Words per day, last 2 weeks'));
    DX_DAYS.forEach((n,i)=>{const bar=el('span',i===DX_DAYS.length-1?'today':null);bar.style.height=(n/max*100)+'%';bar.setAttribute('data-tooltip',phrase('{n} words',{n:n.toLocaleString(language)}));chart.append(el('div',null,null,bar,el('small',null,i===DX_DAYS.length-1?'today':null,...(i===DX_DAYS.length-1?[]:[document.createTextNode(String(3+i))]))));});
    const apps=el('div','dx-apps');for(const [app,pct] of DX_APPS){const bar=el('i');bar.style.width=pct*2.4+'%';apps.append(el('div',null,null,el('span',null,null,document.createTextNode(app)),el('span',null,null,bar),el('small','counter',null,document.createTextNode(pct+'%'))));}
    host.append(el('h3','lab-panel-title','Words per day, last 2 weeks'),chart,el('h3','lab-panel-title','Where you dictate'),apps);
  }

  /* The area owns its settings (R6), page by page; the voice model stays in Settings › Chat & voice, shared with the chat. */
  const dxKeys=(...names)=>el('span','dx-keys',null,...names.map(kbd));
  function dxSettingsPage(host){
    host.append(settingsGroup('Key',key(setting('Dictation key','Through the desktop\'s shortcuts portal: KDE asks once and lets you change it. This is the key it saved.',el('div','control-group',null,dxKeys('Ctrl','Alt','V'),button('Change in KDE…','quiet-control',()=>toast('Opens System Settings › Shortcuts › Vults')))),'dict-key'),
        optionRow('dictMode','Mode','Hold to talk. Two quick taps lock hands-free; one more tap finishes.',['Hold + double tap','Hold only']),
        switchRow('dictSpace','Space locks','Started holding and it got long? Tap Space without letting go: hands-free. X11 only for now.'),
        setting('Cancel','Esc on X11. On Wayland, Ctrl+Alt+X: binding Esc would take it from every app.',dxKeys('Ctrl','Alt','X')),
        key(switchRow('dictSayCancel','Cancel by voice','End with “cancel” or “never mind” and nothing is pasted. The same on X11 and Wayland.'),'dict-say-cancel'),
        setting('Paste again','Pastes the last dictation again, where the cursor is.',dxKeys('Ctrl','Alt','B'))),
      settingsGroup('Agents',key(switchRow('dictSession','Dictate into the session','When the focus is on a flock session\'s terminal, the text goes through Vults, the same on X11 and Wayland. Never into a session waiting on a card: then it stays in the clipboard and the island points to the card.'),'dict-session'),
        switchRow('dictAutoSend','Send by itself','Presses Enter after the text. Never while the session waits on a card.'),
        switchRow('dictSpokenCode','Spoken code','“dot”, “slash”, “dash” become . / - in terminals and sessions; no forced full stop.'),
        key(switchRow('dictAgentsAsk','Agents may ask for a spoken answer','Through Vults\' MCP: the island shows the question, the mic opens only on the key, and you send the text with a click. Never a permission.'),'dict-agents-ask')),
      settingsGroup('Voice',optionRow('dictMic','Microphone',null,['System default','Blue Yeti','Webcam C920']),
        setting('Voice model','Shared with the chat. Small is good in Portuguese; Turbo is the most accurate, best with a GPU.',el('div','control-group',null,el('span','counter',null,document.createTextNode(L(settings.voiceModel))),button('Chat & voice','quiet-control',()=>openSettings('chat')))),
        switchRow('dictSound','Sound when it starts','A short tone when the island starts listening.'),
        key(optionRow('dictDuck','Music while you speak','Players that speak MPRIS (Spotify, Firefox, VLC) come back as they were. Not while a meeting records.',['Lower to 25%','Pause it','Leave it']),'dict-duck'),
        key(switchRow('dictCaption','Caption under the island','The whole line in two rows while you speak and 2 s after; no buttons, and clicks go through it.'),'dict-caption'),
        switchRow('dictRepeats','Cut repeats','Whisper sometimes loops a phrase or invents “Thank you.” in silence; Vults cuts it before pasting.'),
        switchRow('dictPlanB','Plan B','Speech heard but no text: try again with Base and say so on the island.')),
      settingsGroup('Paste',switchRow('dictTerminals','Terminals use Ctrl+Shift+V','Konsole, kitty, Alacritty, WezTerm and the VS Code terminal.'),
        switchRow('dictClipboard','Give the clipboard back','What you had copied comes back 0.7 s later, only if nothing else changed it. Dictation stays out of Klipper.'),
        setting('No text field','The text stays in the clipboard and the island says “Ctrl+V to paste”.',null)),
      settingsGroup('Meetings',switchRow('dictSummary','Summary and to-dos','After each meeting, through the chat\'s model and the network gate. The transcript itself stays on this computer.')),
      settingsGroup('Privacy',optionRow('dictKeep','Keep the history','Audio is never saved; only the text, on this computer.',[{value:'0',label:'Do not keep'},{value:'7',label:'7 days'},{value:'30',label:'30 days'},{value:'365',label:'1 year'},{value:'all',label:'Always'}]),
        setting('Erase the history','Statistics stay.',button('Erase…','',()=>confirm('Erase the history','Every dictation on this computer is erased. Statistics stay.',()=>{dxHistory.length=0;toast('History erased');},'danger','Erase')))));
  }
  function dxCommandsPage(host){
    host.append(settingsGroup('Voice commands',key(switchRow('dictCommands','Voice commands','Hold the command key, say what to do in Vults and let go. Understood on this computer with no model; it never types into an app.',()=>dxOpen(dx.tab)),'dict-commands'),
        ...(settings.dictCommands?[setting('Command key','A second key in the shortcuts portal, so a command never ends up pasted as text.',el('div','control-group',null,dxKeys('Ctrl','Alt','Z'),button('Change in KDE…','quiet-control',()=>toast('Opens System Settings › Shortcuts › Vults')))),
          setting('Names it understands',"Your sessions' projects and agents, and the replacements in Your words. One or two letters off still counts; when two names fit, it asks which.",null),
          optionRow('dictCommandElse','When it is not a command',"Sent to Zeca, it is answered through the chat's model, as if you had typed it.",['Say so on the island','Send it to Zeca as a message']),
          key(more('What you can say',[
            setting('Move around','“next session”, “previous”, “open api”, “go to the api terminal”, “open the web folder”',null),
            setting('Ask','“who is waiting?”, “what is api doing?”, “how much of my plan is left?”, “how are my pull requests?” Answered under the island from what Vults already knows.',null),
            setting('Quiet a project','“mute web”, “pin api”, “hide docs”',null),
            setting('Again and undo','“and docs too”, “same for web”, “undo”. Vults remembers the last command for 90 s.',null),
            setting('Never','“approve”, “allow”, “yes” to a card: cards are answered with a click or a shortcut.',null)]),'dict-commands-list')]:[])));
    const list=el('div','dx-list');host.append(el('h3','lab-panel-title','Recent commands'),list);
    const fill=()=>{list.replaceChildren();const rows=dxHistory.filter(h=>h.command);for(const h of rows)list.append(dxCommandLine(h,fill));if(!rows.length)list.append(el('p','pane-note','Nothing here yet.'));dx.fresh=null;};fill();
  }
  function dxStylesPage(host){
    host.append(settingsGroup('Clean-up',key(switchRow('dictCleanup','Clean up with a model','A request on your behalf, through the network gate with its own budget and trace. Off, only your words and snippets apply.',()=>dxOpen(dx.tab)),'dict-cleanup'),
        ...(settings.dictCleanup?[key(optionRow('dictCleanupModel','Clean-up model','Local stays on this computer. A filter, never an assistant: a question is pasted as a question.',['Chat & voice · claude-haiku-4-5','Local · Ollama · qwen3:1.7b']),'dict-cleanup-model'),
          optionRow('dictStyle','Default style',null,Object.values(DX_STYLES).map(([label])=>label)),
          optionRow('dictWriteIn','Write in','Speak one language, paste another. Code, commands and your words stay as they are.',['The spoken language','English','Espa\u00f1ol']),
          switchRow('dictSessionsEnglish','Agent sessions in English','Only what goes to the flock\'s sessions.'),
          more('Style per app',dxApps.map(a=>setting(a.app,null,choose(phrase('Style for {app}',{app:L(a.app)}),Object.entries(DX_STYLES).map(([value,[label]])=>({value,label})),a.style,value=>{a.style=value;toast('Saved');}))))]:[])));
    const cards=el('div','dx-styles');
    for(const [id,[name,about]] of Object.entries(DX_STYLES)){const b=key(button(null,'dx-style',()=>{changedSetting('dictStyle',name);dxOpen('styles');}),'dx-style-'+id);b.append(el('strong',null,name),el('small',null,about));b.setAttribute('aria-pressed',String(settings.dictStyle===name));cards.append(b);}
    host.append(settingsGroup('Default style',...(settings.dictCleanup?[]:[el('p','pane-note','Off: every line is pasted as spoken.')]),cards));
  }
  function dxDiagnosticsPage(host){
    host.append(key(settingsGroup('Last 50 dictations',...['Release to pasted: 0.9 s typical, 1.8 s slow','Transcription: whisper Small on Vulkan, 0.55 s','Paste: RemoteDesktop portal, 45 ms','This week: 4 repeats cut, Plan B used once'].map(t=>setting(t,null,null))),'dict-diagnostics'),
      settingsGroup('This computer',setting('Wayland · KWin 6 · shortcuts and paste portals ready',null,chip('done','Ready'))));
  }
  /* Talking with Zeca (K8, V6). Off by default; on, audio leaves this computer. */
  function dxTalkPage(host){
    host.append(settingsGroup("Zeca's voice",key(switchRow('zecaVoice','Talk with Zeca',null,()=>dxOpen('talk')),'zeca-voice'),
      ...(settings.zecaVoice?[optionRow('zecaVoiceModel','Realtime model',null,['OpenAI · gpt-realtime','Google · Gemini Live']),optionRow('zecaBudget','Budget per day','Past it, Zeca answers in text and says so.',['$0.50','$1','$3']),
        optionRow('zecaTurn','When you are done','Fast answers after 1 s of silence; Patient waits out sentences that sound unfinished.',['Fast','Balanced','Patient']),
        switchRow('zecaBarge','Interrupt by speaking','Speak over him and he stops. Needs headphones or PipeWire\'s echo cancellation.'),
        switchRow('zecaFollowUp','Answer without the key','After Zeca asks you something, the mic opens for 8 s, shown on the island.')]:[]),
      setting('Never answers a card','Permissions and agents\' questions are answered with a click or a shortcut, never by voice.',chip('quiet','Always'))));
  }

  /* The island while dictating: a grey line over the flock. The birds stay underneath and come back when it ends; the
     amber edge of a card still shows (D5). Colors only on the badge shapes. */
  const DX_ISLAND=[['idle','Flock'],['listening','Listening'],['locked','Hands-free'],['working','Transcribing'],['done','Pasted'],['session','In a session'],['held','Held for a card'],['clipboard','Copied'],['retry','Plan B'],['failed','No sound'],['cancelled','Cancelled'],['speaking','Zeca speaks'],['interrupted','Interrupted'],
    ['command','Command'],['commanded','Command done'],['answered','Answer'],['asked','Which one?'],['refused','Not by voice'],['unknown','Not a command']];
  // What each command state says when picked from the review bar; Try a command passes its own line.
  const DX_COMMAND_SAMPLE={commanded:3,answered:1,asked:6,refused:2,unknown:8};
  function dxClear(){for(const t of dx.timers){window.clearInterval?.(t);clearTimeout(t);}dx.timers=[];}
  /* The caption under the island (V1.12): the whole line where the island has room for one. What was heard on top, dim;
     the result under it, two rows at most. No buttons; it goes with the dictation. */
  function dxPaintCaption(text){
    const stage=$('.review-stage');stage?.querySelector('.island-caption')?.remove();
    if(!stage||!settings.dictCaption||dx.island==='idle'||!text)return;
    const heard=dx.caption?.heard;
    stage.append(el('div','island-caption',null,heard&&heard!==text?el('small',null,null,document.createTextNode(phrase('“{text}”',{text:heard}))):null,el('p',null,null,document.createTextNode(text))));
  }
  // The words so far go to the caption only once the island cuts them.
  function dxLive(said){const t=$('.dx-island-text');dxPaintCaption(t&&t.scrollWidth>t.clientWidth?said:'');}
  function dxBands(host,live){
    const bands=el('span','dx-bands');for(let i=0;i<9;i++)bands.append(el('i'));host.append(bands);
    if(live&&prefs.motion&&window.setInterval)dx.timers.push(window.setInterval(()=>{for(const [i,b] of [...bands.children].entries())b.style.transform='scaleY('+(0.15+Math.random()*(1-Math.abs(i-4)/6)).toFixed(2)+')';},90));
  }
  function paintDictationIsland(host){
    dxClear();host.querySelector('.island-dictation')?.remove();
    for(const b of $$('[data-dictation]'))b.setAttribute('aria-pressed',String(b.dataset.dictation===dx.island));
    host.classList.toggle('dictating',dx.island!=='idle');
    if(dx.island==='idle'){dx.caption=null;dxPaintCaption('');return;}
    const s=dx.island,line=el('span','island-dictation '+s),text=el('span','dx-island-text');
    const glyph=name=>el('span','dx-glyph',null,A.icon(name,14)),hint=(...parts)=>el('span','dx-hint',null,...parts);
    if(s==='listening'||s==='locked'){
      line.append(glyph(s==='locked'?'pin':'mic'));dxBands(line,true);text.append(el('bdi',null,null,document.createTextNode(dx.partial||L(s==='locked'?'Hands-free: speak freely':'Listening…'))));
      const time=el('span','counter dx-time',null,document.createTextNode('0:00')),start=Date.now();
      if(window.setInterval)dx.timers.push(window.setInterval(()=>{time.textContent='0:'+String(Math.floor((Date.now()-start)/1000)).padStart(2,'0');},250));
      line.append(text,time,hint(kbd(s==='locked'?'Ctrl+Alt+V':'Esc'),el('span',null,s==='locked'?'finishes':'or “cancel”')));
    }
    if(s==='command'){line.append(glyph('flock'));dxBands(line,true);text.append(el('bdi',null,null,document.createTextNode(dx.partial||L('Say what to do in Vults…'))));line.append(text,hint(el('span',null,'Let go to run it')));}
    const sample=()=>dx.partial||L(DX_COMMANDS[DX_COMMAND_SAMPLE[s]].result);
    if(s==='commanded'){line.append(el('span','shape shape-done'),text);text.textContent=sample();}
    if(s==='answered'){line.append(glyph('flock'),text);text.textContent=sample();}
    if(s==='asked'){line.append(glyph('search'),text,hint(kbd('Ctrl+Alt+Z'),el('span',null,'say which')));text.textContent=sample();}
    if(s==='refused'||s==='unknown'){line.append(el('span','shape shape-quiet'),text);text.textContent=sample();}
    if(s==='cancelled'){line.append(glyph('close'),text);text.textContent=L('Cancelled: nothing was pasted.');}
    if(s==='working'||s==='retry'){line.append(glyph(s==='retry'?'retry':'mic'),el('span','dx-working'),text);text.textContent=L(s==='retry'?'Small returned nothing; trying Base…':'Transcribing on this computer…');}
    if(s==='done'){line.append(el('span','shape shape-done'),text);text.textContent=dx.partial||phrase('Pasted in {app} · {n} words · {fixes} fixes',{app:'Firefox',n:11,fixes:2});}
    if(s==='session'){line.append(glyph('flock'),text);text.textContent=phrase('In session {session} · {app}: review it and press Enter',{session:'api',app:'Codex'});}
    if(s==='held'){line.append(el('span','shape shape-needs'),text);text.textContent=phrase('{session} is waiting on a card: the text is in the clipboard',{session:'api'});}
    if(s==='clipboard'){line.append(glyph('copy'),text,hint(kbd('Ctrl+V'),el('span',null,'to paste')));text.textContent=L('No text field: it is in the clipboard');}
    if(s==='failed'){line.append(el('span','shape shape-failed'),text);text.textContent=L('Heard nothing. Check the microphone in Dictation › Settings.');}
    if(s==='speaking'||s==='interrupted'){line.append(glyph(s==='speaking'?'sound':'mic'));dxBands(line,s==='speaking');line.append(text,hint(el('span',null,s==='speaking'?'speak to interrupt':'listening')));text.textContent=L(s==='speaking'?'api finished: 3 tests passed, the diff waits for your review.':'Stopped. Go ahead.');}
    host.setAttribute('data-tooltip',text.textContent);host.append(line);
    // The caption shows only what the island cannot: what was heard, the words so far, or a line cut short.
    if(!dx.caption&&!dx.partial&&s in DX_COMMAND_SAMPLE)dx.caption={heard:L(DX_COMMANDS[DX_COMMAND_SAMPLE[s]].heard)};
    const live=['listening','locked','command'].includes(s),said=live&&dx.partial?dx.partial:'';
    dxPaintCaption(dx.caption?(dx.caption.text??text.textContent):said||(!live&&text.scrollWidth>text.clientWidth?text.textContent:''));
  }
  function dxIsland(value,partial=''){
    dx.island=DX_ISLAND.some(([id])=>id===value)?value:'idle';dx.partial=partial;
    if(state.mode==='mini')paintDictationIsland($('.mini-strip'));else setMode('mini');
  }
  // Try it: the island listens, transcribes and pastes, then the Lab shows the new line on top.
  function dxSimulate(){
    const sample={day:'Today',time:'now',app:raw('Firefox'),heard:'ask zeca to review the pull request on cloud code',secs:5};
    const words=L(sample.heard).split(' ');let i=0;dx.caption=null;dxIsland('listening');
    const step=()=>{
      if(dx.island!=='listening')return;
      i++;const bdi=$('.island-dictation bdi');if(bdi)bdi.textContent=words.slice(0,i).join(' ');dxLive(words.slice(0,i).join(' '));
      if(i<words.length){dx.timers.push(setTimeout(step,260));return;}
      dx.timers.push(setTimeout(()=>{
        dxIsland('working');
        dx.timers.push(setTimeout(()=>{
          const segs=dxFinal(sample);dxHistory.unshift(sample);dx.fresh=sample;dx.tab='history';dx.caption={heard:L(sample.heard),text:dxPlain(segs)};
          dxIsland('done',phrase('Pasted in {app} · {n} words · {fixes} fixes',{app:sample.app,n:dxWordCount(dxPlain(segs)),fixes:segs.filter(s=>s.why).length}));
          dx.timers.push(setTimeout(()=>{dx.island='idle';setArea('dictation','room');},1600));
        },900));
      },500));
    };
    dx.timers.push(setTimeout(step,260));
  }
  // Try a command: the next line of DX_COMMANDS, heard on the command key and run with no model; History keeps it.
  function dxCommand(){
    const c=DX_COMMANDS[dx.command++%DX_COMMANDS.length],words=L(c.heard).split(' ');let i=0;dx.caption=null;dxIsland('command');
    const step=()=>{
      if(dx.island!=='command')return;
      i++;const said=words.slice(0,i).join(' '),bdi=$('.island-dictation bdi');if(bdi)bdi.textContent=said;dxLive(said);
      if(i<words.length){dx.timers.push(setTimeout(step,240));return;}
      dx.timers.push(setTimeout(()=>{
        const line={day:'Today',time:'now',command:true,heard:c.heard,result:c.result,secs:2};
        dxHistory.unshift(line);dx.fresh=line;dx.tab='commands';dx.caption={heard:L(c.heard)};dxIsland(c.state,L(c.result));
        dx.timers.push(setTimeout(()=>{dx.island='idle';setArea('dictation','room');},c.state==='answered'||c.state==='asked'?2600:1600));
      },300));
    };
    dx.timers.push(setTimeout(step,240));
  }
  for(const b of $$('[data-dictation]'))b.addEventListener('click',()=>{dx.caption=null;dxIsland(b.dataset.dictation);});
