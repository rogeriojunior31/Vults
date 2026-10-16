  /* One card in three densities (Island signal, Compact, Lab) and inside Zeca's chat. Answering is always a click. */
  const DESTRUCTIVE=/git clean|rm -rf|--force|\|\s*sh\b|git reset --hard/;
  const NEXT_REQUEST={'npm run build':'npm run lint'};
  const QUESTION={text:'Which checks should run before the PR?',options:['Only the UI tests','Everything (about 4 min)','Skip the checks for now']};
  function recordAudit(answer,command,by='You',project='Vults',agent='Codex'){auditRows.unshift({time:new Date().toLocaleTimeString('en-GB',{hour:'2-digit',minute:'2-digit'}),project,agent,answer,by,command:raw(command)});}
  function flockFacts() {
    const needs=sessions.filter(s=>s.status==='approval'||s.status==='question');
    const failed=sessions.filter(s=>s.status==='failed'),working=sessions.filter(s=>s.status==='working'||s.status==='thinking'),quiet=sessions.filter(s=>s.quiet);
    const priority=s=>needs.includes(s)?0:s.status==='failed'?1:s.quiet?2:['working','thinking'].includes(s.status)?3:4;
    return {needs,failed,working,quiet,ordered:[...sessions].sort((a,b)=>priority(a)-priority(b))};
  }
  function sessionBrief(s) {
    if(state.scenario==='expired'&&s.id==='session-1')return 'The request went back to the terminal.';
    if(s.status==='approval')return 'Waiting for your permission';
    if(s.status==='question')return 'Waiting for your answer';
    if(s.status==='failed')return 'A check failed. The output needs review.';
    if(s.status==='finished')return 'The agent finished its turn';
    if(s.status==='thinking')return 'Thinking about the next step';
    if(s.quiet)return phrase('Quiet for {n} minutes. It may be waiting for you.',{n:s.quiet});
    if(s.status==='working'&&s.activity==='read')return 'Reading project context';
    if(s.status==='working')return s.activity==='edit'?'Editing a project file':s.activity==='run'?'Running project checks':'Working on the current step';
    return 'Ready for the next turn';
  }
  // Skipping never answers: the card keeps waiting in Now and in the terminal until its countdown ends.
  const skipKey=s=>s.id+':'+s.step;
  const isSkipped=s=>views.skipped.includes(skipKey(s));
  function skip(s){if(!isSkipped(s))views.skipped.push(skipKey(s));views.skipped=views.skipped.slice(-30);toast('Skipped. It still waits in Now and in its terminal.');remember();}
  function unskip(s){views.skipped=views.skipped.filter(k=>k!==skipKey(s));remember();}
  // Always is an exact rule (agent, tool, target, folder); never for destructive, multi-line or long targets.
  const canAlways = (s,destructive) => !destructive && !s.step.includes('\n') && s.step.length<=300;
  function requestCard(s,density='compact',onAnswer=()=>{}) {
    if(s.status==='question')return questionCard(s,density,onAnswer);
    const destructive=DESTRUCTIVE.test(s.step), chat=density==='chat', expired=state.scenario==='expired'&&s.id==='session-1';
    const card=el('section','request-card density-'+density+(destructive?' destructive':''));card.setAttribute('aria-label',L('Permission required'));
    const who=chat?'Zeca wants to run':phrase('{agent} wants to run',{agent:labelFor(s.agent)});
    const head=el('header','request-head',null,el('span','agent-dot '+(chat?'zeca':s.agent)),el('strong',null,who),el('span','request-tool mono',null,document.createTextNode(s.tool??'Bash')));
    if(s.pending>1&&!chat)head.append(el('span','request-count counter',phrase('1 of {n}',{n:s.pending})));
    if(!chat&&!expired)head.append(key(iconButton('close','Skip for now',()=>{skip(s);onAnswer('Skipped');}),'skip'));
    card.append(head);
    if(s.why)card.append(el('p','request-why',null,el('span',null,s.why)));
    if(destructive)card.append(el('p','request-reason danger-text',null,A.icon('retry',12),el('span',null,'Destructive: no policy ever allows it')));
    else if(density==='lab')card.append(el('p','request-reason',null,A.icon('person',12),el('span',null,'Your policy asks first for builds outside src/')));
    if(s.outside)card.append(el('p','request-reason danger-text',null,A.icon('openOut',12),el('span',null,'This follows content from outside (a web page). Check the command before allowing it.')));
    const where=el('div','request-where',null,el('span','request-cwd mono',null,A.icon('folder',11),document.createTextNode(s.cwd)));
    card.append(el('pre','request-command',null,document.createTextNode(s.step)),where);
    if(expired){card.append(el('p','pane-note','This request went back to the terminal. Answer it there.'));return card;}
    if(!chat)where.append(el('span','request-clock',null,el('span','clock-bar'),el('small','counter',phrase('Goes back to the terminal in {time}',{time:'1:12'}))));
    const decide=answer=>{
      const allowed=answer!=='Denied';
      if(chat){recordAudit(answer,s.step,'You','Vults','Zeca');onAnswer(answer);toast(allowed?'Allowed. Zeca continues.':'Denied. Zeca will not run it.');remember();return;}
      if(answer==='Always allowed')settings.rules.unshift({agent:labelFor(s.agent),tool:s.tool??'Bash',target:s.step,cwd:s.cwd});
      recordAudit(answer,s.step,'You',s.project,labelFor(s.agent));
      const next=allowed&&NEXT_REQUEST[s.step];
      if(s.pending>1&&next){s.step=next;s.pending--;}
      else {s.status=allowed?'working':'idle';s.activity=allowed?'run':null;s.card=s.waiting=false;s.pending=0;}
      toast(answer==='Always allowed'?'Always allowed. Settings › Approvals lists the rule.':allowed?'Allowed. The agent continues.':'Denied. The agent gets your answer.');
      onAnswer(answer);remember();
    };
    const deny=key(button(null,destructive?'primary':'quiet-control',()=>decide('Denied')),'deny');deny.append(el('span',null,'Deny'),...(density==='lab'?[kbd('Ctrl+Alt+N')]:[]));if(!chat)deny.setAttribute('data-tooltip','Ctrl+Alt+N');
    const allow=key(button(null,destructive?'quiet-control':'primary',()=>decide('Allowed')),'allow');allow.append(el('span',null,chat?'Allow':'Allow once'),...(density==='lab'?[kbd('Ctrl+Alt+Y')]:[]));if(!chat)allow.setAttribute('data-tooltip','Ctrl+Alt+Y');
    const actions=el('div','request-actions',null,deny,el('span','spacer'));
    if(canAlways(s,destructive))actions.append(key(button(chat?'Always in this chat':'Always allow','',()=>decide(chat?'Allowed in this chat':'Always allowed')),'always'));
    actions.append(allow);card.append(actions);
    card.addEventListener('keydown',event=>{if(event.ctrlKey&&event.altKey&&['y','n'].includes(event.key?.toLowerCase())){event.preventDefault();(event.key.toLowerCase()==='y'?allow:deny).click();}});
    if(density==='lab'){
      const recent=auditRows.filter(r=>r.project===s.project).slice(0,2).map(r=>el('div',null,null,el('span','counter',null,document.createTextNode(r.time)),el('span',null,phrase('{answer} by {by}',{answer:L(r.answer),by:L(r.by)})),el('code',null,null,document.createTextNode(r.command))));
      card.append(details(phrase('Recent answers in {project}',{project:s.project}),recent.length?recent:['Nothing answered here yet.'],'run-details request-history'));
    }
    return card;
  }
  function questionCard(s,density,onAnswer) {
    const card=el('section','request-card question density-'+density);card.setAttribute('aria-label',L('Question'));
    card.append(el('header','request-head',null,el('span','agent-dot '+s.agent),el('strong',null,phrase('{agent} asks',{agent:labelFor(s.agent)})),el('span','request-tool',s.multi?'Choose any':'Choose one')),el('p','question-text',QUESTION.text));
    const picked=new Set(),list=el('div','question-options');list.setAttribute('role',s.multi?'group':'listbox');
    const answer=text=>{s.status='working';s.activity='run';s.card=s.waiting=false;recordAudit('Answered',text,'You',s.project,labelFor(s.agent));toast('Answer sent. The agent continues.');onAnswer('Answered');remember();};
    QUESTION.options.forEach((option,i)=>{
      const b=key(button(null,'question-option',()=>{if(!s.multi){answer(L(option));return;}picked.has(option)?picked.delete(option):picked.add(option);b.setAttribute('aria-pressed',String(picked.has(option)));send.disabled=!picked.size;}),'option-'+(i+1));
      b.append(kbd(String(i+1)),el('span',null,option));if(s.multi)b.setAttribute('aria-pressed','false');list.append(b);
    });
    const other=key(button(null,'question-option',()=>{const text=field('Your answer');openEditor('Other answer',text.element,()=>{if(!text.control.value.trim())throw new Error('Write an answer first.');answer(text.control.value.trim());return true;},'Send answer');}),'option-other');other.append(kbd(String(QUESTION.options.length+1)),el('span',null,'Other…'));list.append(other);
    card.append(list);
    const send=key(button('Send answer','primary',()=>answer([...picked].map(L).join(', '))),'send-answer');send.disabled=true;
    const release=key(button('Reply in the terminal','',()=>{s.status='idle';s.card=s.waiting=false;recordAudit('Released',QUESTION.text,'You',s.project,labelFor(s.agent));toast('The question went back to the terminal.');onAnswer('Released');remember();}),'release');
    card.append(el('div','request-actions',null,release,el('span','spacer'),...(s.multi?[send]:[])));
    card.addEventListener('keydown',event=>{const n=Number(event.key);if(n>=1&&n<=QUESTION.options.length+1){event.preventDefault();list.children[n-1]?.click();}});
    return card;
  }
  /* Quick actions: the "⋯" on a session, the same rows in Compact and Lab. */
  function sessionActions(trigger,s){
    openActionMenu(trigger,phrase('{project} · {agent}',{project:s.project,agent:labelFor(s.agent)}),[
      {label:'Open terminal',icon:'terminal',hint:raw(s.editor),action:()=>toast(phrase('Raised the {editor} pane for {project}',{editor:s.editor,project:s.project}))},
      {label:'Open folder',icon:'folder',action:()=>toast(phrase('Opened {folder}',{folder:s.cwd}))},
      {label:'View last change',icon:'file',disabled:!labChangesFor(s),action:()=>{views.sessionSelected=s.id;views.flockDetail='diff';setArea('flock',state.mode==='full'?'full':'room');}},
      {label:s.raise?'Let the flock choose':'Keep in front',icon:'pin',action:()=>{s.raise=!s.raise;toast(s.raise?'The flock chooses who is in front.':'Kept in front.');}},
      {group:'This project'},
      {label:settings['pin-'+s.project]?'Unpin project':'Pin project',icon:'pin',action:()=>{settings['pin-'+s.project]=!settings['pin-'+s.project];toast(settings['pin-'+s.project]?'Pinned at the top of the list.':'Unpinned.');setArea(state.area,state.mode);}},
      {label:settings['mute-'+s.project]?'Unmute project':'Mute project',icon:'mute',action:()=>{settings['mute-'+s.project]=!settings['mute-'+s.project];toast('Saved');}},
      {label:'Hide project',icon:'hide',action:()=>confirm('Hide project',phrase('{project} leaves the wire until you show it again in Settings › Projects.',{project:s.project}),()=>{settings['hide-'+s.project]=true;toast('Hidden. Settings › Projects brings it back.');})},
      {label:"This project's bird…",icon:'flock',action:()=>openSettings('projects')},
    ]);
  }
  function quietActions(s,host){
    return el('div','inline-actions quiet-actions',null,
      key(button('Snooze 30 min','',()=>{s.quiet=null;toast('Snoozed for 30 minutes.');refresh();}),'snooze'),
      key(button('Keep going','',()=>{s.quiet=null;s.status='working';toast('Sent: keep going.');refresh();}),'keep-going'),
      key(button('Dismiss','',()=>{s.quiet=null;refresh();}),'dismiss'));
    function refresh(){setArea(state.area,state.mode);remember();}
  }
