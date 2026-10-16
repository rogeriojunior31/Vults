  /* Sample patches: each session keeps its own, so a session without changes never inherits another's. */
  const islandFiles=['render.ts','views.ts','ticker.ts','scene.ts','chat.ts','flock.ts','board.ts','seen.ts','quick.ts','behavior.ts','markdown.ts','icons.ts','daypart.ts','geometry.ts','sound.ts','session.ts'].map((name,index)=>{
    const lines=[' import { SessionView } from "../bridge";',' import { t } from "../i18n";',' '];
    for(const action of ['Selection','Draft','ReadingPosition','PendingRequest'])lines.push(
      ' export function restore'+action+'(session: SessionView) {','-  const saved = lastSession;','-  return saved;',
      '+  const key = session.agent + ":" + session.id;','+  const saved = sessionState.get(key);','+  if (!saved) return null;','+  return { ...saved, project: session.project };',' }',' ',
      ' export function keep'+action+'(session: SessionView) {','-  lastSession = session;','+  sessionState.set(session.agent + ":" + session.id, session);',' }',' ');
    return {path:'ui/src/island/'+name,hunks:[{old_start:20+index*10,new_start:20+index*10,lines}]};
  });
  const nowFiles=[
    {path:'ui/src/island/now.ts',hunks:[{old_start:1,new_start:1,lines:['+import { flockFacts } from "./flock";','+','+/** Rows that ask for an action, most urgent first. */','+export function nowRows(view: View): NowRow[] {','+  const rows = flockFacts(view).needs.map(needsRow);','+  return rows.concat(failedChecks(view), reviewTasks(view));','+}']}]},
    {path:'ui/src/island/render.ts',hunks:[{old_start:910,new_start:910,lines:['   const tabs = [',"-    ['flock', 'chat', 'activity', 'connections', 'settings'],","+    ['flock', 'chat', 'now'],",'   ];']}]},
  ];
  const docsFiles=[{path:'docs/guide/flock.md',hunks:[{old_start:40,new_start:40,lines:[' ## Badges',' ','-A red badge means an error.','+- Triangle: a permission waits for you.','+- Diamond: a question waits for your answer.','+- Circle: the turn finished.','+- Square: a check failed.']}]}];
  const LAB_CHANGES={'session-1':[{step:6,files:nowFiles}],'session-2':[{step:4,files:islandFiles.slice(0,2)},{step:6,files:islandFiles.slice(0,6)},{step:8,files:islandFiles}],'session-3':[{step:5,files:docsFiles}]};
  const labChangesFor=s=>s&&LAB_CHANGES[s.id];
  const labTotals=files=>files.reduce((sum,file)=>{for(const h of file.hunks)for(const line of h.lines){if(line.startsWith('+'))sum.added++;if(line.startsWith('-'))sum.removed++;}return sum;},{added:0,removed:0});
  const labPositions=new Map();
  let captureLabPosition=()=>{};
  const DETAIL_TABS=[['session','Session'],['diff','Changes'],['activity','Activity'],['context','Context'],['zeca','Zeca']];
  /* The shared detail: a session and a task run read the same way. */
  function detailPanel(host,{id,tabs:items,current,onTab,render}){
    const panel=el('section','lab-panel');panel.id='lab-panel-'+id;panel.setAttribute('role','tabpanel');
    const nav=tabs(items.filter(([tab])=>tab!=='zeca'||settings.zeca),current,(tab,focus)=>{captureLabPosition();onTab(tab);if(focus)host.querySelector('[data-key="tab-'+tab+'"]')?.focus({preventScroll:true});},'Details','detail-tabs');
    host.append(nav,panel);let scrollNode=panel;
    render(panel,node=>scrollNode=node);
    const positionKey=id+':'+current+(current==='diff'?':'+views.flockStep+':'+views.flockFile:'');
    captureLabPosition=()=>{labPositions.set(positionKey,{top:scrollNode.scrollTop,left:scrollNode.scrollLeft??0});if(labPositions.size>50)labPositions.delete(labPositions.keys().next().value);};
    const position=labPositions.get(positionKey);scrollNode.scrollTop=position?.top??0;scrollNode.scrollLeft=position?.left??0;
  }
  function paintDiff(panel,s,setScroll){
    const changes=labChangesFor(s);
    if(!changes){panel.append(el('p','pane-note','No changes kept for this session yet.'));return;}
    const change=changes.find(c=>c.step===views.flockStep)??changes.at(-1);views.flockStep=change.step;
    if(views.flockFile!=='all'&&!change.files.some(f=>f.path===views.flockFile))views.flockFile='all';
    panel.classList.add('lab-panel-diff');
    const toolbar=el('div','lab-diff-toolbar'),repaint=()=>{captureLabPosition();remember();setArea(state.area,state.mode);};
    const step=choose('The change shown',changes.map(c=>({value:String(c.step),label:phrase(c.files.length===1?'Step {step} · 1 file':'Step {step} · {n} files',{step:c.step,n:c.files.length})})),String(change.step),value=>{views.flockStep=Number(value);repaint();});
    const file=choose('Changed file',[{value:'all',label:phrase('All {n} files',{n:change.files.length})},...change.files.map(f=>({value:f.path,label:raw(f.path)}))],views.flockFile,value=>{views.flockFile=value;repaint();});
    const totals=labTotals(change.files);
    toolbar.append(step,file,el('span','lab-diff-totals counter',null,el('span','diff-add',null,document.createTextNode('+'+totals.added)),el('span','diff-remove',null,document.createTextNode('−'+totals.removed))));panel.append(toolbar);
    const reader=el('div','lab-diff-scroll');reader.setAttribute('role','region');reader.setAttribute('aria-label',L('Code changes'));setScroll(reader);
    for(const f of change.files.filter(f=>views.flockFile==='all'||f.path===views.flockFile)){
      const section=el('section','lab-diff-file'),sum=labTotals([f]);
      section.append(el('header','lab-file-head',null,el('code',null,null,document.createTextNode(f.path)),el('span','counter',null,el('span','diff-add',null,document.createTextNode('+'+sum.added)),el('span','diff-remove',null,document.createTextNode('−'+sum.removed))),button('Open file','',()=>toast(phrase('Opened {file} at its first change',{file:f.path})))));
      for(const h of f.hunks){
        section.append(el('div','lab-hunk mono',null,document.createTextNode('@@ −'+h.old_start+' +'+h.new_start+' @@')));let old=h.old_start,next=h.new_start;
        for(const line of h.lines){
          const plus=line.startsWith('+'),minus=line.startsWith('-');
          section.append(el('div','lab-code-row '+(plus?'added':minus?'removed':'context'),null,el('span','lab-line counter',null,document.createTextNode(plus?'':String(old))),el('span','lab-line counter',null,document.createTextNode(minus?'':String(next))),el('span','lab-mark',null,document.createTextNode(line[0].trim())),el('code',null,null,document.createTextNode(line.slice(1)))));
          if(!plus)old++;if(!minus)next++;
        }
      }
      reader.append(section);
    }
    panel.append(reader,el('div','lab-diff-footnote','Sample patch. No project files were changed.'));
  }
  function paintSteps(panel,s){
    if(!s){panel.append(el('p','pane-note','No run yet. Start the task to see its steps.'));return;}
    panel.append(el('h3','lab-panel-title',phrase('Last {n} steps',{n:s.steps.length})));
    s.steps.forEach((step,index)=>panel.append(el('div','lab-step',null,el('span','counter',null,document.createTextNode(String(s.step_count-s.steps.length+index+1))),el('code',null,null,document.createTextNode(step)))));
    if(s.output)panel.append(el('h3','lab-panel-title','What it said last'),el('pre','diff-preview',null,document.createTextNode(s.output)));
  }
  function contextList(rows){const list=el('dl','lab-context');for(const [label,value] of rows)list.append(el('dt',null,label),el('dd',null,null,document.createTextNode(L(value))));return list;}
  function zecaTab(panel,project,questions){
    panel.append(el('p','pane-note','Ask about it. Your conversation and draft stay together.'),el('div','inline-actions',null,...questions.map(q=>button(q,'quiet-control',()=>prepareChat(q,state.mode==='full'?'full':'room')))),key(button('Ask Zeca','primary',()=>askSession(project)),'lab-ask'));
  }
  /* Lab · Flock */
  function renderFlock(host){
    host.classList.add('lab-split');
    const list=el('section','lab-list'),detail=el('section','lab-detail');host.append(list,detail);
    if(stageState(list,'No sessions right now.'))return;
    const stage=el('div','flock-stage'),targets=el('div','lab-bird-targets');list.append(stage);
    const facts=flockFacts(),wire=sessions.filter(s=>!settings['hide-'+s.project]);
    const flock=mountFlock(stage,'lab',wire,(scene,shown)=>{
      targets.replaceChildren();for(const slot of scene.slots()){
        const s=shown.find(v=>v.agent+':'+v.id===slot.key);if(!s)continue;
        const b=key(button(null,'lab-bird-target',()=>select(s)),'bird-'+s.id);
        b.style.left=Math.max(0,slot.x-8)+'px';b.style.width=slot.width+16+'px';b.setAttribute('aria-label',phrase('Select {project}: {status}',{project:s.project,status:L(sessionBrief(s))}));b.setAttribute('aria-pressed',String(views.sessionSelected===s.id));b.setAttribute('data-tooltip',s.project);bindBirdFocus(b,scene,s);targets.append(b);
      }
    });stage.append(targets);
    const searcher=search('Find a session…',q=>{views.sessionQuery=q;paintRows();remember();});searcher.querySelector('input').value=views.sessionQuery;
    const filter=segment([['All sessions','All'],['Needs you','Needs you'],['Working','Working'],['Quiet','Gone quiet']],value=>{views.sessionFilter=value;paintRows();remember();},views.sessionFilter);
    const rows=el('div','lab-rows');list.append(el('div','workspace-toolbar',null,filter,searcher),el('p','list-count pane-note',phrase('{flying} in flight · {wire} on the wire',{flying:facts.working.length,wire:wire.length})),rows);
    function matches(s){return (s.project+' '+labelFor(s.agent)).toLowerCase().includes(views.sessionQuery.toLowerCase())&&(views.sessionFilter==='All sessions'||views.sessionFilter==='Needs you'&&(facts.needs.includes(s)||s.status==='failed')||views.sessionFilter==='Working'&&facts.working.includes(s)||views.sessionFilter==='Quiet'&&(s.quiet||s.status==='idle'));}
    function paintRows(){
      rows.replaceChildren();const found=facts.ordered.filter(s=>!settings['hide-'+s.project]&&matches(s));
      found.forEach(s=>{const b=key(button(null,'session-row',()=>select(s)),s.id);b.setAttribute('aria-pressed',String(s.id===views.sessionSelected));bindBirdFocus(b,flock,s);
        b.append(el('span','agent-dot '+s.agent),el('span','session-copy',null,el('strong',null,s.project),el('small',null,raw(labelFor(s.agent)+' · '+L(s.started)))),sessionChip(s));rows.append(b);});
      if(!found.length)rows.append(el('p','pane-note','Nothing here matches your search.'),button('Clear filters','quiet-control',()=>{views.sessionQuery='';views.sessionFilter='All sessions';setArea('flock',state.mode);}));
    }
    function select(s){captureLabPosition();views.sessionSelected=s.id;remember();setArea('flock',state.mode);host.querySelector('[data-key="'+s.id+'"]')?.focus({preventScroll:true});}
    paintRows();
    const s=sessions.find(s=>s.id===views.sessionSelected)??facts.ordered[0];if(!s)return;
    if(!DETAIL_TABS.some(([id])=>id===views.flockDetail)||!settings.zeca&&views.flockDetail==='zeca')views.flockDetail='session';
    const more=key(iconButton('list','Quick actions',()=>sessionActions(more,s)),'lab-more');
    const task=s.task&&tasks.find(t=>t.id===s.task);
    detail.append(el('header','detail-head',null,el('div',null,null,el('h3',null,s.project),el('span','task-meta',null,el('span',null,raw(labelFor(s.agent)+' · '+s.branch)),sessionChip(s))),el('div','page-actions',null,...(task?[key(button(null,'task-link',()=>openTask(task.id)),'task-link')]:[]),more)));
    if(task)detail.querySelector('.task-link').append(A.icon('tasks',13),el('span',null,phrase('Task: {task}',{task:L(task.title)})));
    detailPanel(detail,{id:s.id,tabs:DETAIL_TABS,current:views.flockDetail,onTab:tab=>{views.flockDetail=tab;remember();setArea('flock',state.mode);},render:(panel,setScroll)=>sessionTab(panel,s,views.flockDetail,setScroll)});
  }
  function sessionTab(panel,s,tab,setScroll){
    if(tab==='session'){
      panel.append(el('p','detail-brief',sessionBrief(s)),el('p','compact-meta counter',null,document.createTextNode([phrase('Step {n}',{n:s.step_count}),...(s.subagents?[phrase(s.subagents===1?'1 subagent':'{n} subagents',{n:s.subagents})]:[]),s.cost?'≈ $'+s.cost.toFixed(2):L('Cost not reported')].join(' · '))));
      if(['approval','question'].includes(s.status)||state.scenario==='expired'&&s.id==='session-1')panel.append(requestCard(s,'lab',()=>setArea(state.area,state.mode)));
      if(s.quiet)panel.append(quietActions(s));
      if(s.status==='failed')panel.append(el('h3','lab-panel-title','What it said last'),el('pre','diff-preview',null,document.createTextNode(s.output)));
      panel.append(el('div','inline-actions',null,button('Open terminal','quiet-control',()=>toast(phrase('Raised the {editor} pane for {project}',{editor:s.editor,project:s.project}))),...(labChangesFor(s)?[button('View last change','quiet-control',()=>{views.flockDetail='diff';setArea(state.area,state.mode);})]:[])));
    }
    else if(tab==='diff')paintDiff(panel,s,setScroll);
    else if(tab==='activity')paintSteps(panel,s);
    else if(tab==='context')panel.append(contextList([['Project',s.project],['Repository','github.com/you/'+s.project],['Branch',s.branch],['Folder',s.cwd],['Agent',labelFor(s.agent)],['Editor',s.editor],['Status',statusFor(s.status)],['Cost',s.cost?phrase('≈ ${cost} at API prices',{cost:s.cost.toFixed(2)}):'Not reported'],['How it started',s.started]]));
    else zecaTab(panel,s.project,['Why did it fail?','What changed in this session?'].filter((q,i)=>i>0||s.status==='failed'));
  }
  /* Lab · Tasks: a task run is a session, so its detail reads like one. */
  const TASK_TABS=[['overview','Overview'],['diff','Changes'],['activity','Activity'],['context','Context'],['zeca','Zeca']];
  const RUN_AGENTS=['Claude Code','Codex','OpenCode','Qwen Code','Copilot CLI','Kimi Code'];
  function newTask(title,repo,engine=''){const row={id:'task-'+(tasks.length+1),title:raw(title),repo,engine,status:'To do',run:null,criteria:[],checked:[],issue:'',router:'',cost:'',rounds:[],output:''};tasks.unshift(row);return row;}
  function openTask(id,tab='overview'){views.taskSelected=id;views.taskDetail=tab==='review'?'overview':tab;views.taskFilter='Open';views.taskQuery='';setArea('tasks',state.mode==='full'?'full':'room');}
  function editTask(row) {
    const draft=row?{title:L(row.title),repo:row.repo,criteria:row.criteria.map(L).join('\n')}:taskDraft;
    const title=field('Goal',draft.title,'text',value=>{draft.title=value;remember();}),project=choose('Project',['Vults','frontend','docs','api','infra'],draft.repo,value=>{draft.repo=value;remember();}),criteria=field('Done when…, one per line',draft.criteria,'textarea',value=>{draft.criteria=value;remember();});
    const content=el('div',null,null,title.element,el('label',null,null,el('span',null,'Project'),project),criteria.element,el('p','pane-note','Creating a task does not start an agent. It runs in its own clone on a vults/task branch.'));
    openEditor(row?'Edit task':'New task',content,()=>{
      if(!draft.title.trim())throw new Error('Enter a goal first.');
      const next={title:raw(draft.title.trim()),repo:draft.repo,criteria:draft.criteria.split('\n').map(s=>raw(s.trim())).filter(Boolean)};
      if(row){Object.assign(row,next);row.checked=next.criteria.map((_,i)=>row.checked[i]??false);}
      else {const made=newTask(next.title,next.repo);made.criteria=next.criteria;made.checked=next.criteria.map(()=>false);views.taskSelected=made.id;taskDraft={title:'',repo:'Vults',criteria:'',engine:'Codex'};}
      state.scenario='ready';setArea('tasks',state.mode);toast('Task saved');return true;
    },row?'Save':'Create task');
  }
  function startTask(row,agent){
    row.engine=agent;row.status='Doing';row.review=undefined;row.router=phrase('Started with {agent}: you chose it.',{agent});
    if(!row.run){const s={...sessions[1],id:'run-'+row.id,project:row.repo,agent:agent==='Codex'?'codex':agent==='Claude Code'?'claude':'opencode',status:'working',activity:'read',step:'Read the task goal',step_count:1,subagents:0,branch:'vults/'+row.id,cwd:'~/.local/share/vults/tasks/'+row.id,cost:0.01,task:row.id,steps:['Read the task goal'],quiet:null,pending:0,output:null};sessions.push(s);A.assignSpecies(sessions);row.run=s.id;}
    toast(phrase('Started with {agent}. Its bird joins the wire.',{agent}));remember();setArea('tasks',state.mode);
  }
  function taskActions(row){
    const run=row.run&&sessions.find(s=>s.id===row.run),actions=[];
    const startMenu=b=>openActionMenu(b,'Start with',RUN_AGENTS.map(agent=>({label:agent,icon:'play',hint:agent===row.engine?'Last owner':['Claude Code','Codex'].includes(agent)?'Your setup':'ACP',action:()=>startTask(row,agent)})));
    if(row.status==='To do'||row.status==='Dropped'){const b=key(button(row.status==='Dropped'?'Reopen and start…':'Start with…','primary',()=>startMenu(b)),'task-start');actions.push(b);}
    if(row.review==='waiting'){
      const other=row.engine==='Codex'?'Claude Code':'Codex';
      actions.push(key(button('Accept and finish','primary',()=>{row.review=undefined;row.status='Done';toast('Task done');remember();setArea('tasks',state.mode);}),'task-accept'),
        key(button(phrase('Review with {agent}',{agent:other}),'quiet-control',()=>{if(row.rounds.length>=3){toast('Three review rounds already. Decide yourself.');return;}row.rounds.push({by:other,verdict:'Approved',note:'Looks right; the translation matches.'});toast(phrase('Round {n} by {agent}',{n:row.rounds.length,agent:other}));remember();setArea('tasks',state.mode);}),'task-review'));
    }
    else if(row.status==='Doing')actions.push(key(button('Stop','quiet-control',()=>{row.status='Blocked';if(run){run.status='idle';run.activity=null;}row.output='Stopped by you.';toast('Stopped. Go on resumes it.');remember();setArea('tasks',state.mode);}),'task-stop'));
    if(row.status==='Blocked')actions.push(key(button('Go on','primary',()=>{row.status='Doing';if(run){run.status='working';run.activity='run';}toast('Going on in the same session.');remember();setArea('tasks',state.mode);}),'task-go-on'),key(button('Start fresh','quiet-control',()=>openEditor('Start fresh',el('div',null,null,el('p','pane-note','A new session starts with these handoff notes instead of the old conversation.'),el('pre','diff-preview',null,document.createTextNode('Goal: '+L(row.title)+'\nDone so far: '+row.criteria.filter((_,i)=>row.checked[i]).map(L).join('; ')+'\nLast output: '+L(row.output)))),()=>{row.status='Doing';toast('Started fresh with handoff notes.');setArea('tasks',state.mode);return true;},'Start fresh')),'task-fresh'));
    if(['Doing','Blocked'].includes(row.status)&&row.review!=='waiting'){const other=row.engine==='Codex'?'Claude Code':'Codex';actions.push(key(button(phrase('Hand off to {agent}',{agent:other}),'',()=>{row.engine=other;row.router=phrase('Handed off to {agent} with handoff notes.',{agent:other});toast(phrase('Handed off to {agent}',{agent:other}));remember();setArea('tasks',state.mode);}),'task-handoff'));}
    const more=key(iconButton('list','More actions',()=>openActionMenu(more,'More actions',[
      {label:'Edit task',icon:'pencil',action:()=>editTask(row)},
      {label:row.issue?phrase('Unlink {issue}',{issue:row.issue}):'Link an issue',icon:'tag',action:()=>{if(row.issue){row.issue='';setArea('tasks',state.mode);return;}const issue=field('Issue number or URL');openEditor('Link an issue',issue.element,()=>{const v=issue.control.value.trim();if(!/^#\d+$|^https:\/\/github\.com\/[^/]+\/[^/]+\/issues\/\d+$/.test(v))throw new Error('Enter #12 or a GitHub issue URL.');row.issue=raw(v.startsWith('#')?v:'#'+v.split('/').pop());setArea('tasks',state.mode);return true;});}},
      {label:'Open an issue on GitHub',icon:'openOut',disabled:!!row.issue||!settings.github,action:()=>{row.issue='#'+(400+tasks.indexOf(row));toast(phrase('Opened issue {issue} with gh',{issue:row.issue}));setArea('tasks',state.mode);}},
      {group:'Status'},...Object.keys(TASK_TONE).filter(s=>s!==row.status).map(status=>({label:phrase('Mark as {status}',{status:L(status)}),icon:'check',action:()=>{row.status=status;row.review=undefined;setArea('tasks',state.mode);remember();}})),
      {group:'Remove'},
      {label:'Remove its folder…',key:'remove-folder',icon:'folder',disabled:!row.run||row.status==='Doing',action:()=>confirm('Remove its folder','The clone on its vults/task branch is deleted. The branch stays on GitHub if you pushed it.',()=>{toast('Folder removed');},'danger','Remove folder')},
      {label:'Delete task…',key:'delete-task',icon:'close',disabled:row.status==='Doing',action:()=>confirm('Delete task','The task, its criteria and its review rounds are deleted.',()=>{tasks.splice(tasks.indexOf(row),1);views.taskSelected=tasks[0]?.id??'';setArea('tasks',state.mode);remember();},'danger','Delete')},
    ])),'task-more');
    actions.push(more);return actions;
  }
  function renderTasks(host){
    host.classList.add('lab-split');
    const list=el('section','lab-list'),detail=el('section','lab-detail');host.append(list,detail);
    list.append(pageHeading('Tasks',phrase('Today: ${spent} of ${budget}',{spent:'2.40',budget:Number(settings.dailyBudget).toFixed(2)}),key(button('New task','primary',()=>editTask()),'new-task')));
    if(stageState(list,'No tasks yet.',button('New task','quiet-control',()=>editTask())))return;
    const finder=search('Search tasks',value=>{views.taskQuery=value;paint();remember();});finder.querySelector('input').value=views.taskQuery;
    const filter=segment([['Open','Open'],['Needs you','Needs you'],['Finished','Finished']],value=>{views.taskFilter=value;paint();remember();},views.taskFilter);
    const rows=el('div','lab-rows');list.append(el('div','workspace-toolbar',null,filter,finder),rows);
    function matches(row){const finished=['Done','Dropped'].includes(row.status);return (views.taskFilter==='Open'?!finished:views.taskFilter==='Finished'?finished:row.review==='waiting'||row.status==='Blocked')&&(L(row.title)+' '+row.repo).toLowerCase().includes(views.taskQuery.toLowerCase());}
    function paint(){
      rows.replaceChildren();const found=tasks.filter(matches);let group;
      [...found].sort((a,b)=>a.repo.localeCompare(b.repo)).forEach(row=>{
        if(row.repo!==group){group=row.repo;rows.append(el('small','menu-group',null,document.createTextNode(group)));}
        const b=key(button(null,'session-row task-row',()=>{views.taskSelected=row.id;remember();setArea('tasks',state.mode);}),row.id);b.setAttribute('aria-pressed',String(row.id===views.taskSelected));
        b.append(A.icon('tasks',15),el('span','session-copy',null,el('strong',null,row.title),el('small',null,null,document.createTextNode([row.engine||L('No owner yet'),row.checked.filter(Boolean).length+'/'+row.criteria.length].join(' · ')))),taskChip(row));rows.append(b);
      });
      if(!found.length)rows.append(el('p','pane-note','Nothing here matches your search.'),button('Clear filters','quiet-control',()=>{views.taskQuery='';views.taskFilter='Open';setArea('tasks',state.mode);}));
    }
    paint();
    const row=tasks.find(t=>t.id===views.taskSelected)??tasks.find(matches);if(!row){detail.append(el('p','pane-note','Choose a task.'));return;}
    const run=row.run&&sessions.find(s=>s.id===row.run);
    detail.append(el('header','detail-head',null,el('div',null,null,el('h3',null,row.title),el('span','task-meta',null,el('span',null,null,document.createTextNode([row.repo,row.engine||L('No owner yet'),...(row.issue?[row.issue]:[])].join(' · '))),taskChip(row))),el('div','page-actions',null,...taskActions(row))));
    if(run&&['approval','question'].includes(run.status))detail.append(key(button(null,'run-card-link',()=>{views.sessionSelected=run.id;views.flockDetail='session';setArea('flock',state.mode);}),'run-card'));
    detail.querySelector('.run-card-link')?.append(el('span','shape shape-needs'),el('span',null,phrase('Its run is waiting for you: {command}',{command:run.step})),A.icon('chevron',12));
    detailPanel(detail,{id:row.id,tabs:TASK_TABS,current:views.taskDetail,onTab:tab=>{views.taskDetail=tab;remember();setArea('tasks',state.mode);},render:(panel,setScroll)=>{
      const tab=views.taskDetail;
      if(tab==='overview'){
        if(row.router)panel.append(el('p','router-line',null,A.icon('branch',12),el('span',null,row.router)));
        const done=row.checked.filter(Boolean).length,criteria=el('section','criteria-list',null,el('h3','lab-panel-title',phrase('Done when… {done} of {all}',{done,all:row.criteria.length})));
        row.criteria.forEach((text,i)=>{const c=el('input');c.type='checkbox';c.checked=!!row.checked[i];c.addEventListener('change',()=>{row.checked[i]=c.checked;remember();setArea('tasks',state.mode);});criteria.append(el('label',null,null,c,el('span',null,text)));});
        if(!row.criteria.length)criteria.append(el('p','pane-note','No criteria yet. Add them in Edit task.'));
        const next=row.criteria.find((_,i)=>!row.checked[i]);if(next)criteria.append(el('p','pane-note',phrase('Next: {criterion}',{criterion:L(next)})));
        panel.append(criteria);
        if(row.output)panel.append(el('h3','lab-panel-title','What it said last'),el('pre','diff-preview',null,document.createTextNode(L(row.output))));
        if(row.rounds.length){panel.append(el('h3','lab-panel-title','Review rounds'));row.rounds.forEach((r,i)=>panel.append(el('div','review-round',null,el('span','counter',null,document.createTextNode(String(i+1))),el('strong',null,r.by),chip(r.verdict==='Approved'?'done':'needs',r.verdict),el('p',null,r.note))));}
      }
      else if(tab==='diff')paintDiff(panel,run,setScroll);
      else if(tab==='activity')paintSteps(panel,run);
      else if(tab==='context')panel.append(contextList([['Project',row.repo],['Branch',run?.branch??'vults/'+row.id],['Clone',run?.cwd??'Created when it starts'],['Owner',row.engine||'No owner yet'],['Issue',row.issue||'None'],['Cost',row.cost||'Not started']]));
      else zecaTab(panel,row.repo,[phrase('Review the task: {task}',{task:L(row.title)})]);
    }});
  }
