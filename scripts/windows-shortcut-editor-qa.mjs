// Opt-in regression test in the real development WebView2. Fake persistence never changes settings.
const targets = () => fetch('http://127.0.0.1:9223/json/list').then(r => r.json());
async function connect(path) {
  const target = (await targets()).find(target => new URL(target.url).pathname === path);
  if (!target) throw new Error(`Missing development webview: ${path}`);
  const socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; });
  let id = 0;
  const pending = new Map();
  socket.onmessage = event => {
    const value = JSON.parse(event.data), call = pending.get(value.id);
    if (!call) return;
    clearTimeout(call.timer); pending.delete(value.id);
    value.error ? call.reject(value.error) : call.resolve(value.result);
  };
  return {
    close: () => socket.close(),
    evaluate: async expression => {
      const result = await new Promise((resolve, reject) => {
        const next = ++id, timer = setTimeout(() => reject(new Error('Editor QA timed out')), 15000);
        pending.set(next, { resolve, reject, timer });
        socket.send(JSON.stringify({ id: next, method: 'Runtime.evaluate', params: { expression, awaitPromise: true, returnByValue: true } }));
      });
      if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
      return result.result.value;
    },
  };
}
const island = await connect('/');
await island.evaluate(`window.__TAURI_INTERNALS__.invoke('open_nest_window',{tab:'flock',full:false})`);
island.close();
let settings;
for (let i = 0; i < 50 && !settings; i++) {
  if ((await targets()).some(target => new URL(target.url).pathname === '/settings.html')) settings = await connect('/settings.html');
  else await new Promise(resolve => setTimeout(resolve, 100));
}
if (!settings) throw new Error('Settings did not open');
try {
  console.log(await settings.evaluate(`(async () => {
    const { createWindowsShortcuts } = await import('/src/surfaces/settings/windows-shortcuts.ts');
    const assert = (condition, message) => { if (!condition) throw new Error(message); };
    const defaults = {open:'Ctrl+Alt+Space',next:'Ctrl+Alt+Right',previous:'Ctrl+Alt+Left',mini:'Ctrl+Alt+F1',quick:'Ctrl+Alt+F2',room:'Ctrl+Alt+F3',allow:'Ctrl+Alt+F8',deny:'Ctrl+Alt+F9','talk-cancel':'Ctrl+Alt+F10'};
    const snapshot = requested => ({requested,defaults,active:Object.fromEntries(Object.entries(requested).filter(([,key])=>key)),conflicts:[]});
    let saves=0, rejectSave, resolveSave;
    const editor = createWindowsShortcuts({read:async()=>snapshot({...defaults}),save:async()=>{
      saves++; return new Promise((resolve,reject)=>{resolveSave=resolve;rejectSave=reject;});
    }});
    const host=document.createElement('div'); document.body.append(host);
    try {
      await editor.refresh(); host.append(editor.render());
      const root=host.firstElementChild;
      const firstKey=()=>root.querySelectorAll('select')[1];
      const apply=()=>root.querySelector('[data-shortcuts-apply]');
      const change=value=>{firstKey().value=value;firstKey().dispatchEvent(new Event('change'));};
      assert(apply().disabled,'unchanged keys cannot be saved');
      change('F11'); apply().click(); apply().click();
      assert(saves===1,'saving must be single flight');
      assert([...root.querySelectorAll('select')].every(field=>field.disabled),'pending edits must be locked');
      rejectSave('shortcut-conflict:open'); await new Promise(resolve=>setTimeout(resolve,0));
      assert(firstKey().value==='F11' && !apply().disabled,'conflict retains the editable draft');
      assert(root.querySelector('[role=alert]'),'conflict must be visible');
      apply().click(); resolveSave(snapshot({...defaults,open:'Ctrl+Alt+F11'})); await new Promise(resolve=>setTimeout(resolve,0));
      assert(apply().disabled && root.querySelector('[role=status]'),'successful commit updates active state');
      root.querySelectorAll('.shortcut-actions button')[1].click();
      assert(saves===2 && firstKey().value==='Space' && !apply().disabled,'restore stages defaults without writing');
      change(''); apply().click(); resolveSave(snapshot({...defaults,open:''})); await new Promise(resolve=>setTimeout(resolve,0));
      assert(firstKey().value==='' && root.querySelector('select').disabled,'an action can be disabled');
      const unsupported=createWindowsShortcuts({read:async()=>null,save:async()=>{throw new Error('must not write');}});
      await unsupported.refresh(); assert(unsupported.render().hidden && unsupported.desktopEditor(),'other platforms keep their shortcut owner');
      let failRead=true;
      const retry=createWindowsShortcuts({read:async()=>{if(failRead)throw new Error('read');return snapshot({...defaults});},save:async()=>snapshot({...defaults})});
      await retry.refresh(); assert(retry.render().querySelector('button'),'read failure offers retry');
      assert(!retry.desktopEditor(),'an unavailable Windows worker cannot offer KDE settings');
      failRead=false; await retry.refresh(); assert(retry.supported(),'failed reads can recover');
      return 'Shortcut editor: pending, conflict/draft retention, retry, disable, staged defaults, active status and platform ownership passed.';
    } finally {host.remove();}
  })()`));
} finally { settings.close(); }
