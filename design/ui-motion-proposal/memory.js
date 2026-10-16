  /* Memory is Zeca's, so it opens beside his chat in the Lab: proposals first, then what he keeps. */
  function paintMemoryPanel(){
    const host=$('.room-memory'),room=['room','full'].includes(state.mode);
    host.hidden=!room||!state.memoryPanel||state.area!=='chat';$('.chat-shell').classList.toggle('with-memory',!host.hidden);memoryTrigger.setAttribute('aria-pressed',String(!host.hidden));
    if(host.hidden)return;
    host.replaceChildren(el('div','recent-head',null,el('h3',null,'Memory'),iconButton('close','Close',()=>toggleMemoryPanel(false))));
    if(!settings.memory){host.append(el('section','empty-state',null,el('h3',null,"Zeca's memory is off."),button('Open memory settings','quiet-control',()=>openSettings('chat'))));return;}
    if(memory.debrief&&settings.sleep)host.append(el('p','memory-debrief',null,A.icon('moon',12),el('span',null,memory.debrief)));
    host.append(segment([['Waiting for you',phrase('Waiting · {n}',{n:memory.pending.length})],['What Zeca remembers','Remembers'],['History','History']],value=>{views.memoryTab=value;paintMemoryPanel();remember();},views.memoryTab));
    const body=el('div','memory-body');host.append(body);
    if(views.memoryTab==='Waiting for you'){
      if(!memory.pending.length)body.append(el('p','pane-note','Nothing waits for you.'));
      memory.pending.forEach(proposal=>{
        const act=accepted=>{memory.pending.splice(memory.pending.indexOf(proposal),1);if(accepted){memory.facts=memory.facts.filter(f=>!(proposal.current&&f.text===proposal.current));memory.facts.push({id:proposal.id,place:proposal.place,text:proposal.text});}memory.history.unshift({...proposal,accepted});paintMemoryPanel();paintNav();toast(accepted?'Remembered':'Turned down');remember();};
        const change=el('div','memory-change');
        if(proposal.current)change.append(el('small',null,'Current'),el('p','diff-remove',proposal.current));
        change.append(el('small',null,proposal.current?'Proposed':'New memory'),el('p',proposal.current?'diff-add':null,proposal.text));
        body.append(key(el('article','content-row',null,el('h3',null,proposal.place),change,el('div','inline-actions',null,key(button('Turn down','quiet-control',()=>act(false)),'turn-down'),key(button(proposal.current?'Accept change':'Remember','primary',()=>act(true)),'remember'))),proposal.id));
      });
    }else if(views.memoryTab==='What Zeca remembers'){
      const files=el('div','memory-files');for(const [name,used] of memory.files){const bar=el('span','plan-bar',null,el('span',used>=90?'critical':used>=70?'warning':''));bar.firstChild.style.width=used+'%';files.append(el('div','plan-row',null,el('code',null,null,document.createTextNode(name)),bar,el('small','counter',null,document.createTextNode(used+'%'))));}
      body.append(files);
      memory.facts.forEach(fact=>body.append(key(el('article','content-row',null,el('h3',null,fact.place),el('p',null,fact.text),button('Remove','',()=>confirm('Remove memory','Zeca forgets this note. History can bring it back.',()=>{memory.facts.splice(memory.facts.indexOf(fact),1);memory.history.unshift({...fact,accepted:false,removed:true});paintMemoryPanel();},'default','Remove'))),fact.id)));
      if(!memory.facts.length)body.append(el('p','pane-note','Nothing yet.'));
    }else{
      memory.history.forEach(item=>body.append(el('article','content-row',null,el('h3',null,item.removed?'Removed':item.accepted?'Remembered':'Turned down'),el('p',null,item.text),button('Undo','quiet-control',()=>{memory.history.splice(memory.history.indexOf(item),1);memory.facts=memory.facts.filter(f=>f.id!==item.id);if(item.removed)memory.facts.push({id:item.id,place:item.place,text:item.text});else memory.pending.push({id:item.id,place:item.place,text:item.text,current:item.current??null});paintMemoryPanel();paintNav();}))));
      if(!memory.history.length)body.append(el('p','pane-note','Nothing yet.'));
    }
  }
