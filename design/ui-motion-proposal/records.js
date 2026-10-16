  /* Lab · Activity: six views of the local records. Every figure here is sample data. */
  const ACTIVITY_TABS=[['week','Week'],['projects','By project'],['history','History'],['cost','Cost'],['traces','Traces'],['audit','Audit']];
  const DAYS=['Mon','Tue','Wed','Thu','Fri','Sat','Sun'];
  function renderActivity(host) {
    host.append(pageHeading('Activity',null));
    host.append(tabs(ACTIVITY_TABS,state.activity,id=>{state.activity=id;views.recordQuery='';setArea('activity',state.mode);},'Activity','activity-tabs'));
    if(stageState(host,'No activity yet.'))return;
    if(settings.recordsCleared&&!['projects','cost'].includes(state.activity)&&!(state.activity==='audit'&&auditRows.length)){host.append(el('section','empty-state',null,el('h3',null,'No activity yet.'),el('p',null,'New turns appear here as your agents work.')));return;}
    if(!settings.history&&['week','history','traces'].includes(state.activity)){host.append(el('section','empty-state',null,el('h3',null,'History is off'),el('p',null,'Vults keeps no record of turns while it is off. Audit always stays.'),button('Open activity settings','quiet-control',()=>openSettings('activity'))));return;}
    const panel=el('div','activity-content');host.append(panel);
    const view={week:renderWeek,projects:renderProjects,cost:renderCost,traces:renderTraces}[state.activity];
    if(view)view(panel);else renderRecords(panel,state.activity);
  }
  function renderWeek(host) {
    const values=views.week===0?[2,4,3,6,5,0,0]:[3,2,5,4,2,1,0];
    const heading=el('h3',null,views.week===0?'This week':phrase('{n} weeks ago',{n:-views.week}));
    const next=button('Next week ›','',()=>{if(views.week<0){views.week++;setArea('activity',state.mode);}});next.disabled=views.week===0;
    host.append(el('div','week-nav',null,button('‹ Previous week','',()=>{views.week--;setArea('activity',state.mode);}),heading,next,key(button('Save as image','quiet-control',()=>{const hide=el('input');hide.type='checkbox';openEditor('Save as image',el('div',null,null,el('p','pane-note','A 1080\u00d71350 card with this week\'s figures.'),el('label','check-row',null,hide,el('span',null,'Hide project names'))),()=>{toast(hide.checked?'Saved without project names':'Saved to Pictures');return true;},'Save image');}),'save-image')));
    host.append(el('div','week-summary stat-tiles',null,...[[String(values.reduce((a,b)=>a+b,0)),'Agent turns'],['4','Projects'],['3 h 12 min','Agent time'],['2','Needed you']].map(([value,label])=>el('div','stat-tile',null,el('strong','counter',null,document.createTextNode(L(value))),el('span',null,label)))));
    const chart=el('div','week-chart');chart.setAttribute('role','group');chart.setAttribute('aria-label',L('Agent turns by day'));
    const max=Math.max(...values,1);
    DAYS.forEach((day,i)=>{const b=button(null,'week-day'+(views.day===i?' selected':''),()=>{views.day=i;setArea('activity',state.mode);});b.setAttribute('aria-pressed',String(views.day===i));b.setAttribute('aria-label',phrase('{day}: {n} turns',{day:L(day),n:values[i]}));const bar=el('span','day-bar');bar.style.height=(values[i]/max*96)+'px';b.append(el('span','counter',null,document.createTextNode(String(values[i]))),bar,el('span',null,day));chart.append(b);});
    host.append(chart,el('div','week-records',null,...[['Busiest day',DAYS[values.indexOf(Math.max(...values))]],['Longest turn','24 min · frontend'],['Most turns','Vults · 9']].map(([label,value])=>el('div',null,null,el('small',null,label),el('strong',null,null,document.createTextNode(L(value)))))));
    host.append(details('How activity is counted',['A turn is one answer from an agent, from your prompt to its stop.','Agent time adds up the turns, not the time sessions stay open.','Nothing leaves this computer.']));
    const grid=el('div','year-grid');grid.setAttribute('aria-label',L('The last 52 weeks'));for(let i=0;i<364;i++){const cell=el('span','year-cell level-'+((i*7+i%5)%9<4?0:(i*3)%4+1));grid.append(cell);}
    host.append(el('h3','lab-panel-title','The last 52 weeks'),grid,el('p','pane-note',settings.github?'With GitHub on, a second tab shows your contribution calendar.':'Turn GitHub on to compare with your contribution calendar.'));
  }
  function renderProjects(host) {
    const list=el('div'), finder=search('Search projects',value=>{views.recordQuery=value;paint();remember();});finder.querySelector('input').value=views.recordQuery;host.append(el('div','workspace-toolbar',null,finder),list);
    const ROOSTS=[['Vults','4 h 10 min',['main','feat/compact-now'],2,'$1.80','$14.20'],['frontend','2 h 05 min',['fix/chat-scroll'],1,'$0.60','$9.10'],['docs','40 min',['docs/flock-badges'],1,'$0.18','$2.40'],['api','25 min',['fix/retry-flake'],1,'Not reported','$0.90']];
    function paint(){list.replaceChildren();ROOSTS.filter(([p])=>p.toLowerCase().includes(views.recordQuery.toLowerCase())).forEach(([project,time,branches,count,today,total])=>{
      const s=sessions.find(s=>s.project===project),fact=memory.facts.find(f=>f.place===project);
      const item=key(el('article','content-row roost',null,el('header',null,null,el('h3',null,null,document.createTextNode(project)),s?sessionChip(s):null),el('p','task-meta',null,document.createTextNode(phrase('{time} this week · {n} sessions · today {today} · total {total}',{time:L(time),n:count,today:L(today),total}))),el('p','task-meta',null,document.createTextNode('~/Projects/'+project+' · '+branches.join(', ')))),'roost-'+project);
      if(fact)item.append(el('p',null,null,A.icon('sideRight',12),document.createTextNode(' '+L(fact.text))));
      item.append(el('div','inline-actions',null,button('Open session','quiet-control',()=>{views.sessionSelected=s?.id??'session-1';setArea('flock','room');}),button('Ask Zeca','',()=>askSession(project)),button('Share it with Claude…','',()=>shareAgents(project)),button('Preferences ↗','',()=>openSettings('projects'))));list.append(item);
    });if(!list.children.length)list.append(el('p','pane-note','Nothing here matches your search.'));}paint();
  }
  // AGENTS.md is the shared memory; Claude reads it once CLAUDE.md imports it (rule 3: diff, backup, click).
  function shareAgents(project){
    openEditor('Share it with Claude',el('div',null,null,el('p','pane-note',phrase('{project}\'s CLAUDE.md gains one line. A dated backup is kept.',{project})),el('pre','diff-preview',null,el('span','diff-add',null,document.createTextNode('+ @AGENTS.md\n')),document.createTextNode('  # '+project+'\n  Existing instructions stay.'))),()=>{toast('Written. Backup kept next to the file.');return true;},'Apply');
  }
  function renderCost(host) {
    settings.dailyBudget??=10;const share=2.4/settings.dailyBudget;
    host.append(el('section','cost-block',null,el('h3',null,'Daily budget'),el('p','pane-note','API estimates only. At 80% you get a notification; when it is spent, new task starts wait. Running turns finish.'),el('strong','counter',null,document.createTextNode('$2.40 / $'+Number(settings.dailyBudget).toFixed(2)))));
    const progress=el('progress','budget-progress');progress.value=2.4;progress.max=settings.dailyBudget;progress.setAttribute('aria-label',L('Daily budget'));progress.classList.toggle('warning',share>=.7);progress.classList.toggle('critical',share>=.9);
    host.append(progress,button('Edit budget','quiet-control',()=>{const budget=field('Daily budget (USD)',String(settings.dailyBudget),'number');budget.control.min='.01';budget.control.step='.01';openEditor('Daily budget',budget.element,()=>{const n=Number(budget.control.value);if(!Number.isFinite(n)||n<=0)throw new Error('Enter a positive amount.');settings.dailyBudget=n;setArea('activity',state.mode);return true;});}));
    host.append(el('h3','lab-panel-title','Plan windows'),...PLANS.map(plan=>el('div','plan-row',null,el('span','agent-dot '+plan.agent),el('span',null,raw(plan.name)),el('span','plan-bar',null,el('span',plan.used>=90?'critical':plan.used>=70?'warning':'')),el('small','counter',null,document.createTextNode(phrase('{n}% of the 5 h window',{n:plan.used}))))));
    host.querySelectorAll('.plan-bar span').forEach((bar,i)=>bar.style.width=PLANS[i].used+'%');
    const table=el('table','record-table'),body=el('tbody');table.append(el('thead',null,null,el('tr',null,null,...['Project','Today','Two weeks','Tokens'].map(word=>el('th',null,word)))),body);
    [['Vults','$1.80','$14.20','1.9M'],['frontend','$0.60','$9.10','1.1M'],['docs','$0.18','$2.40','240k'],['api','Not reported','$0.90','—']].forEach(values=>body.append(el('tr',null,null,...values.map(value=>el('td','counter',null,document.createTextNode(L(value)))))));
    host.append(el('h3','lab-panel-title','By project'),table,el('p','pane-note','Amber at 70% · red at 90%. Subscription plans are not billed per token.'));
  }
  const TRACES=[{time:'10:44',model:'Sonnet',tokens:'4.2k',cost:'$0.02',tools:[['flock_list','0.1 s'],['session_detail("Vults")','0.2 s']]},{time:'10:31',model:'Sonnet',tokens:'9.8k',cost:'$0.05',tools:[['github_snapshot','0.4 s'],['web_fetch("docs.github.com")','1.3 s']]},{time:'09:12',model:'Haiku',tokens:'1.1k',cost:'$0.00',tools:[['recall("build")','0.1 s']]}];
  function renderTraces(host){
    host.append(el('p','pane-note','Zeca\'s chat turns: model, tokens, cost and each tool call with its wait. Kept 30 days.'));
    TRACES.forEach(t=>{const d=details(phrase('{time} · {model} · {tokens} tokens · {cost}',{time:t.time,model:t.model,tokens:t.tokens,cost:t.cost}),t.tools.map(([name,wait])=>el('div',null,null,el('code',null,null,document.createTextNode(name)),el('small','counter',null,document.createTextNode(wait)))));host.append(d);});
  }
  function renderRecords(host,kind) {
    const rows=kind==='audit'?auditRows:[{time:'10:42',project:'Vults',agent:'Codex',answer:'Finished',by:'',command:'12 steps · 8 min'},{time:'10:31',project:'api',agent:'OpenCode',answer:'Failed',by:'',command:'cargo test -p api'},{time:'10:20',project:'docs',agent:'Codex',answer:'Finished',by:'',command:'6 steps · 4 min'},{time:'09:55',project:'frontend',agent:'Claude Code',answer:'Finished',by:'',command:'21 steps · 24 min'}];
    host.append(el('p','pane-note',kind==='audit'?'Who answered each card: you, your phone, an Always rule or a policy. Append-only, kept 90 days.':'Finished turns, counts only. The latest 200.'));
    const list=el('div'), finder=search(kind==='audit'?'Search answers and commands':'Search turns',value=>{views.recordQuery=value;paint();remember();});finder.querySelector('input').value=views.recordQuery;host.append(el('div','workspace-toolbar',null,finder,button('Clear filters','',()=>{views.recordQuery='';finder.querySelector('input').value='';paint();remember();})),list);
    function paint(){list.replaceChildren();const found=rows.filter(row=>Object.values(row).map(L).join(' ').toLowerCase().includes(views.recordQuery.toLowerCase()));
      if(!found.length){list.append(el('p','pane-note','Nothing here matches your search.'));return;}
      const table=el('table','record-table'),body=el('tbody');table.append(el('thead',null,null,el('tr',null,null,...['Time','Project','Agent',kind==='audit'?'Answer':'Result',...(kind==='audit'?['By','Command']:['Turn'])].map(word=>el('th',null,word)))),body);
      found.forEach(row=>{const b=button(null,'',()=>openEditor(kind==='audit'?'Audit':'History',el('div',null,null,el('h3',null,null,document.createTextNode(row.project)),el('p','pane-note',null,document.createTextNode(row.agent+' · '+L(row.answer)+(row.by?' · '+L(row.by):''))),el('pre','diff-preview',null,document.createTextNode(row.command)))));b.append(document.createTextNode(row.project));body.append(el('tr',null,null,el('td','counter',null,document.createTextNode(row.time)),el('td',null,null,b),el('td',null,null,document.createTextNode(row.agent)),el('td',null,null,chip(['Allowed','Finished','Answered'].includes(row.answer)?'done':['Denied','Failed'].includes(row.answer)?'failed':'quiet',row.answer)),...(kind==='audit'?[el('td',null,row.by),el('td',null,null,el('code',null,null,document.createTextNode(row.command)))]:[el('td',null,null,document.createTextNode(row.command))])));});list.append(table);
    }paint();
  }
