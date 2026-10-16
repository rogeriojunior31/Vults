  /* Connections: each connector has a face for you (what to see and do) and a face for Zeca (what he may read or do). */
  const CONNECTORS=[{id:'github',name:'GitHub',about:'Pull requests, checks, review requests and releases, through gh.'}];
  const SECTIONS=[['inbox','Inbox'],['pulls','Pull requests'],['actions','Actions'],['releases','Releases']];
  function openConnection(section,id){views.connectionFace='you';views.connectionSection=section;views.connectionSelected=id;views.connectionQuery='';setArea('connections','room');}
  const checkTone=value=>value==='passed'?'done':value==='failed'?'failed':'working';
  function renderConnections(host) {
    const refresh=key(button('Refresh','quiet-control',()=>{if(Date.now()-github.refreshedAt<60000){toast('Refreshed less than a minute ago.');return;}github.refreshedAt=Date.now();github.synced=0;github.requests++;setArea('connections',state.mode);toast('Synced with GitHub');}),'refresh');
    host.append(pageHeading('Connections',null,refresh,key(button('Add a connection','',()=>openCatalog()),'add-connection')));
    const picker=el('div','connector-chips');CONNECTORS.forEach(c=>{const b=key(button(null,'connector-chip',()=>{}),'connector-'+c.id);b.setAttribute('aria-pressed','true');b.append(A.icon('pull',13),el('span',null,raw(c.name)),el('span','shape shape-'+(settings.github?'done':'quiet')));picker.append(b);});host.append(picker);
    if(!settings.github){host.append(el('section','empty-state',null,el('h3',null,'GitHub is off'),el('p',null,'Turn it on to see your pull requests and checks. Vults then calls gh every 5 minutes, within its daily budget.'),button('Open connector settings','quiet-control',()=>openSettings('connectors'))));return;}
    host.append(el('p','sync-line pane-note',phrase('Synced {age} · {used} of {budget} requests today',{age:github.synced?phrase('{n} min ago',{n:github.synced}):L('just now'),used:github.requests,budget:'3,000'})));
    host.append(tabs([['you','For you'],['zeca','For Zeca']],views.connectionFace,face=>{views.connectionFace=face;remember();setArea('connections',state.mode);},'GitHub','face-tabs'));
    if(stageState(host,'No connection news yet.'))return;
    if(views.connectionFace==='zeca'){renderZecaFace(host);return;}
    const counts=Object.fromEntries(SECTIONS.map(([id])=>[id,github[id].filter(r=>r.isNew&&!settings.githubSeen[r.id]).length]));
    const finder=search('Search title, ref, repo, branch or author',value=>{views.connectionQuery=value;paint();remember();});finder.querySelector('input').value=views.connectionQuery;
    const sections=segment(SECTIONS.map(([id,label])=>[id,counts[id]?phrase(counts[id]===1?'{section} · 1 new':'{section} · {n} new',{section:L(label),n:counts[id]}):label]),value=>{views.connectionSection=value;views.connectionSelected='';paint();remember();},views.connectionSection);
    const markAll=key(button('Mark all seen','',()=>{for(const r of github[views.connectionSection])settings.githubSeen[r.id]=true;setArea('connections',state.mode);}),'connections-mark-all');
    const list=el('div','connection-list'),detail=el('article','connection-detail');
    const columns=el('div','connection-columns',null,list,detail);host.append(el('div','workspace-toolbar',null,sections,finder,markAll),columns);
    // The list repaints its rows, so its edge is added back each time.
    const edge=()=>flexible(columns,list,'conn',{def:420,min:280,max:680,label:'List',rail:()=>{const r=el('div','rail-marks');(github[views.connectionSection]??[]).slice(0,8).forEach(row=>{const b=button(null,'rail-dot',()=>{views.connectionSelected=row.id;paint();},row.title);b.setAttribute('data-tooltip',L(row.title));b.setAttribute('aria-pressed',String(row.id===views.connectionSelected));b.append(el('span','shape shape-'+(row.checks?checkTone(row.checks):'quiet')));r.append(b);});return r;}});
    function paint(){
      list.replaceChildren();const rows=github[views.connectionSection]??[];
      const found=rows.filter(row=>[L(row.title),row.repo,row.id,row.ref,row.branch,row.author].join(' ').toLowerCase().includes(views.connectionQuery.toLowerCase()));
      found.forEach(row=>{
        const b=key(button(null,'connection-row',()=>{views.connectionSelected=row.id;paint();enter(detail,'gentle',4,1);remember();}),row.id);b.setAttribute('aria-pressed',String(row.id===views.connectionSelected));
        const fresh=row.isNew&&!settings.githubSeen[row.id];
        b.append(el('span','shape shape-'+(row.checks?checkTone(row.checks):fresh?'news':'quiet')),el('div','connection-copy',null,el('strong',null,row.title),el('small',null,null,document.createTextNode([row.repo,row.ref??(views.connectionSection==='pulls'?row.id:''),row.kind?L(row.kind):'',row.age?phrase('{age} ago',{age:row.age}):''].filter(Boolean).join(' · ')))),...(fresh?[el('span','new-dot','New')]:[]));list.append(b);
      });
      if(!found.length){list.append(el('p','pane-note','Nothing here matches your search.'),button('Clear filters','quiet-control',()=>{views.connectionQuery='';finder.querySelector('input').value='';paint();}));detail.replaceChildren();return;}
      const selected=found.find(row=>row.id===views.connectionSelected)??found[0];views.connectionSelected=selected.id;paintDetail(selected);edge();
    }
    function paintDetail(row){
      detail.replaceChildren();const section=views.connectionSection;
      if(row.checks)detail.append(chip(checkTone(row.checks),row.merged?'Merged':row.checks==='passed'?'Checks passed':row.checks==='failed'?'Checks failed':'Checks running'));
      detail.append(el('h3',null,row.title),el('span','task-meta',null,document.createTextNode([row.repo,row.id.startsWith('#')?row.id:row.ref,row.branch,row.author&&'@'+row.author,row.size].filter(Boolean).join(' · '))));
      if(row.notes)detail.append(el('p',null,row.notes));
      const actions=el('div','detail-actions',null,button('Open in GitHub ↗','quiet-control',()=>toast(phrase('Opened {ref} in your browser',{ref:row.ref??row.id}))),key(button('Ask Zeca','primary',()=>prepareChat(phrase(section==='actions'?'Why is {repo} main red?':'Review {title}',{repo:row.repo,title:L(row.title)}))),'ask-zeca'));
      if(section==='pulls'){
        const merge=key(button('Squash and merge','quiet-control',()=>confirm('Squash and merge',phrase('Merges {ref} at its green commit. The action is traced.',{ref:row.id}),()=>{row.merged=true;paintDetail(row);remember();toast('Merged');},'default','Squash and merge')),'merge');
        merge.disabled=row.checks!=='passed'||row.merged;merge.setAttribute('data-tooltip',L(row.checks!=='passed'?'Merging waits for green checks.':'Merges the exact green commit.'));actions.append(merge);
      }
      if(row.isNew&&!settings.githubSeen[row.id])actions.append(key(button('✓ Seen','',()=>{settings.githubSeen[row.id]=true;paint();remember();}),'seen'));
      detail.append(actions);
      if(row.checkList){const checks=details(phrase('Checks · {n}',{n:row.checkList.length}),[...row.checkList].sort((a,b)=>(a[1]==='failed'?-1:0)-(b[1]==='failed'?-1:0)).map(([name,value])=>el('div',null,null,el('span','shape shape-'+checkTone(value)),el('span',null,raw(name)),el('small',null,value))));checks.open=row.checks==='failed';detail.append(checks);}
      detail.append(details('Technical details',[el('code','mono',null,document.createTextNode('gh pr view '+(row.id.startsWith('#')?row.id.slice(1):row.ref?.slice(1)??'')+' --repo you/'+row.repo+' --json statusCheckRollup')),el('p','pane-note','Copy-only. Vults never runs a command you type here.')]));
    }
    paint();
  }
  function renderZecaFace(host){
    host.append(el('p','pane-note','What Zeca may read or do with GitHub. Each tool has its own switch and leaves a trace. Acting always asks you first.'));
    const list=el('div','zeca-tools');host.append(list);
    zecaTools.forEach(tool=>{
      const toggle=button(null,'toggle',()=>{tool.on=!tool.on;toggle.setAttribute('aria-checked',String(tool.on));toast(tool.on?'Zeca may use it':'Zeca no longer uses it');remember();},phrase('Let Zeca use {tool}',{tool:tool.name}));toggle.setAttribute('role','switch');toggle.setAttribute('aria-checked',String(tool.on));toggle.append(el('span'));
      list.append(key(el('article','zeca-tool',null,el('div','setting-copy',null,el('strong',null,null,el('code',null,null,document.createTextNode(tool.name)),chip(tool.kind==='Read'?'quiet':'needs',tool.kind==='Read'?'Reads':'Acts, asks first')),el('small',null,tool.about),el('small','counter',phrase('Last used {when} · {n} calls today',{when:L(tool.used),n:tool.calls}))),el('div','inline-actions',null,toggle,button('Traces','',()=>{state.activity='traces';setArea('activity',state.mode);}))),'tool-'+tool.id));
    });
    host.append(setting('Budget for Zeca','His GitHub reads count against the connector\'s 3,000 daily requests.',el('span','counter',null,document.createTextNode('7 / 3,000'))));
  }
  function openCatalog(){
    const list=el('div','catalog');
    list.append(el('article','content-row',null,el('header',null,null,el('h3',null,'GitHub'),chip(settings.github?'done':'quiet',settings.github?'On':'Off')),el('p',null,CONNECTORS[0].about),button('Manage in Settings','quiet-control',()=>{$('.editor-close').click();openSettings('connectors');})));
    list.append(el('p','pane-note','Each new connector arrives with both faces: what you see and what Zeca may use, with its own switch and budget. docs/dev has the contributing guide.'));
    openEditor('Add a connection',list);
  }
