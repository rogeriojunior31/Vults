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
    const searcher=search('Search…',q=>{views.sessionQuery=q;paintRows();remember();});searcher.querySelector('input').value=views.sessionQuery;
    const grouping=segment([['state','By state'],['project','By project']],value=>{views.sessionGroup=value;paintRows();remember();},views.sessionGroup);grouping.classList.add('group-by');
    const rows=el('div','lab-rows');list.append(el('div','workspace-toolbar',null,searcher,grouping),rows);
    flexible(host,list,'flock-list',{def:380,min:260,max:560,label:'Sessions',rail:()=>{const r=el('div','rail-marks');facts.ordered.filter(s=>!settings['hide-'+s.project]).slice(0,8).forEach(s=>{const b=key(button(null,'rail-dot',()=>select(s),s.project),'rail-'+s.id);b.setAttribute('data-tooltip',s.project+' · '+L(sessionBrief(s)));b.setAttribute('aria-pressed',String(s.id===views.sessionSelected));b.append(el('span','shape shape-'+(toneFor(s)==='quiet'?'quiet':s.status==='question'?'question':toneFor(s))));r.append(b);});return r;}});
    const groupOf=s=>settings['pin-'+s.project]?'pinned':soaring(s)?'sky':facts.needs.includes(s)||s.status==='failed'?'needs':facts.working.includes(s)?'working':'quiet';
    const GROUPS=[['pinned','Pinned','quiet'],['needs','Needs you','needs'],['working','Working','working'],['quiet','Gone quiet','quiet'],['sky','In the sky','done']];
    function row(s){
      const b=key(button(null,'session-row',()=>select(s)),s.id);b.setAttribute('aria-pressed',String(s.id===views.sessionSelected));bindBirdFocus(b,flock,s);
      b.append(el('span','agent-dot '+s.agent),el('span','session-copy',null,el('strong',null,s.project),el('small',null,s.task?raw(labelFor(s.agent)+' · '+L(s.started)):labelFor(s.agent))),soaring(s)?chip('done','In the sky'):sessionChip(s));
      if(settings['pin-'+s.project])b.append(A.icon('pin',11));return b;
    }
    function paintRows(){
      rows.replaceChildren();const found=facts.ordered.filter(s=>!settings['hide-'+s.project]&&(s.project+' '+labelFor(s.agent)).toLowerCase().includes(views.sessionQuery.toLowerCase()));
      const groups=views.sessionGroup==='project'?[...new Set(found.map(s=>s.project))].sort().map(p=>[p,p,null]):GROUPS;
      for(const [id,label,tone] of groups){
        const members=found.filter(s=>views.sessionGroup==='project'?s.project===id:groupOf(s)===id);if(!members.length)continue;
        const section=el('section','task-group');section.append(el('h3',null,null,tone?el('span','shape shape-'+tone):A.icon('folder',12),el('span','group-label',null,document.createTextNode(views.sessionGroup==='project'?id:L(label))),el('span','group-count counter',null,document.createTextNode(String(members.length)))));
        members.forEach(s=>section.append(row(s)));rows.append(section);
      }
      if(!found.length)rows.append(el('p','pane-note','Nothing here matches your search.'),button('Clear filters','quiet-control',()=>{views.sessionQuery='';setArea('flock',state.mode);}));
    }
    function select(s){views.sessionSelected=s.id;views.detailOpen.flock=true;setArea('flock',state.mode);focusDetail(host,s.id);}
    paintRows();
    const s=sessions.find(s=>s.id===views.sessionSelected)??facts.ordered[0];if(!s)return;
    if(!DETAIL_TABS.some(([id])=>id===views.flockDetail)||!settings.zeca&&views.flockDetail==='zeca')views.flockDetail='session';
    const more=key(iconButton('list','Quick actions',()=>sessionActions(more,s)),'lab-more');
    detail.append(el('header','detail-head',null,el('div',null,null,el('h3',null,s.project),el('span','task-meta',null,el('span',null,raw(labelFor(s.agent)+' · '+s.branch)),sessionChip(s))),el('div','page-actions',null,more)));
    detailPanel(detail,{id:s.id,tabs:DETAIL_TABS,current:views.flockDetail,onTab:tab=>{views.flockDetail=tab;remember();setArea('flock',state.mode);},render:(panel,setScroll)=>sessionTab(panel,s,views.flockDetail,setScroll)});
    listDetail(host,list,detail,s.id,'Back to sessions');
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
