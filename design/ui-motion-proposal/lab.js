  const labFiles=['render.ts','views.ts','ticker.ts','scene.ts','chat.ts','flock.ts','board.ts','seen.ts','quick.ts','behavior.ts','markdown.ts','icons.ts','daypart.ts','geometry.ts','sound.ts','session.ts'].map((name,index)=>{
    const lines=[' import { SessionView } from "../bridge";',' import { t } from "../i18n";',' '];
    for(const action of ['Selection','Draft','ReadingPosition','PendingRequest'])lines.push(
      ' export function restore'+action+'(session: SessionView) {',
      '-  const saved = lastSession;',
      '-  return saved;',
      '+  const key = session.agent + ":" + session.id;',
      '+  const saved = sessionState.get(key);',
      '+  if (!saved) return null;',
      '+  return { ...saved, project: session.project };',
      ' }',' ',
      ' export function keep'+action+'(session: SessionView) {',
      '-  lastSession = session;',
      '+  sessionState.set(session.agent + ":" + session.id, session);',
      ' }',' ');
    if(index===0)lines.push('+  const sameConversation = previous.session === current.session && previous.project === current.project && previous.executor === current.executor && previous.branch === current.branch && previous.source === current.source && previous.request === current.request;');
    return {path:'ui/src/island/'+name,hunks:[{old_start:20+index*10,new_start:20+index*10,lines}]};
  });
  const labChanges=[{step:4,files:labFiles.slice(0,2)},{step:6,files:labFiles.slice(0,6)},{step:8,files:labFiles}];
  const labTotals=files=>files.reduce((sum,file)=>{for(const h of file.hunks)for(const line of h.lines){if(line.startsWith('+'))sum.added++;if(line.startsWith('-'))sum.removed++;}return sum;},{added:0,removed:0});
  const labPositions=new Map();
  let captureLabPosition=()=>{};
  function renderFlock(host){
    host.classList.add('lab-flock');
    const tabs=[['session','Session'],['diff','Changes'],['activity','Activity'],['context','Context'],['zeca','Zeca']];
    if(!tabs.some(([id])=>id===views.flockDetail)||!settings.zeca&&views.flockDetail==='zeca')views.flockDetail='session';
    const pending=key(button('Review request','quiet-control',()=>{captureLabPosition();const first=flockFacts().needs[0];if(!first)return;views.sessionSelected=first.id;views.flockDetail='session';paint();remember();}),'lab-pending');
    const heading=el('header','lab-head'),picker=choose('Selected session',sessions.map(s=>({value:s.id,label:s.project+' · '+labelFor(s.agent)})),views.sessionSelected,value=>{
      captureLabPosition();views.sessionSelected=value;paint();remember();
    }),status=el('span','lab-status');
    heading.append(el('strong',null,'Flock'),picker,status,pending);
    const panel=el('section','lab-panel'),nav=el('nav','lab-bottom-tabs');panel.id='lab-session-panel';panel.setAttribute('role','tabpanel');nav.setAttribute('role','tablist');nav.setAttribute('aria-label',L('Session details'));
    host.append(heading,panel,nav);let positionKey='',scrollNode=panel;
    captureLabPosition=()=>{if(positionKey){labPositions.set(positionKey,{top:scrollNode.scrollTop,left:scrollNode.scrollLeft??0});if(labPositions.size>50)labPositions.delete(labPositions.keys().next().value);}};
    function selected(){return sessions.find(s=>s.id===views.sessionSelected)??sessions[0];}
    function switchTab(id,focus=false){captureLabPosition();views.flockDetail=id;paint();if(focus)nav.querySelector('[data-key="lab-'+id+'"]')?.focus({preventScroll:true});remember();}
    function paintHeading(s){picker.value=s.id;status.textContent=L(statusFor(s.status));pending.hidden=!flockFacts().needs.length;}
    function paint(){
      const s=selected();if(!s){panel.replaceChildren(el('p','pane-note','No sessions right now.'));return;}
      paintHeading(s);
      stopScenes();panel.replaceChildren();panel.classList.toggle('lab-panel-diff',views.flockDetail==='diff');nav.replaceChildren();
      for(const [id,label] of tabs){
        if(id==='zeca'&&!settings.zeca)continue;
        const b=key(button(label,'',()=>switchTab(id)),'lab-'+id);b.id='lab-tab-'+id;b.setAttribute('role','tab');b.setAttribute('aria-selected',String(id===views.flockDetail));b.setAttribute('aria-controls',panel.id);b.tabIndex=id===views.flockDetail?0:-1;nav.append(b);
      }
      panel.setAttribute('aria-labelledby','lab-tab-'+views.flockDetail);scrollNode=panel;
      if(views.flockDetail==='session')renderFlockSummary(panel,session=>{paintHeading(session);positionKey=session.id+':session';});
      else if(views.flockDetail==='diff')paintDiff(s);
      else if(views.flockDetail==='activity'){
        panel.append(el('h3','lab-panel-title','Activity'));
        s.steps.forEach((step,index)=>panel.append(el('div','lab-step',null,el('span','counter',String(s.step_count-s.steps.length+index+1)),el('code',null,step))));
      }else if(views.flockDetail==='context'){
        panel.append(el('h3','lab-panel-title','Context'));
        const context=el('dl','lab-context');for(const [label,value] of [['Project',s.project],['Repository',s.roost],['Branch',s.branch],['Folder',s.cwd],['Agent',labelFor(s.agent)],['Editor',s.editor],['Status',statusFor(s.status)]])context.append(el('dt',null,label),el('dd',null,value));panel.append(context);
      }else panel.append(el('h3','lab-panel-title','Zeca'),el('p','pane-note','Ask about the selected session. Your conversation and draft stay together.'),key(button('Ask Zeca','primary',()=>askSession(s.project)),'lab-ask'));
      positionKey=s.id+':'+views.flockDetail+(views.flockDetail==='diff'?':'+views.flockStep+':'+views.flockFile:'');
      const position=labPositions.get(positionKey);scrollNode.scrollTop=position?.top??0;scrollNode.scrollLeft=position?.left??0;
    }
    function paintDiff(s){
      if(!['session-1','session-2'].includes(s.id)){panel.append(el('p','pane-note','No changes kept for this session yet.'));return;}
      const change=labChanges.find(c=>c.step===views.flockStep)??labChanges.at(-1);views.flockStep=change.step;
      if(views.flockFile!=='all'&&!change.files.some(f=>f.path===views.flockFile))views.flockFile='all';
      const toolbar=el('div','lab-diff-toolbar');
      const step=choose('The change shown',labChanges.map(c=>({value:String(c.step),label:phrase('Step {step} · {n} files',{step:c.step,n:c.files.length})})),String(change.step),value=>{captureLabPosition();views.flockStep=Number(value);paint();remember();});
      const file=choose('Changed file',[{value:'all',label:phrase('All {n} files',{n:change.files.length})},...change.files.map(f=>({value:f.path,label:f.path}))],views.flockFile,value=>{captureLabPosition();views.flockFile=value;paint();remember();});
      const totals=labTotals(change.files),count=el('span','lab-diff-totals counter',null,el('span','diff-add','+'+totals.added),el('span','diff-remove','−'+totals.removed));
      toolbar.append(step,file,count);panel.append(toolbar);
      const reader=el('div','lab-diff-scroll');reader.setAttribute('role','region');reader.setAttribute('aria-label',L('Code changes'));scrollNode=reader;
      for(const f of change.files.filter(f=>views.flockFile==='all'||f.path===views.flockFile)){
        const section=el('section','lab-diff-file'),sum=labTotals([f]);
        section.append(el('header','lab-file-head',null,el('code',null,f.path),el('span','counter',null,el('span','diff-add','+'+sum.added),el('span','diff-remove','−'+sum.removed))));
        for(const h of f.hunks){
          section.append(el('div','lab-hunk mono','@@ −'+h.old_start+' +'+h.new_start+' @@'));let old=h.old_start,next=h.new_start;
          for(const line of h.lines){
            const plus=line.startsWith('+'),minus=line.startsWith('-');
            section.append(el('div','lab-code-row '+(plus?'added':minus?'removed':'context'),null,el('span','lab-line counter',plus?'':String(old)),el('span','lab-line counter',minus?'':String(next)),el('span','lab-mark',line[0].trim()),el('code',null,line.slice(1))));
            if(!plus)old++;if(!minus)next++;
          }
        }
        reader.append(section);
      }
      panel.append(reader,el('div','lab-diff-footnote','Sample patch. No project files were changed.'));
    }
    nav.addEventListener('keydown',event=>{
      if(event.isComposing||event.keyCode===229||!['ArrowLeft','ArrowRight','Home','End'].includes(event.key))return;
      event.preventDefault();const available=tabs.filter(([id])=>id!=='zeca'||settings.zeca),at=available.findIndex(([id])=>id===views.flockDetail);
      const next=event.key==='Home'?0:event.key==='End'?available.length-1:(at+(event.key==='ArrowRight'?1:-1)+available.length)%available.length;switchTab(available[next][0],true);
    });
    paint();
  }
  $('[data-lab-diff]').addEventListener('click',()=>{views.sessionSelected='session-2';views.flockDetail='diff';views.flockFile='all';views.flockStep=8;state.scenario='ready';setArea('flock','room');});
