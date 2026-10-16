  /* Lab · Settings, behind the gear. The same groups and fields as the app's Settings window. */
  const CATEGORIES=[['general','General','Experience'],['chat','Chat & voice','Experience'],['flock','Flock & Zeca','Experience'],['agents','Agents','Work'],['approvals','Approvals','Work'],['projects','Projects','Work'],['activity','Activity','System'],['connectors','Connectors','System'],['phone','Phone','System'],['about','About','System']];
  const PROVIDERS=['Anthropic','OpenAI','Google','OpenRouter','Groq','DeepSeek','Mistral','xAI','Ollama','LM Studio'];
  const CAPABILITIES=['Permissions','Questions','Diffs','Stopping a turn','Sandbox','Started from Vults'];
  const AGENT_CAN={claude:['yes','yes','yes','yes','yes','yes'],codex:['yes','yes','yes','yes','yes','yes'],antigravity:['own','own','no','no','no','no'],opencode:['yes','yes','yes','yes','no','yes'],qwen:['yes','terminal','yes','yes','no','yes'],copilot:['yes','terminal','no','yes','no','yes'],cursor:['own','own','no','no','no','no'],droid:['own','own','no','no','no','no'],kimi:['own','own','no','yes','no','yes']};
  function changedSetting(name,value) {
    settings[name]=value;toast('Saved');
    if(name==='animations'){prefs.motion=value&&!matchMedia('(prefers-reduced-motion: reduce)').matches;root.classList.toggle('motion-off',!prefs.motion);if(!prefs.motion)settleMotion();for(const s of activeScenes)s.setActive(prefs.motion&&!document.hidden);}
    if(name==='zeca')A.setZeca(value);
    if(name==='species')A.setZecaSpecies(value);
    if(name==='outfit')A.setZecaLook(['none','auto'].includes(value)?null:value);
    if(['species','outfit','zeca'].includes(name)){paintBird();for(const scene of activeScenes)scene.refresh();paintNav();}
    paintChrome();remember();
  }
  function makeSwitch(name,label,onChange) {
    const b=button(null,'toggle',()=>{const on=!settings[name];b.setAttribute('aria-checked',String(on));changedSetting(name,on);onChange?.(on);enter(b,'snappy',0,.98);},label);b.setAttribute('role','switch');b.setAttribute('aria-checked',String(!!settings[name]));b.append(el('span'));return b;
  }
  function optionRow(name,title,about,choices){return setting(title,about,choose(title,choices,settings[name],value=>changedSetting(name,typeof settings[name]==='number'?Number(value):value)));}
  function switchRow(name,title,about,onChange){return setting(title,about,makeSwitch(name,title,onChange));}
  function settingsGroup(title,...rows){return el('section','settings-group',null,el('h3',null,title),...rows.filter(Boolean));}
  function more(title,rows){return details(title,rows,'run-details setting-more');}
  function reviewDiff(title,lines,onApply,applyLabel='Write the file'){
    const pre=el('pre','diff-preview');for(const line of lines)pre.append(el('span',line.startsWith('+')?'diff-add':line.startsWith('-')?'diff-remove':null,null,document.createTextNode(line+'\n')));
    openEditor(title,el('div',null,null,el('p','pane-note','Review the change. A dated backup is kept before anything is written; other hooks stay in place.'),pre),()=>{onApply();return true;},applyLabel);
  }
  function renderSettings(host) {
    host.append(pageHeading('Settings',null));
    if(stageState(host,'No settings loaded.'))return;
    const categories=el('nav','settings-categories'),content=el('div','settings-content');categories.setAttribute('aria-label',L('Settings'));let last;
    CATEGORIES.forEach(([id,label,group])=>{if(last!==group){categories.append(el('small',null,group));last=group;}const b=key(button(label,'',()=>{state.category=id;paint();remember();}),id);categories.append(b);});
    const grid=el('div','settings-grid',null,categories,content);host.append(grid);flexible(grid,categories,'settings',{def:190,min:150,max:300,label:'Sections',rail:()=>{const r=el('div','rail-marks');CATEGORIES.forEach(([id,label])=>{const b=button(null,'rail-dot',()=>{state.category=id;paint();remember();},label);b.setAttribute('data-tooltip',L(label));b.setAttribute('aria-pressed',String(state.category===id));b.append(el('span','rail-letter',null,document.createTextNode(L(label)[0])));r.append(b);});return r;}});paint();
    function paint(){
      stopScenes();content.replaceChildren();for(const b of categories.querySelectorAll('button'))b.setAttribute('aria-pressed',String(b.dataset.key===state.category));
      const title=CATEGORIES.find(([id])=>id===state.category)?.[1]??'General';content.append(el('h3','settings-title',title));
      ({general,chat,flock,agents,approvals,projects,activity,connectors,phone,about})[state.category]?.();
      enter(content,'gentle',4,1);
    }
    function general(){
      const languageSelect=choose('Language',[{value:'system',label:'System'},{value:'en',label:'English'},{value:'pt-BR',label:'Portugu\u00eas (Brasil)'},{value:'es',label:'Espa\u00f1ol'},{value:'zh',label:'\u7b80\u4f53\u4e2d\u6587'}],language,value=>setLanguage(value==='system'?'pt-BR':value));
      const volume=el('input');volume.type='range';volume.min='0';volume.max='100';volume.value=settings.volume;volume.setAttribute('aria-label',L('Volume'));const number=el('span','counter',null,document.createTextNode(settings.volume+'%'));volume.addEventListener('input',()=>{number.textContent=volume.value+'%';settings.volume=Number(volume.value);remember();});
      content.append(
        settingsGroup('Island',optionRow('presence','Presence','Island: the pill at the top. Panel: a notification instead. Quiet: only cards. Paused: cards go back to the terminal.',['Island','Panel','Quiet','Paused']),optionRow('fold','Fold the island','Seconds after the pointer leaves.',[5,10,15,30,60,120].map(n=>({value:String(n),label:n+' s'}))),switchRow('hover','Open on hover','Open the island when the pointer rests on it.'),optionRow('screen','Screen','Which monitor holds the island.',['Follow the pointer','DP-1 · 2560\u00d71440','eDP-1 · 1920\u00d71200']),optionRow('widget','Corner widget','A small flock in a corner of the desktop.',['Off','Top left','Top right','Bottom left','Bottom right'])),
        settingsGroup('Attention',switchRow('notifications','Notifications','Desktop notifications in Panel: finished, failed, needs you, gone quiet.'),switchRow('sounds','Sounds','Short cues for attention and finished work.'),setting('Volume',null,el('div','control-group',null,number,volume)),optionRow('dnd','Do not disturb','Cards still wait for you; nothing makes a sound or pulses.',['Off','30 min','1 h','4 h'])),
        settingsGroup('Language & desktop',setting('Theme','Colors from the sp-night palette.',(()=>{const pick=choose('Theme',THEMES,settings.theme,value=>{applyTheme(value);toast('Saved');remember();});pick.classList.add('settings-theme');return pick;})()),setting('Language','The language used across Vults.',languageSelect),switchRow('media','Now playing','Show what is playing and its controls (MPRIS).'),switchRow('usage','Plan usage','Claude Code through its status line; Codex checked every 5 minutes.'),switchRow('autostart','Start with the desktop','Open Vults when you log in.')),
        more('Keyboard shortcuts',['Ctrl+Alt+Y / N · Allow or deny the front card','Ctrl+Alt+V · Hold to talk','Ctrl+Alt+J / K · Next or previous session','Ctrl+Alt+Space · Keyboard mode','Ctrl+Alt+1 / 2 / 3 · Island, Compact, Lab','Ctrl+K · Search','Alt+1–6 · Lab tabs','Esc · Close, or leave full screen']));
    }
    function chat(){
      const access=choose("Who answers Zeca's cards",['Asks first','Your policies','Full access'],settings.access,value=>{
        if(value==='Full access'){access.value=settings.access;confirm('Full access','Zeca runs his tools without asking. Destructive commands still ask, a policy\'s deny still holds, and content from outside brings the cards back.',()=>{changedSetting('access',value);access.value=value;},'danger','Allow full access');}
        else changedSetting('access',value);
      });
      const provider=choose('Provider',PROVIDERS,settings.apiProvider??'Anthropic',value=>{settings.apiProvider=value;paint();});
      const id=(settings.apiProvider??'Anthropic').toLowerCase().replace(' ','');
      const keyRow=settings.keys[id]?el('div','control-group',null,chip('done','Key saved'),button('Remove','',()=>confirm('Remove key','The key is deleted from the OS keyring.',()=>{settings.keys[id]=false;paint();},'default','Remove'))):button('Add key…','quiet-control',()=>{const k=field('API key','','password');openEditor('API key',el('div',null,null,k.element,el('p','pane-note','Stored in the OS keyring. Vults never writes it to a file.')),()=>{if(k.control.value.trim().length<8)throw new Error('Paste the whole key.');settings.keys[id]=true;paint();return true;});});
      content.append(
        settingsGroup('Engines',switchRow('native','Your Claude Code and Codex setup','The CLI reads your CLAUDE.md, skills, MCP servers and hooks. Applies to the next conversation.'),setting('API provider','For the Vults harness. Models load live from the provider.',provider),setting('API key',null,keyRow),setting("Who answers Zeca's cards",'Every answer is audited. Destructive commands always ask.',access)),
        settingsGroup('Memory',switchRow('memory',"Zeca's memory",'A local journal, kept 90 days with secrets removed.'),switchRow('summary','A summary of each day','Up to 8 requests a day on your subscription.'),switchRow('suggestions','Suggest what to remember','Proposals wait for you; up to 20 a day.'),switchRow('sleep','Let Zeca sleep on it','Merges notes overnight and leaves a debrief; up to 4 a day.'),switchRow('embeddings','Recall by meaning','Uses a local Ollama embedding model.'),setting('Review memories',null,button('Open Memory','quiet-control',()=>openMemory()))),
        settingsGroup('Voice',switchRow('voice','Talk to the chat','whisper.cpp on this computer; nothing is sent.'),optionRow('voiceModel','Voice model','Downloaded once and checked.',['Base','Small','Large v3 Turbo']),optionRow('voiceLanguage','Language you speak',null,['Automatic','Portugu\u00eas','English','Espa\u00f1ol']),setting('Dictation','Hold the same key in any app.',button('Dictation settings','quiet-control',()=>dxOpen('settings')))),
        settingsGroup('Web for Zeca',switchRow('web','Web for Zeca','Search and read pages. Page content never answers a card.'),optionRow('webEngine','Search engine','Brave and Tavily need a key; SearXNG needs its URL.',['Brave','Tavily','SearXNG'])));
    }
    function flock(){
      const preview=el('div','settings-bird');const bird=new A.Scene(A.FOCUS_SCENE);activeScenes.push(bird);preview.append(bird.canvas);bird.update([],null,{clip:'idle',agent:'codex'});bird.setActive(!document.hidden);still(bird);
      const looks=el('select');looks.setAttribute('aria-label',L("Zeca's look"));for(const group of A.LOOK_GROUPS){const g=el('optgroup');g.label=L(group.title);for(const look of group.looks){const o=el('option',null,look.label);o.value=look.value;g.append(o);}looks.append(g);}looks.value=settings.outfit;looks.addEventListener('change',()=>changedSetting('outfit',looks.value));
      content.append(preview,settingsGroup('Zeca',switchRow('zeca','Zeca','Off, the flock, cards, shortcuts and connectors keep working.'),optionRow('species',"Zeca's species",null,A.SPECIES.map(s=>({value:s.id,label:raw(s.name)}))),setting("Zeca's look",'Auto follows the calendar.',looks)),
        settingsGroup('Flock',optionRow('pool','The flock draws from',null,['Brazil','The Americas','The world']),switchRow('visitors','Rare visitors','Now and then a visitor crosses the sky.'),switchRow('animations','Animations','Your system\'s reduced motion always wins.')));
    }
    function agents(){
      content.append(el('p','pane-note','Each install shows a diff, keeps a dated backup and waits for your click.'));
      A.AGENTS.filter(a=>a.setup!=='none').forEach(a=>{
        const on=settings['agent-'+a.id]??['claude','codex','opencode'].includes(a.id),removeOnly=a.setup==='remove-only';
        const row=key(el('article','content-row agent-row',null,el('header',null,null,el('h3',null,raw(a.name)),chip(on?'done':'quiet',removeOnly?'Retired':on?'Hooks installed':'Not set up'))),'agent-'+a.id);
        const label=removeOnly?'Remove hooks…':on?'Update hooks…':'Install hooks…';
        row.append(el('div','inline-actions',null,button(label,'quiet-control',()=>reviewDiff(L(label),['  "hooks": {','+   "PreToolUse": [{ "command": "vults-hook --agent '+a.id+'" }],','    "Stop": [{ "command": "your-own-hook.sh" }]','  }'],()=>{settings['agent-'+a.id]=!removeOnly;paint();toast('Written. Backup kept.');})),...(on&&!removeOnly?[button('Remove hooks…','',()=>reviewDiff(L('Remove hooks…'),['-   "PreToolUse": [{ "command": "vults-hook --agent '+a.id+'" }],'],()=>{settings['agent-'+a.id]=false;paint();}))]:[])));
        content.append(row);
      });
      const table=el('table','record-table capability-table'),body=el('tbody');table.append(el('thead',null,null,el('tr',null,null,el('th',null,'Agent'),...CAPABILITIES.map(c=>el('th',null,c)))),body);
      A.AGENTS.filter(a=>AGENT_CAN[a.id]).forEach(a=>body.append(el('tr',null,null,el('td',null,raw(a.name)),...AGENT_CAN[a.id].map(v=>el('td',null,null,chip(v==='yes'?'done':v==='no'?'quiet':'needs',v==='yes'?'Yes':v==='no'?'No':v==='own'?'Its own prompt':'In the terminal'))))));
      content.append(el('h3','lab-panel-title','What each agent can do'),el('div','table-scroll',null,table),button('Check again','',()=>toast('Checked: nothing changed.')));
    }
    function approvals(){
      content.append(settingsGroup('Always rules',el('p','pane-note','Each rule matches one agent, tool, target and folder exactly. Destructive commands never get one.'),...settings.rules.map(rule=>el('article','content-row rule-row',null,el('header',null,null,el('code',null,null,document.createTextNode(rule.target)),button('Remove','',()=>confirm('Remove rule','Matching requests ask you again.',()=>{settings.rules.splice(settings.rules.indexOf(rule),1);paint();remember();},'default','Remove'))),el('p','task-meta',null,document.createTextNode([rule.agent,rule.tool,rule.cwd].join(' · '))))),...(settings.rules.length?[]:[el('p','pane-note','No rules yet. Use Always allow on a card.')])));
      const policies=settingsGroup('Policies',el('p','pane-note','A deny beats an allow. Allow applies to Claude Code only. Destructive actions always ask.'));
      settings.policies.forEach(policy=>policies.append(el('article','content-row',null,el('header',null,null,el('h3',null,policy.scope),chip('done','Accepted')),el('pre','diff-preview',null,document.createTextNode(policy.text)),el('div','inline-actions',null,button('Edit','quiet-control',()=>editPolicy(policy)),button('Revoke','',()=>confirm('Revoke policy','Cards from this folder ask you again.',()=>{settings.policies.splice(settings.policies.indexOf(policy),1);paint();},'default','Revoke'))))));
      policies.append(button('New policy…','quiet-control',()=>editPolicy(null)));content.append(policies);
    }
    function editPolicy(policy){
      const text=field('Policy',policy?.text??'ask class network\nallow tool Read','textarea'),scope=field('Folder',policy?.scope??'~/Projects/frontend');
      openEditor(policy?'Edit policy':'New policy',el('div',null,null,scope.element,text.element,el('p','pane-note','Next you see the change as a diff and accept it with a click.')),()=>{
        const before=(policy?.text??'').split('\n'),after=text.control.value.split('\n');
        window.setTimeout(()=>reviewDiff('Review the policy',[...before.filter(l=>l&&!after.includes(l)).map(l=>'- '+l),...after.filter(l=>l&&!before.includes(l)).map(l=>'+ '+l),...after.filter(l=>before.includes(l)).map(l=>'  '+l)],()=>{if(policy)policy.text=text.control.value;else settings.policies.push({id:'p-'+Date.now(),scope:raw(scope.control.value),text:raw(text.control.value)});paint();toast('Policy accepted');},'Accept'),10);return true;
      },'Review diff');
    }
    function projects(){
      ['Vults','frontend','docs','api','infra'].forEach(project=>{
        const bird=choose('Bird',[{value:'auto',label:'The flock chooses'},...A.SPECIES.slice(0,8).map(s=>({value:s.id,label:raw(s.name)}))],settings['bird-'+project]??'auto',value=>{settings['bird-'+project]=value;toast('Saved');});
        content.append(settingsGroup(project,switchRow('pin-'+project,'Pinned','Kept near the start of the wire.'),switchRow('mute-'+project,'Muted','Cards still wait; no sounds or pulses.'),switchRow('hide-'+project,'Hidden','Off the wire until you show it again.'),setting('Bird',null,bird),setting('Forget this project','Its preferences go; its history stays in Activity.',button('Forget','',()=>confirm('Forget project',phrase('{project}\'s preferences are removed.',{project}),()=>toast('Forgotten'),'default','Forget')))));
      });
    }
    function activity(){
      content.append(settingsGroup('Local records',switchRow('history','Keep a history','Turns, traces and the week stay on this computer.'),setting('Activity',null,button('Open Activity','quiet-control',()=>setArea('activity','room')))),settingsGroup('Maintenance',setting('Clear history','Removes turns, conversations, journal and traces. Memory, policies, phones and costs stay.',button('Clear history…','quiet-control',()=>confirm('Clear history','This removes turns, conversations, the journal and traces from this computer.',()=>clearExampleHistory(),'danger','Clear history')))));
    }
    function connectors(){
      content.append(settingsGroup('GitHub',switchRow('github','GitHub','Calls gh every 5 minutes, or every minute while checks run.'),setting('Status',null,settings.github?chip('done',phrase('Signed in as {user} · synced {n} min ago',{user:'you',n:github.synced})):chip('quiet','Off')),setting('Daily budget','Every request goes through the network gate and leaves a trace.',el('span','counter',null,document.createTextNode(github.requests+' / 3,000'))),setting('Open Connections',null,button('Open Connections','quiet-control',()=>setArea('connections','room')))));
    }
    function phone(){
      const transport=choose('Transport',['Relay','Direct'],settings.transport,value=>{settings.transport=value;paint();});
      content.append(settingsGroup('Answer cards from a phone',switchRow('phone','Answer cards from a phone','Deny needs nothing; Allow and Always need the phone\'s biometrics.'),setting('Transport','Relay works from anywhere; direct needs the same network.',transport),settings.transport==='Relay'?setting('Relay URL',null,el('code','mono',null,document.createTextNode('relay.vults.dev'))):setting('Listen on',null,el('code','mono',null,document.createTextNode('192.168.0.12:7443'))),setting('Pair a phone','The QR code is valid for 2 minutes.',button('Pair…','quiet-control',()=>openEditor('Pair a phone',el('div','pairing',null,el('div','qr-sample'),el('p','pane-note','Scan with the Vults app, or type this code:'),el('strong','counter',null,document.createTextNode('K7Q2 · 4M9X'))),()=>{settings.phone=true;paint();return true;},'Paired')))),
        settingsGroup('Paired phones',settings.phone?el('article','content-row',null,el('header',null,null,el('h3',null,raw('Pixel 9')),button('Revoke','',()=>confirm('Revoke phone','It can no longer answer cards.',()=>{settings.phone=false;paint();},'default','Revoke'))),el('p','task-meta','Paired today')):el('p','pane-note','No phone paired.')));
    }
    function about(){
      content.append(el('article','content-row',null,el('h3',null,raw('Vults 0.1.4')),el('p',null,'A flock watching your coding agents. MIT licensed; no telemetry.'),details('Local paths',[el('code','mono',null,document.createTextNode('~/.config/vults/settings.toml')),el('code','mono',null,document.createTextNode('~/.local/share/vults/vults.sqlite'))]),button('Review this prototype','quiet-control',()=>setArea('review','room'))));
    }
  }
