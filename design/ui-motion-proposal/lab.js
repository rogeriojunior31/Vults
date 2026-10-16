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
  const LAB_CHANGES={'session-1':[{step:6,files:nowFiles}],'session-2':[{step:4,files:islandFiles.slice(0,2)},{step:6,files:islandFiles.slice(0,6)},{step:8,files:islandFiles}],'session-3':[{step:5,files:docsFiles}],'run-task-1':[{step:4,files:islandFiles.slice(0,3)}],'run-task-2':[{step:3,files:nowFiles}],'run-task-3':[{step:3,files:docsFiles}]};
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
  /* Lab · Flock: the wire and its sky across the top, the list and one detail below. Resting birds circle in the sky. */
  function renderFlock(host){
    host.classList.add('lab-split','with-sky');
    const band=el('section','lab-sky'),list=el('section','lab-list'),detail=el('section','lab-detail');host.append(band,list,detail);
    if(stageState(list,'No sessions right now.'))return;
    const facts=flockFacts(),wire=sessions.filter(s=>!settings['hide-'+s.project]);
    const stage=el('div','flock-stage lab-wire'),targets=el('div','lab-bird-targets');band.append(stage);
    const flock=mountFlock(stage,'lab',wire,(scene,shown)=>{
      targets.replaceChildren();for(const slot of scene.slots()){
        const s=shown.find(v=>v.agent+':'+v.id===slot.key);if(!s)continue;
        const b=key(button(null,'lab-bird-target',()=>select(s)),'bird-'+s.id);
        b.style.left=Math.max(0,slot.x-8)+'px';b.style.width=slot.width+16+'px';b.setAttribute('aria-label',phrase('Select {project}: {status}',{project:s.project,status:L(sessionBrief(s))}));b.setAttribute('aria-pressed',String(views.sessionSelected===s.id));b.setAttribute('data-tooltip',s.project);bindBirdFocus(b,scene,s);targets.append(b);
      }
    });stage.append(targets);
    const note=el('p','sky-note');band.append(note);
    const count=()=>{const flying=wire.filter(soaring).length;note.textContent=phrase('{flying} resting in the sky · {wire} on the wire',{flying,wire:wire.length-flying});};
    attachSky('lab',band,flock,wire,count);
    const searcher=search('Find a session…',q=>{views.sessionQuery=q;paintRows();remember();});searcher.querySelector('input').value=views.sessionQuery;
    const filter=segment([['All sessions','All'],['Needs you','Needs you'],['Working','Working'],['Quiet','Gone quiet']],value=>{views.sessionFilter=value;paintRows();remember();},views.sessionFilter);
    const rows=el('div','lab-rows');list.append(el('div','workspace-toolbar',null,filter,searcher),rows);
    flexible(host,list,'flock-list',{def:380,min:260,max:560,label:'Sessions'});
    function matches(s){return (s.project+' '+labelFor(s.agent)).toLowerCase().includes(views.sessionQuery.toLowerCase())&&(views.sessionFilter==='All sessions'||views.sessionFilter==='Needs you'&&(facts.needs.includes(s)||s.status==='failed')||views.sessionFilter==='Working'&&facts.working.includes(s)||views.sessionFilter==='Quiet'&&(s.quiet||s.status==='idle'));}
    function paintRows(){
      rows.replaceChildren();const found=facts.ordered.filter(s=>!settings['hide-'+s.project]&&matches(s));
      found.forEach(s=>{const b=key(button(null,'session-row',()=>select(s)),s.id);b.setAttribute('aria-pressed',String(s.id===views.sessionSelected));bindBirdFocus(b,flock,s);
        b.append(el('span','agent-dot '+s.agent),el('span','session-copy',null,el('strong',null,s.project),el('small',null,raw(labelFor(s.agent)+' · '+L(s.started)))),soaring(s)?chip('done','In the sky'):sessionChip(s));rows.append(b);});
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
      if(['approval','question'].includes(s.status)&&isSkipped(s))panel.append(el('p','pane-note','Skipped. It still waits in Now and in its terminal.'),key(button('Show the card','quiet-control',()=>{unskip(s);setArea(state.area,state.mode);}),'unskip'));
      else if(['approval','question'].includes(s.status)||state.scenario==='expired'&&s.id==='session-1')panel.append(requestCard(s,'lab',()=>setArea(state.area,state.mode)));
      if(s.quiet)panel.append(quietActions(s));
      if(s.status==='failed')panel.append(el('h3','lab-panel-title','What it said last'),el('pre','diff-preview',null,document.createTextNode(s.output)));
      panel.append(el('div','inline-actions',null,button('Open terminal','quiet-control',()=>toast(phrase('Raised the {editor} pane for {project}',{editor:s.editor,project:s.project}))),...(labChangesFor(s)?[button('View last change','quiet-control',()=>{views.flockDetail='diff';setArea(state.area,state.mode);})]:[])));
    }
    else if(tab==='diff')paintDiff(panel,s,setScroll);
    else if(tab==='activity')paintSteps(panel,s);
    else if(tab==='context')panel.append(contextList([['Project',s.project],['Repository','github.com/you/'+s.project],['Branch',s.branch],['Folder',s.cwd],['Agent',labelFor(s.agent)],['Editor',s.editor],['Status',statusFor(s.status)],['Cost',s.cost?phrase('≈ ${cost} at API prices',{cost:s.cost.toFixed(2)}):'Not reported'],['How it started',s.started]]));
    else zecaTab(panel,s.project,['Why did it fail?','What changed in this session?'].filter((q,i)=>i>0||s.status==='failed'));
  }
  /* Lab · Tasks: agents Vults starts on its own. A task reads as a run: plan, run, check, review, PR. */
  const TASK_TABS=[['run','Run'],['diff','Changes'],['context','Context'],['zeca','Zeca']];
  const RUN_AGENTS=['Claude Code','Codex','OpenCode','Qwen Code','Copilot CLI','Kimi Code'];
  const AGENT_KIND={'Claude Code':'claude','Codex':'codex','OpenCode':'opencode','Qwen Code':'qwen'};
  const STAGES=[['plan','Plan'],['run','Run'],['check','Check'],['review','Review'],['pr','PR']];
  function taskStage(row){
    const r=row.run;
    if(row.status==='Done')return {at:5};
    if(row.status==='To do'||row.status==='Dropped'||!r)return {at:0};
    if(row.review==='waiting')return {at:3,needs:true};
    if(r.status==='failed'||row.status==='Blocked')return {at:row.checks.includes('failed')?2:1,failed:true};
    if(r.status==='finished')return {at:2};
    return {at:1,needs:['approval','question'].includes(r.status)};
  }
  const taskNeedsYou=row=>row.review==='waiting'||row.status==='Blocked'||['approval','question'].includes(row.run?.status);
  function taskGroup(row){
    if(['Done','Dropped'].includes(row.status))return 'done';
    if(taskNeedsYou(row))return 'needs';
    if(row.status==='Doing')return 'running';
    return 'queued';
  }
  function newTask(title,repo,engine=''){const row={id:'task-'+(tasks.length+1),title:raw(title),repo,engine,status:'To do',onWire:false,run:null,criteria:[],checks:[],checked:[],issue:'',router:'',cost:'',rounds:[]};tasks.unshift(row);return row;}
  function openTask(id,tab='run'){views.taskSelected=id;views.taskDetail=tab==='review'?'run':tab;views.taskQuery='';setArea('tasks',state.mode==='full'?'full':'room');}
  function editTask(row) {
    const draft=row?{title:L(row.title),repo:row.repo,criteria:row.criteria.map(L).join('\n')}:taskDraft;
    const title=field('Goal',draft.title,'text',value=>{draft.title=value;remember();}),project=choose('Project',['Vults','frontend','docs','api','infra'],draft.repo,value=>{draft.repo=value;remember();}),criteria=field('Done when…, one per line',draft.criteria,'textarea',value=>{draft.criteria=value;remember();});
    const content=el('div',null,null,title.element,el('label',null,null,el('span',null,'Project'),project),criteria.element,el('p','pane-note','A line that names a command (npm run build passes) becomes a check Vults can run.'),el('p','pane-note','Creating a task does not start an agent. It runs in its own clone on a vults/task branch.'));
    openEditor(row?'Edit task':'New task',content,()=>{
      if(!draft.title.trim())throw new Error('Enter a goal first.');
      const next={title:raw(draft.title.trim()),repo:draft.repo,criteria:draft.criteria.split('\n').map(s=>raw(s.trim())).filter(Boolean)};
      if(row){Object.assign(row,next);row.checked=next.criteria.map((_,i)=>row.checked[i]??false);row.checks=next.criteria.map((_,i)=>row.checks[i]??null);}
      else {const made=newTask(next.title,next.repo);made.criteria=next.criteria;made.checked=next.criteria.map(()=>false);made.checks=next.criteria.map(()=>null);views.taskSelected=made.id;taskDraft={title:'',repo:'Vults',criteria:'',engine:'Codex'};}
      state.scenario='ready';setArea('tasks',state.mode);toast('Task saved');return true;
    },row?'Save':'Create task');
  }
  function startTask(row,agent){
    row.engine=agent;row.status='Doing';row.review=undefined;row.router=phrase('Started with {agent}: you chose it.',{agent});
    if(!row.run){row.run=run({id:'run-'+row.id,task:row.id,agent:AGENT_KIND[agent]??'other',project:row.repo,status:'working',activity:'read',step:'Read the task goal',step_count:1,cost:0.01,steps:['Read the task goal'],output:null});A.assignSpecies([row.run]);}
    else {row.run.status='working';row.run.activity='read';}
    syncWire(row);toast(phrase('Started with {agent}.',{agent}));remember();setArea('tasks',state.mode);
  }
  // A run joins the wire only when its task asks for it.
  function syncWire(row){
    const at=sessions.findIndex(s=>s.id===row.run?.id);
    if(row.onWire&&row.run&&at<0){sessions.push(row.run);A.assignSpecies(sessions);}
    if((!row.onWire||!row.run)&&at>=0)sessions.splice(at,1);
  }
  function runCheck(row,i){
    const text=L(row.criteria[i]);row.checks[i]='running';setArea('tasks',state.mode);
    window.setTimeout(()=>{row.checks[i]=row.id==='task-4'?'failed':'passed';row.checked[i]=row.checks[i]==='passed';remember();if(state.area==='tasks')setArea('tasks',state.mode);},900);
    toast(phrase('Running the check: {check}',{check:text}));
  }
  function taskActions(row){
    const r=row.run,actions=[];
    const startMenu=b=>openActionMenu(b,'Start with',RUN_AGENTS.map(agent=>({label:agent,icon:'play',hint:agent===row.engine?'Last owner':['Claude Code','Codex'].includes(agent)?'Your setup':'ACP',action:()=>startTask(row,agent)})));
    if(row.status==='To do'||row.status==='Dropped'){const b=key(button(row.status==='Dropped'?'Reopen and start…':'Start with…','primary',()=>startMenu(b)),'task-start');actions.push(b);}
    if(row.review==='waiting'){
      const other=row.engine==='Codex'?'Claude Code':'Codex';
      actions.push(key(button('Accept and open a PR','primary',()=>{row.review=undefined;row.status='Done';github.pulls.unshift({id:'#'+(365+tasks.indexOf(row)),title:row.title,repo:row.repo,branch:'vults/'+row.id,author:'you',size:'+40 −6',checks:'running',checkList:[['CI','running']],merged:false,isNew:true});toast('Pull request opened. Its checks are running.');remember();setArea('tasks',state.mode);}),'task-accept'),
        key(button(phrase('Review with {agent}',{agent:other}),'quiet-control',()=>{if(row.rounds.length>=3){toast('Three review rounds already. Decide yourself.');return;}row.rounds.push({by:other,verdict:'Approved',note:'Looks right; the translation matches.'});toast(phrase('Round {n} by {agent}',{n:row.rounds.length,agent:other}));remember();setArea('tasks',state.mode);}),'task-review'));
    }
    else if(row.status==='Doing')actions.push(key(button('Stop','quiet-control',()=>{row.status='Blocked';if(r){r.status='idle';r.activity=null;r.output='Stopped by you.';}toast('Stopped. Go on resumes it.');remember();setArea('tasks',state.mode);}),'task-stop'));
    if(row.status==='Blocked')actions.push(key(button('Go on','primary',()=>{row.status='Doing';if(r){r.status='working';r.activity='run';}toast('Going on in the same session.');remember();setArea('tasks',state.mode);}),'task-go-on'),key(button('Start fresh','quiet-control',()=>openEditor('Start fresh',el('div',null,null,el('p','pane-note','A new session starts with these handoff notes instead of the old conversation.'),el('pre','diff-preview',null,document.createTextNode('Goal: '+L(row.title)+'\nDone so far: '+row.criteria.filter((_,i)=>row.checked[i]).map(L).join('; ')+'\nLast output: '+L(r?.output??'')))),()=>{row.status='Doing';if(r){r.status='working';r.activity='read';}toast('Started fresh with handoff notes.');setArea('tasks',state.mode);return true;},'Start fresh')),'task-fresh'));
    if(['Doing','Blocked'].includes(row.status)&&row.review!=='waiting'){const other=row.engine==='Codex'?'Claude Code':'Codex';actions.push(key(button(phrase('Hand off to {agent}',{agent:other}),'',()=>{row.engine=other;row.router=phrase('Handed off to {agent} with handoff notes.',{agent:other});toast(phrase('Handed off to {agent}',{agent:other}));remember();setArea('tasks',state.mode);}),'task-handoff'));}
    const more=key(iconButton('list','More actions',()=>openActionMenu(more,'More actions',[
      {label:'Edit task',icon:'pencil',action:()=>editTask(row)},
      {label:row.onWire?'Take it off the wire':'Show it on the wire',key:'task-wire',icon:'flock',disabled:!row.run,action:()=>{row.onWire=!row.onWire;syncWire(row);toast(row.onWire?'Its bird joins the wire.':'Its bird leaves the wire.');remember();setArea('tasks',state.mode);}},
      {label:row.issue?phrase('Unlink {issue}',{issue:row.issue}):'Link an issue',icon:'tag',action:()=>{if(row.issue){row.issue='';setArea('tasks',state.mode);return;}const issue=field('Issue number or URL');openEditor('Link an issue',issue.element,()=>{const v=issue.control.value.trim();if(!/^#\d+$|^https:\/\/github\.com\/[^/]+\/[^/]+\/issues\/\d+$/.test(v))throw new Error('Enter #12 or a GitHub issue URL.');row.issue=raw(v.startsWith('#')?v:'#'+v.split('/').pop());setArea('tasks',state.mode);return true;});}},
      {label:'Open an issue on GitHub',icon:'openOut',disabled:!!row.issue||!settings.github,action:()=>{row.issue='#'+(400+tasks.indexOf(row));toast(phrase('Opened issue {issue} with gh',{issue:row.issue}));setArea('tasks',state.mode);}},
      {group:'Status'},...Object.keys(TASK_TONE).filter(s=>s!==row.status).map(status=>({label:phrase('Mark as {status}',{status:L(status)}),icon:'check',action:()=>{row.status=status;row.review=undefined;setArea('tasks',state.mode);remember();}})),
      {group:'Remove'},
      {label:'Remove its folder…',key:'remove-folder',icon:'folder',disabled:!r||row.status==='Doing',action:()=>confirm('Remove its folder','The clone on its vults/task branch is deleted. The branch stays on GitHub if you pushed it.',()=>{toast('Folder removed');},'danger','Remove folder')},
      {label:'Delete task…',key:'delete-task',icon:'close',disabled:row.status==='Doing',action:()=>confirm('Delete task','The task, its criteria and its review rounds are deleted.',()=>{tasks.splice(tasks.indexOf(row),1);row.onWire=false;syncWire(row);views.taskSelected=tasks[0]?.id??'';setArea('tasks',state.mode);remember();},'danger','Delete')},
    ])),'task-more');
    actions.push(more);return actions;
  }
  // One line that says where a task is, in the list's own words.
  function taskLine(row){
    const r=row.run;
    if(row.status==='Done')return {tone:'done',shape:'done',text:L('Done')};
    if(row.status==='Dropped')return {tone:'quiet',shape:'quiet',text:L('Dropped')};
    if(row.review==='waiting')return {tone:'needs',shape:'needs',text:phrase('Your review · round {n}',{n:row.rounds.length})};
    if(['approval','question'].includes(r?.status))return {tone:'needs',shape:'needs',text:phrase('Waiting for you · {command}',{command:r.step})};
    if(row.status==='Blocked'||r?.status==='failed')return {tone:'failed',shape:'failed',text:row.checks.includes('failed')?L('A check failed'):L('Stopped')};
    if(row.status==='Doing')return {tone:'working',shape:'working',text:phrase('Running · step {n}',{n:r?.step_count??1})};
    return {tone:'quiet',shape:'quiet',text:L('Queued')};
  }
  const TASK_GROUPS=[['needs','Needs you','needs'],['running','Running','working'],['queued','Queued','quiet'],['done','Finished','done']];
  function renderTasks(host){
    host.classList.add('task-board');
    const side=el('aside','task-side'),main=el('section','task-main');host.append(side,main);
    const add=key(button(null,'primary task-new',()=>editTask(),'New task'),'new-task');add.append(A.icon('plus',14),el('span',null,'New task'));
    side.append(el('div','task-side-head',null,el('strong',null,'Tasks'),el('span','counter',null,document.createTextNode(String(tasks.filter(t=>!['Done','Dropped'].includes(t.status)).length)))),add);
    const panel=flexible(host,side,'tasks',{def:300,min:220,max:460,rail:60,label:'Tasks',keep:true});
    if(stageState(main,'No tasks yet.',button('New task','quiet-control',()=>editTask())))return;
    if(!panel.folded){const finder=search('Search tasks',value=>{views.taskQuery=value;paintList();remember();});finder.querySelector('input').value=views.taskQuery;side.append(finder);}
    const list=el('nav','task-groups');list.setAttribute('aria-label',L('Tasks'));side.append(list,el('p','task-budget counter',phrase('Today: ${spent} of ${budget}',{spent:'2.40',budget:Number(settings.dailyBudget).toFixed(2)})));
    function paintList(){
      list.replaceChildren();
      for(const [group,label,tone] of TASK_GROUPS){
        const rows=tasks.filter(t=>taskGroup(t)===group&&(L(t.title)+' '+t.repo).toLowerCase().includes(views.taskQuery.toLowerCase()));if(!rows.length)continue;
        const section=el('section','task-group tone-'+tone);section.append(el('h3',null,null,el('span','shape shape-'+tone),el('span','group-label',label),el('span','group-count counter',null,document.createTextNode(String(rows.length)))));
        rows.forEach(row=>{
          const line=taskLine(row),stage=taskStage(row);
          const b=key(button(null,'task-item',()=>{views.taskSelected=row.id;remember();setArea('tasks',state.mode);},row.title),row.id);b.setAttribute('aria-pressed',String(row.id===views.taskSelected));b.setAttribute('data-tooltip',L(row.title)+' · '+line.text);
          const steps=el('span','task-steps');steps.setAttribute('aria-hidden','true');STAGES.forEach((_,i)=>steps.append(el('span',i<stage.at||stage.at===5?'done':i===stage.at?(stage.failed?'failed':stage.needs?'needs':'current'):'')));
          b.append(el('span','task-avatar tone-'+line.tone,null,el('span','agent-dot '+(AGENT_KIND[row.engine]??'other'))),el('span','task-item-copy',null,el('strong',null,row.title),el('small','task-line tone-'+line.tone,null,el('span','shape shape-'+line.shape),el('span',null,null,document.createTextNode(line.text))),el('small',null,null,document.createTextNode([row.repo,row.engine||L('No owner yet')].join(' · '))),steps));
          section.append(b);
        });
        list.append(section);
      }
      if(!list.children.length)list.append(el('p','pane-note','Nothing here matches your search.'));
    }
    paintList();
    const row=tasks.find(t=>t.id===views.taskSelected)??tasks[0];if(!row){main.append(el('p','pane-note','Choose a task.'));return;}
    const r=row.run,stage=taskStage(row);
    main.append(el('header','task-head',null,el('div','task-title',null,el('h2',null,row.title),el('span','task-meta',null,el('span','agent-dot '+(AGENT_KIND[row.engine]??'other')),el('span',null,null,document.createTextNode([row.engine||L('No owner yet'),row.repo,...(row.issue?[row.issue]:[]),...(row.cost?[L(row.cost)]:[])].join(' · '))),...(row.onWire?[chip('working','On the wire')]:[]))),el('div','page-actions',null,...taskActions(row))));
    const rail=el('ol','task-timeline');rail.setAttribute('aria-label',L('Progress'));
    STAGES.forEach(([id,label],i)=>{const state_=i<stage.at||stage.at===5?'done':i===stage.at?(stage.failed?'failed':stage.needs?'needs':'current'):'todo';rail.append(el('li','stage stage-'+state_,null,el('span','stage-dot'),el('span',null,label)));});
    main.append(rail);
    if(row.router)main.append(el('p','router-line',null,A.icon('branch',12),el('span',null,row.router)));
    detailPanel(main,{id:row.id,tabs:TASK_TABS,current:TASK_TABS.some(([id])=>id===views.taskDetail)?views.taskDetail:'run',onTab:tab=>{views.taskDetail=tab;remember();setArea('tasks',state.mode);},render:(panel,setScroll)=>{
      const tab=TASK_TABS.some(([id])=>id===views.taskDetail)?views.taskDetail:'run';
      if(tab==='run'){
        const live=el('section','task-section live-output',null,el('h3','lab-panel-title',r&&['working','thinking','approval'].includes(r.status)?'Live output':'Output'));
        const term=el('div','terminal');
        if(r){r.steps.forEach((step,i)=>term.append(el('div','term-line',null,el('span','counter',null,document.createTextNode(String(r.step_count-r.steps.length+i+1).padStart(2,' '))),el('code',null,null,document.createTextNode(step)))));if(r.output)term.append(el('div','term-line say',null,el('span',null,'›'),el('code',null,null,document.createTextNode(L(r.output)))));}
        else term.append(el('p','pane-note',row.output??'No run yet. Start the task to see its agent work here.'));
        live.append(term);if(r&&['approval','question'].includes(r.status))live.append(requestCard(r,'lab',()=>setArea('tasks',state.mode)));
        panel.append(live);
        const checks=el('section','task-section',null,el('h3','lab-panel-title',phrase('Done when… {done} of {all}',{done:row.checked.filter(Boolean).length,all:row.criteria.length})));
        row.criteria.forEach((text,i)=>{
          const c=el('input');c.type='checkbox';c.checked=!!row.checked[i];c.addEventListener('change',()=>{row.checked[i]=c.checked;remember();setArea('tasks',state.mode);});
          const result=row.checks[i],runnable=/\b(npm|cargo|pnpm|retry::)/.test(String(row.criteria[i]));
          const line=el('div','check-line',null,el('label',null,null,c,el('span',null,text)));
          if(result)line.append(chip(result==='passed'?'done':result==='failed'?'failed':'working',result==='passed'?'Passed':result==='failed'?'Failed':'Running'));
          if(runnable&&result!=='running')line.append(key(button(result?'Run again':'Run the check','',()=>runCheck(row,i)),'run-check-'+i));
          checks.append(line);
        });
        if(!row.criteria.length)checks.append(el('p','pane-note','No criteria yet. Add them in Edit task.'));
        panel.append(checks);
        if(row.rounds.length){const review=el('section','task-section',null,el('h3','lab-panel-title','Review'));row.rounds.forEach(rd=>review.append(el('div','review-bubble',null,el('span','agent-dot '+(AGENT_KIND[rd.by]??'other')),el('div',null,null,el('div','review-by',null,el('strong',null,rd.by),chip(['Approved','Fixed'].includes(rd.verdict)?'done':'needs',rd.verdict)),el('p',null,rd.note)))));panel.append(review);}
      }
      else if(tab==='diff')paintDiff(panel,r,setScroll);
      else if(tab==='context')panel.append(contextList([['Project',row.repo],['Branch',r?.branch??'vults/'+row.id],['Clone',r?.cwd??'Created when it starts'],['Owner',row.engine||'No owner yet'],['On the wire',row.onWire?'Yes':'No'],['Issue',row.issue||'None'],['Cost',row.cost||'Not started']]));
      else zecaTab(panel,row.repo,[phrase('Review the task: {task}',{task:L(row.title)})]);
    }});
  }
