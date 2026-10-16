(() => {
  const root = document.getElementById('vults-ui-proposal');
  if (!root) return;
  const A = VultsProposalAssets;
  A.setLang('pt-BR');
  const copy = {
    'Interactive prototype': ['Proposta interativa', 'Propuesta interactiva', '\u4ea4\u4e92\u539f\u578b'],
    'Sample data': ['Dados de exemplo', 'Datos de ejemplo', '\u793a\u4f8b\u6570\u636e'],
    'Tasks in Room': ['Tasks na Sala', 'Tareas en Sala', '\u623f\u95f4\u4e2d\u7684\u4efb\u52a1'],
    'Connections in Room': ['Conex\u00f5es na Sala', 'Conexiones en Sala', '\u623f\u95f4\u4e2d\u7684\u8fde\u63a5'],
    'Settings in Room': ['Configura\u00e7\u00f5es na Sala', 'Ajustes en Sala', '\u623f\u95f4\u4e2d\u7684\u8bbe\u7f6e'],
    'Open settings in Room': ['Abrir configura\u00e7\u00f5es na Sala', 'Abrir ajustes en Sala', '\u5728\u623f\u95f4\u4e2d\u6253\u5f00\u8bbe\u7f6e'],
    'Back to Compact': ['Voltar ao Compact', 'Volver a Compact', '\u8fd4\u56de Compact'],
    'Empty': ['Vazio', 'Vac\u00edo', '\u7a7a\u767d'],
    'Conversation': ['Conversa', 'Conversaci\u00f3n', '\u5bf9\u8bdd'],
    'Models': ['Modelos', 'Modelos', '\u6a21\u578b'],
    'Notification': ['Notifica\u00e7\u00e3o', 'Notificaci\u00f3n', '\u901a\u77e5'],
    'Your project, one conversation away.': ['Seu projeto, a uma conversa de dist\u00e2ncia.', 'Tu proyecto, a una conversaci\u00f3n.', '\u901a\u8fc7\u5bf9\u8bdd\u4e86\u89e3\u4f60\u7684\u9879\u76ee\u3002'],
    'A session needs you': ['Uma sess\u00e3o precisa de voc\u00ea', 'Una sesi\u00f3n te necesita', '\u4e00\u4e2a\u4f1a\u8bdd\u9700\u8981\u4f60'],
    'Project context': ['Contexto do projeto', 'Contexto del proyecto', '\u9879\u76ee\u4e0a\u4e0b\u6587'],
    'Context consulted': ['Contexto consultado', 'Contexto consultado', '\u5df2\u67e5\u9605\u4e0a\u4e0b\u6587'],
    'Project files read': ['Arquivos do projeto lidos', 'Archivos del proyecto le\u00eddos', '\u5df2\u8bfb\u53d6\u9879\u76ee\u6587\u4ef6'],
    'Recent changes checked': ['Altera\u00e7\u00f5es recentes consultadas', 'Cambios recientes consultados', '\u5df2\u67e5\u770b\u6700\u8fd1\u66f4\u6539'],
    'Local prototype only': ['Somente nesta pr\u00e9via', 'Solo en esta vista previa', '\u4ec5\u5728\u6b64\u9884\u89c8\u4e2d'],
    'Ready to continue': ['Pronto para continuar', 'Listo para continuar', '\u53ef\u4ee5\u7ee7\u7eed'],
    'In progress': ['Em andamento', 'En curso', '\u8fdb\u884c\u4e2d'],
    'For review': ['Para revisar', 'Para revisar', '\u5f85\u5ba1\u67e5'],
    'All tasks': ['Todas as tarefas', 'Todas las tareas', '\u6240\u6709\u4efb\u52a1'],
    'The next useful action, before maintenance.': ['A pr\u00f3xima a\u00e7\u00e3o, antes da manuten\u00e7\u00e3o.', 'La pr\u00f3xima acci\u00f3n, antes del mantenimiento.', '\u4f18\u5148\u663e\u793a\u4e0b\u4e00\u4e2a\u6709\u7528\u7684\u64cd\u4f5c\u3002'],
    'Refine the chat composer': ['Refinar o compositor do chat', 'Refinar el compositor del chat', '\u6539\u8fdb\u804a\u5929\u8f93\u5165\u533a'],
    'Make model selection easier': ['Facilitar a escolha de modelos', 'Facilitar la selecci\u00f3n de modelos', '\u7b80\u5316\u6a21\u578b\u9009\u62e9'],
    'Review the connection checks': ['Revisar os checks das conex\u00f5es', 'Revisar las comprobaciones de conexiones', '\u5ba1\u67e5\u8fde\u63a5\u68c0\u67e5'],
    'Keep the draft on one surface': ['Manter o rascunho em uma superf\u00edcie', 'Mantener el borrador en una superficie', '\u5728\u540c\u4e00\u533a\u57df\u4fdd\u7559\u8349\u7a3f'],
    'Keep the transcript readable': ['Manter a conversa leg\u00edvel', 'Mantener legible la conversaci\u00f3n', '\u4fdd\u6301\u5bf9\u8bdd\u6613\u8bfb'],
    'Preserve keyboard navigation': ['Preservar a navega\u00e7\u00e3o por teclado', 'Conservar la navegaci\u00f3n con teclado', '\u4fdd\u7559\u952e\u76d8\u5bfc\u822a'],
    'Nothing here matches your search.': ['Nada aqui corresponde \u00e0 busca.', 'No hay coincidencias con la b\u00fasqueda.', '\u6ca1\u6709\u5339\u914d\u7684\u7ed3\u679c\u3002'],
    'Review changes before merging.': ['Revise as altera\u00e7\u00f5es antes do merge.', 'Revisa los cambios antes de fusionar.', '\u5408\u5e76\u524d\u8bf7\u5ba1\u67e5\u66f4\u6539\u3002'],
    'Checks and review, without hunting through details.': ['Checks e revis\u00e3o, sem procurar entre detalhes.', 'Comprobaciones y revisi\u00f3n, sin buscar entre detalles.', '\u6e05\u6670\u5c55\u793a\u68c0\u67e5\u4e0e\u5ba1\u67e5\u3002'],
    'Keep Zeca close, without crowding the conversation.': ['Zeca por perto, sem apertar a conversa.', 'Zeca cerca, sin apretar la conversaci\u00f3n.', '\u8ba9 Zeca \u4fdd\u6301\u5728\u8eab\u8fb9\uff0c\u540c\u65f6\u7559\u8db3\u5bf9\u8bdd\u7a7a\u95f4\u3002'],
    'Essential controls first.': ['O essencial vem primeiro.', 'Lo esencial primero.', '\u4f18\u5148\u663e\u793a\u5fc5\u8981\u63a7\u4ef6\u3002'],
    'Use the existing animations.': ['Usar as anima\u00e7\u00f5es existentes.', 'Usar las animaciones existentes.', '\u4f7f\u7528\u73b0\u6709\u52a8\u753b\u3002'],
    'Every command asks first.': ['Cada comando pede permiss\u00e3o.', 'Cada comando pide permiso.', '\u6bcf\u4e2a\u547d\u4ee4\u90fd\u4f1a\u5148\u8be2\u95ee\u3002'],
    'The saved conversation stays on this computer.': ['A conversa salva fica neste computador.', 'La conversaci\u00f3n guardada permanece en este equipo.', '\u5df2\u4fdd\u5b58\u7684\u5bf9\u8bdd\u4ec5\u5b58\u50a8\u5728\u6b64\u7535\u8111\u4e0a\u3002'],
    'The reply stays in a clean reading column.': ['A resposta fica em uma coluna de leitura limpa.', 'La respuesta queda en una columna de lectura limpia.', '\u56de\u7b54\u663e\u793a\u5728\u7b80\u6d01\u7684\u9605\u8bfb\u680f\u4e2d\u3002'],
    'The model, files and send action share the composer.': ['Modelo, arquivos e envio ficam no compositor.', 'Modelo, archivos y env\u00edo comparten el compositor.', '\u6a21\u578b\u3001\u6587\u4ef6\u548c\u53d1\u9001\u6309\u94ae\u5171\u7528\u8f93\u5165\u533a\u3002'],
    'Supporting activity opens only when you need it.': ['A atividade de apoio abre quando voc\u00ea precisar.', 'La actividad de apoyo se abre cuando la necesitas.', '\u9700\u8981\u65f6\u624d\u5c55\u5f00\u8f85\u52a9\u6d3b\u52a8\u3002'],
    'I reviewed the recent changes.': ['Revisei as altera\u00e7\u00f5es recentes.', 'Revis\u00e9 los cambios recientes.', '\u6211\u5df2\u5ba1\u67e5\u6700\u8fd1\u7684\u66f4\u6539\u3002'],
    'This question stays in the prototype.': ['Essa pergunta fica somente no prot\u00f3tipo.', 'Esta pregunta permanece en el prototipo.', '\u6b64\u95ee\u9898\u4ec5\u4fdd\u7559\u5728\u539f\u578b\u4e2d\u3002'],
    'Appearance': ['Apar\u00eancia', 'Apariencia', '\u5916\u89c2'],
    'Advanced': ['Avan\u00e7ado', 'Avanzado', '\u9ad8\u7ea7'],
    'Saved': ['Salvo', 'Guardado', '\u5df2\u4fdd\u5b58'],
    'Start over': ['Come\u00e7ar de novo', 'Empezar de nuevo', '\u91cd\u65b0\u5f00\u59cb'],
  };
  const extraCopy = /* EXTRA_COPY */;
  let language = 'pt-BR';
  const L = text => { const index=['pt-BR','es','zh'].indexOf(language); return index<0?text:(copy[text]?.[index] ?? extraCopy[text]?.[index] ?? A.t(text)); };
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
    const b = button(null, 'icon-action', action, label); b.append(A.icon(name, 15)); return b;
  };
  const prefs = { motion: !matchMedia('(prefers-reduced-motion: reduce)').matches, spacing: 12 };
  const springs = { snappy: { k:600, c:30, m:.6 }, gentle: { k:220, c:24, m:1 }, bouncy: { k:400, c:25, m:.8 } };
  function frames(preset, dy = 6, scale = .98, duration = 300) {
    const { k,c,m } = springs[preset], a=c/(2*m), w=Math.sqrt(k/m-a*a);
    return Array.from({ length:31 }, (_,i) => {
      const time=i/30*duration/1000;
      const p=i===30?1:1-Math.exp(-a*time)*(Math.cos(w*time)+a/w*Math.sin(w*time));
      return { opacity:Math.max(0,Math.min(1,p)), transform:`translateY(${dy*(1-p)}px) scale(${scale+(1-scale)*p})` };
    });
  }
  function enter(node, preset='snappy', dy=6, scale=.98) {
    if (!prefs.motion || !node.animate) return;
    node.animate(frames(preset,dy,scale), { duration:300, fill:'none' });
  }
  const windowEl = $('.product-window'), chatEl = $('.chat-view'), log = $('.transcript');
  function resizeFrom() { positionOpenMenus(); }
  const input = $('textarea'), popover = $('.model-popover'), history = $('.history-popover');
  const state = { area:'chat', mode:'quick', scenario:'ready', activity:'week', category:'general', conversation:'current', example:'empty', engine:'Claude Code', model:'Default model', effort:'Default effort', files:[], messages:[], busy:false, voice:false, notice:false };
  let restoring = true;
  const AREAS=['summary','flock','chat','tasks','activity','connections','settings','memory','review'];
  const original = window.openai?.widgetState;
  if (original?.modelContent?.area && AREAS.includes(original.modelContent.area)) state.area=original.modelContent.area;
  function remember() {
    if (restoring) return;
    window.openai?.setWidgetState?.({ modelContent:{ area:state.area, mode:state.mode, activity:state.activity, category:state.category, example:state.example, engine:state.engine, model:state.model }, privateContent:snapshot() }).catch(() => {});
  }
  const scene = new A.Scene(A.FOCUS_SCENE);
  scene.canvas.setAttribute('aria-hidden','true'); $('.portrait').append(scene.canvas);
  function paintBird(clip) {
    scene.update([],null,{ clip:clip ?? (state.busy?'think':state.voice?'listen':'idle'),agent:state.engine==='Codex'?'codex':'claude' });
    scene.setActive(settings.zeca && state.area==='chat' && state.mode!=='mini' && !windowEl.classList.contains('very-short') && !(windowEl.classList.contains('workspace')&&windowEl.classList.contains('tight')) && !document.hidden);
    if (!prefs.motion) window.setTimeout(() => scene.setActive(false),0);
  }
  $('.portrait').addEventListener('click',() => { if (prefs.motion && !state.busy && !state.voice) scene.react('hello'); });
  $('.portrait').addEventListener('pointerenter',() => { if (prefs.motion) scene.lookAt('front'); });
  $('.portrait').addEventListener('pointerleave',() => { if (prefs.motion) scene.lookAt(null); });
  document.addEventListener('visibilitychange',() => paintBird());
  for (const node of $$('[data-word]')) node.textContent=L(node.dataset.word);
  $('.product-head-actions').append(iconButton('sound','Sounds',() => changedSetting('sounds',!settings.sounds)),iconButton('search','Search',() => openPalette()),iconButton('fullscreen','Full screen',() => setMode(state.mode==='full'?'room':'full')));
  const modelTrigger = button(null,'model-trigger',() => toggleModels(), 'Model and effort');
  modelTrigger.setAttribute('aria-expanded','false'); modelTrigger.setAttribute('aria-controls','vults-proposal-model-menu');
  popover.id='vults-proposal-model-menu';
  const attach = iconButton('clip','Attach a file',() => { if (!state.files.length) state.files.push('design-notes.md'); paintFiles(); paintBird('swallow'); window.setTimeout(() => paintBird(),1400); });
  const mic = iconButton('mic','Voice',() => { state.voice=!state.voice; input.hidden=state.voice; $('.voice-preview').hidden=!state.voice; mic.setAttribute('aria-pressed',String(state.voice)); if(!state.voice&&!input.value.trim()){input.value=L('Review the layout');input.dispatchEvent(new Event('input'));} paintBird();remember(); });
  const send = button(null,'send-action',() => submit(), 'Send (Enter)'); send.append(A.icon('chevron',15));
  $('.composer-left').append(attach,modelTrigger); $('.composer-right').append(mic,send);
  const historyTrigger = iconButton('list','Conversations',() => toggleHistory());
  history.id='vults-proposal-history';historyTrigger.setAttribute('aria-expanded','false');historyTrigger.setAttribute('aria-controls',history.id);
  $('.chat-actions').append(historyTrigger, iconButton('plus','New',() => newConversation()));
  $('.context-folder').append(A.icon('folder',12),button('Vults','folder-trigger',() => openFolder()),A.icon('chevronDown',10));
  $('.context-access').append(A.icon('person',12),button('Asks first','access-trigger',() => {state.category='chat';setArea('settings','room');},'Open chat access settings in Room'));
  input.placeholder=L('Ask Zeca…'); input.setAttribute('aria-label',L('Ask Zeca…'));
  input.addEventListener('input',() => { input.style.height='auto'; input.style.height=Math.min(90,input.scrollHeight)+'px'; send.disabled=!state.busy&&!input.value.trim()&&!state.files.length; remember(); });
  input.addEventListener('keydown',event => {
    if (event.isComposing || event.keyCode===229) return;
    if (event.key==='Enter'&&!event.shiftKey) { event.preventDefault(); submit(); }
  });
  const sampleQuestion = () => L('What changed recently?');
  function details(title, rows, cls='run-details') {
    const d=el('details',cls), s=el('summary',null,null,A.icon('chevron',12),el('span',null,title));
    const body=el('div','run-body');
    rows.forEach(text => body.append(el('div',null,null,A.icon('check',12),el('span',null,text))));
    d.append(s,body); d.addEventListener('toggle',() => { if (d.open) enter(body,'gentle',-3,1); });
    return d;
  }
  function paintConversation() {
    const readingTop=log.scrollTop;
    chatEl.classList.toggle('populated',state.messages.length>0||state.busy);
    const title=$('.intro-copy h2'), caption=$('.intro-caption');
    title.textContent=state.messages.length||state.busy?'Zeca':L('Ask Zeca');
    caption.textContent=state.messages.length||state.busy?L('Project context'):L('Your project, one conversation away.');
    log.replaceChildren();
    for (const m of state.messages) {
      if (m.who==='you') log.append(el('div','message you',m.text));
      else {
        log.append(details('Context consulted',['Project files read','Recent changes checked']));
        const reply=el('div','message reply',null,el('div','message-signature',null,el('span','agent-dot'),el('span',null,'Zeca')),el('p',null,m.text));
        if (m.sample) {
          const list=el('ul');
          ['The reply stays in a clean reading column.','The model, files and send action share the composer.','Supporting activity opens only when you need it.'].forEach(text => list.append(el('li',null,text)));
          reply.append(list);
        }
        reply.append(el('div','message-tools',null,button('Copy','',() => { toast('Copied in this prototype'); })));
        log.append(reply);
      }
    }
    if (state.busy) {
      const thinking=el('div','message-signature',null,el('span','live-dot'),el('span',null,'Thinking…'));
      log.append(thinking);
    }
    send.disabled=!state.busy&&!input.value.trim()&&!state.files.length;
    send.setAttribute('aria-label',L(state.busy?'Stop':'Send (Enter)'));
    send.replaceChildren(A.icon(state.busy?'stop':'chevron',15));
    log.scrollTop=readingTop;
    paintBird(); paintModelTrigger(); paintSuggestions();
  }
  function paintFiles() {
    const host=$('.attachment-strip'); host.replaceChildren();
    state.files.forEach(name => {
      const item=el('div','attachment',null,A.icon('file',18),el('div',null,null,button(name,'attachment-open',() => openEditor(name,el('pre','diff-preview','# '+name+'\n\n'+L('Keep the transcript readable')))),el('small',null,'Markdown')));
      item.append(iconButton('close','Remove',() => { state.files=[]; paintFiles(); }));
      host.append(item); enter(item,'bouncy',0,.92);
    });
    send.disabled=!state.busy&&!input.value.trim()&&!state.files.length;
  }
  function paintModelTrigger() {
    modelTrigger.replaceChildren(el('span','agent-dot'+(state.engine==='Codex'?' codex':'')),el('span','model-text',state.model==='Default model'?state.engine:state.model),A.icon('chevronDown',11));
  }
  function modelChoices() {
    const list=$('.model-options'); list.replaceChildren();
    $('.engine-select').value=state.engine;
    $('.engine-select').disabled=state.messages.length>0||state.busy;
    $('.model-popover-foot').hidden=state.messages.length>0;
    if (state.messages.length) {
      list.append(el('p','pinned-model',"A conversation keeps its model; New starts another."),button('New','quiet-control',() => { closeModels(); newConversation(); toggleModels(true); }));
      return;
    }
    const models=state.engine==='Codex'?['Default model','GPT-6.1-Sol','GPT-6-Astra','GPT-6-Sol']:['Default model','Sonnet','Opus','Haiku'];
    models.forEach(name => {
      const choice=button(null,'model-option',() => { state.model=name; paintModelTrigger(); closeModels(); remember(); });
      choice.setAttribute('aria-pressed',String(state.model===name));
      choice.append(el('span','check-space',null,...(state.model===name?[A.icon('check',13)]:[])),el('span',null,name));
      if (name==='Default model') choice.append(el('small',null,state.engine));
      list.append(choice);
    });
  }
  function toggleModels(force) {
    const open=force ?? modelTrigger.getAttribute('aria-expanded')!=='true';
    closeMenu(history,historyTrigger,false,false);
    if (!open) { closeModels(); return; }
    modelChoices();
    openMenu(popover,modelTrigger);
    $('.model-close').focus({preventScroll:true});
  }
  function closeModels(returnFocus=true,animate=true) { closeMenu(popover,modelTrigger,returnFocus,animate); }
  $('.model-close').append(A.icon('close',13)); $('.model-close').setAttribute('aria-label',L('Close'));
  $('.model-close').addEventListener('click',() => closeModels());
  $('.engine-select').addEventListener('change',event => {
    if(state.messages.length||state.busy){event.target.value=state.engine;return;}
    const next=event.target.value;
    state.engine=next; state.model='Default model'; modelChoices(); paintModelTrigger(); paintBird();
    positionOpenMenus();
    remember();
  });
  ['Default effort','Low','Medium','High'].forEach(word => $('.effort-select').append(el('option',null,word)));
  $('.effort-select').addEventListener('change',event => { state.effort=event.target.value; remember(); });
  popover.addEventListener('keydown',event => {
    if (event.isComposing || event.keyCode===229 || event.target.tagName==='SELECT') return;
    if (!['ArrowDown','ArrowUp','Home','End'].includes(event.key)) return;
    const choices=[...popover.querySelectorAll('.model-option')]; if (!choices.length) return;
    event.preventDefault(); const i=choices.indexOf(document.activeElement);
    choices[event.key==='Home'?0:event.key==='End'?choices.length-1:(i+(event.key==='ArrowDown'?1:-1)+choices.length)%choices.length].focus();
  });
  function toggleHistory() {
    closeModels(false,false);
    if (history.getAttribute('data-open')==='true') {closeMenu(history,historyTrigger,true);return;}
    history.replaceChildren(el('div','model-popover-head','Conversations',iconButton('close','Close',() => closeMenu(history,historyTrigger,true))));
    saveConversation();conversations.forEach(c=>history.append(button(c.title,'',()=>{closeMenu(history,historyTrigger,false,false);openConversation(c.id);})));
    history.append(button('New','quiet-control',()=>{closeMenu(history,historyTrigger,false,false);newConversation();}));
    openMenu(history,historyTrigger);
  }
  document.addEventListener('pointerdown',event => {
    if (!popover.hidden&&!popover.contains(event.target)&&!modelTrigger.contains(event.target)) closeModels(false);
    if (!history.hidden&&!history.contains(event.target)&&!historyTrigger.contains(event.target)) closeMenu(history,historyTrigger,false);
  });
  function showExample(example) {
    const before=windowEl.getBoundingClientRect().height;
    clearTimeout(replyTimer);
    state.voice=false; input.hidden=false; $('.voice-preview').hidden=true; mic.setAttribute('aria-pressed','false');
    state.example=example; state.busy=example==='thinking'; state.notice=example==='notification';
    state.messages=['conversation','thinking','notification'].includes(example)?[{who:'you',text:sampleQuestion()}]:[];
    if (example==='conversation'||example==='notification') state.messages.push({who:'zeca',text:'I reviewed the recent changes.',sample:true});
    closeModels(false,false);closeMenu(history,historyTrigger,false,false);
    paintConversation(); paintNotice();
    resizeFrom(before);
    $$('[data-example]').forEach(b => b.setAttribute('aria-pressed',String(b.dataset.example===example)));
    if (example==='models') toggleModels(true);
    remember();
  }
  let replyTimer;
  function submit() {
    if (state.busy) { clearTimeout(replyTimer); state.busy=false; paintConversation(); return; }
    const text=input.value.trim(); if (!text&&!state.files.length) return;
    const before=windowEl.getBoundingClientRect().height;
    closeModels(false); state.messages.push({who:'you',text:text||'design-notes.md'}); state.busy=true;
    input.value=''; input.style.height='auto'; state.files=[]; paintFiles(); paintConversation();resizeFrom(before);log.scrollTop=log.scrollHeight;
    replyTimer=window.setTimeout(() => { const before=windowEl.getBoundingClientRect().height, top=log.scrollTop, follow=log.scrollHeight-log.scrollTop-log.clientHeight<40 && Date.now()-lastManualScroll>=150; state.busy=false; state.messages.push({who:'zeca',text:'This question stays in the prototype.',sample:false}); paintConversation();resizeFrom(before);enter(log.lastElementChild,'gentle');log.scrollTop=follow?log.scrollHeight:top;remember(); },1400);
    remember();
  }
  let noticeAnimation;
  function paintNotice() {
    const slot=$('.notice-slot'),shown=slot.getAttribute('data-open')==='true';
    if(shown===state.notice)return;
    const before=slot.hidden?0:slot.getBoundingClientRect().height;noticeAnimation?.cancel();slot.setAttribute('data-open',String(state.notice));slot.inert=!state.notice;
    if(!prefs.motion||restoring){slot.hidden=!state.notice;return;}
    slot.hidden=false;slot.inert=!state.notice;const height=state.notice?Math.min(80,slot.scrollHeight):0;
    noticeAnimation=slot.animate([{height:before+'px',opacity:shown?1:0},{height:height+'px',opacity:state.notice?1:0}],{duration:220,easing:'cubic-bezier(0.16,1,0.3,1)'});
    noticeAnimation.finished.then(()=>{slot.hidden=!state.notice;slot.inert=!state.notice;}).catch(()=>{});
  }
  $('.notice-close').append(A.icon('close',12)); $('.notice-close').setAttribute('aria-label',L('Dismiss'));
  $('.notice-close').addEventListener('click',() => { const before=windowEl.getBoundingClientRect().height;state.notice=false;paintNotice();resizeFrom(before); });
  let toastTimer;
  function toast(text) { const host=$('.feedback');host.textContent=L(text);host.hidden=false;clearTimeout(toastTimer);toastTimer=setTimeout(() => host.hidden=true,1800); }
  function confirm(title,about,action) {
    const overlay=$('.dialog-overlay'), dialog=$('.confirmation'), previous=document.activeElement;
    dialog.querySelector('h2').textContent=L(title); dialog.querySelector('p').textContent=L(about);
    const cancel=button('Cancel','quiet-control',() => close());
    const yes=button(title,'danger',() => { action();close(); });
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
      if (!popover.hidden) { event.preventDefault();closeModels(); }
      else if (!history.hidden) { event.preventDefault();closeMenu(history,historyTrigger,true); }
      else if (state.voice) { state.voice=false;input.hidden=false;$('.voice-preview').hidden=true;paintBird(); }
    }
  });
  function pageHeading(title,about,action) { return el('div','page-title',null,el('div',null,null,el('h2',null,title),el('p',null,about)),action); }
  function search(label,action) {
    const field=el('input');field.type='search';field.placeholder=L(label);field.setAttribute('aria-label',L(label));field.addEventListener('input',() => action(field.value));
    return el('label','search-field',null,A.icon('search',14),field);
  }
  function segment(items,onChange) {
    const host=el('div','segment');
    items.forEach((word,i) => { const b=button(word,'',() => { for (const node of host.children) node.setAttribute('aria-pressed',String(node===b));enter(b,'snappy',0,.96);onChange(word); });b.setAttribute('aria-pressed',String(i===0));host.append(b); });
    return host;
  }
  function setting(title,about,control) { return el('div','setting-row',null,el('div','setting-copy',null,el('strong',null,title),el('small',null,about)),control); }
  /* WORKSPACE_LOGIC */
  $$('[data-area]').forEach(b => b.addEventListener('click',() => setArea(b.dataset.area,b.dataset.mode ?? 'room')));
  $$('[data-size]').forEach(b => b.addEventListener('click',() => setMode(b.dataset.size)));
  $('.mini-strip').addEventListener('click',() => setMode('quick'));
  $('.scenario-select').addEventListener('change',() => setScenario($('.scenario-select').value));
  $('.viewport-select').addEventListener('change',() => {const stage=$('.review-stage');stage.classList.toggle('narrow',$('.viewport-select').value==='narrow');stage.classList.toggle('short',$('.viewport-select').value==='short');layoutSurface(true);resizeCompactFlock();paintBird();});
  $('.return-compact').addEventListener('click',() => setMode('quick'));
  $$('[data-example]').forEach(b => b.addEventListener('click',() => showExample(b.dataset.example)));
  window.addEventListener('openai:set_globals',event => {
    const saved=event.detail?.globals?.widgetState?.modelContent;
    if (saved?.area && AREAS.includes(saved.area)) {
      restoring=true;restorePrototype(event.detail.globals.widgetState);setArea(state.area,state.mode);restoring=false;
    }
  });
  restorePrototype(original);paintFiles();paintConversation();setArea(state.area,state.mode);
  restoring=false;
  if (globalThis.Tweak) {
    const tweak=new Tweak({container:root,onChange:() => {root.classList.toggle('motion-off',!prefs.motion);$('.chat-view').style.padding=prefs.spacing+'px';paintBird();}});
    tweak.addToggle(prefs,'motion',{label:'Animations'});
    tweak.addSlider(prefs,'spacing',{label:'Spacing',min:8,max:16,step:2,unit:'px'});
  }
})();
