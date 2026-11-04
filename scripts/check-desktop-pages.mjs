// Opt-in smoke test against the real development WebView2, with no provider calls or settings writes.
import { mkdir, writeFile } from 'node:fs/promises';
import auditHitTargets from './desktop-hit-targets.mjs';

const port = Number(process.env.VULTS_WEBVIEW_DEBUG_PORT ?? 9223);
const output = 'target/windows-ui-qa';
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const pages = () => fetch(`http://127.0.0.1:${port}/json/list`).then(response => response.json());
class Page {
  constructor(target) {
    if (!target) throw new Error('The requested Vults webview is missing.');
    this.socket = new WebSocket(target.webSocketDebuggerUrl);
    this.pending = new Map();
    this.id = 0;
    this.ready = new Promise((resolve, reject) => {
      this.socket.onopen = resolve;
      this.socket.onerror = reject;
    });
    this.socket.onmessage = event => {
      const value = JSON.parse(event.data), call = this.pending.get(value.id);
      if (!call) return;
      clearTimeout(call.timer);
      this.pending.delete(value.id);
      if (value.error) call.reject(new Error(value.error.message));
      else call.resolve(value.result);
    };
  }
  async call(method, params = {}) {
    await this.ready;
    return new Promise((resolve, reject) => {
      const id = ++this.id;
      const timer = setTimeout(() => { this.pending.delete(id); reject(new Error(`${method} timed out`)); }, 12000);
      this.pending.set(id, { resolve, reject, timer });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
  }
  async evaluate(expression) {
    const value = await this.call('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
    if (value.exceptionDetails) throw new Error(value.exceptionDetails.text);
    return value.result.value;
  }
  invoke(command, args) {
    return this.evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`);
  }
  async screenshot(name) {
    const capture = await this.call('Page.captureScreenshot', { format: 'png' });
    await writeFile(`${output}/${name}.png`, Buffer.from(capture.data, 'base64'));
  }
  close() { this.socket.close(); }
}

await mkdir(output, { recursive: true });
const find = (targets, path) => targets.find(target => new URL(target.url).pathname === path);
const island = new Page(find(await pages(), '/'));
await island.invoke('open_nest_window', { tab: 'flock', full: false });
let targets;
for (let attempt = 0; attempt < 50; attempt++) {
  targets = await pages();
  if (find(targets, '/nest.html') && find(targets, '/settings.html')) break;
  await pause(100);
}
const nest = new Page(find(targets, '/nest.html'));
const settings = new Page(find(targets, '/settings.html'));
const oldThemes = await Promise.all([nest, settings].map(page => page.evaluate('document.documentElement.getAttribute("data-theme")')));
const tabs = ['flock', 'flock-profiles', 'chat', 'memory', 'dictation', 'dictation-history', 'dictation-commands',
  'dictation-dictionary', 'dictation-snippets', 'dictation-meetings', 'dictation-conversation', 'dictation-statistics',
  'connections', 'activity', 'week', 'projects', 'history', 'cost', 'traces', 'audit'];
const sections = ['general', 'dictation', 'chat', 'flock', 'agents', 'approvals', 'projects', 'activity', 'connectors', 'phone', 'about',
  ...['voice', 'conversation', 'keys', 'commands', 'words', 'styles', 'meetings', 'agents', 'paste', 'privacy', 'diagnostics'].map(id => `dictation${id}`)];
const viewports = [[480, 360], [560, 540], [760, 540], [1000, 720], [1180, 820], [1600, 900]];
const scales = [1, 1.25, 1.5, 1.75, 2];
const results = [];
const expression = `(${auditHitTargets.toString()})()`;
let lastUpdate = Date.now();
try {
  for (const theme of ['dark', 'light']) for (const scale of scales) for (const [width, height] of viewports) {
    for (const page of [nest, settings]) {
      await page.call('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: scale, mobile: false });
      await page.evaluate(`document.documentElement.setAttribute('data-theme',${JSON.stringify(theme)})`);
    }
    for (const [page, destinations, command, argument, group] of [
      [nest, tabs, 'open_nest_window', 'tab', 'nest'],
      [settings, sections, 'open_settings_window', 'section', 'settings'],
    ]) for (const destination of destinations) {
      await island.invoke(command, { [argument]: destination, full: false });
      await pause(50);
      const row = { theme, scale, width, height, page: `${group}/${destination}`, ...await page.evaluate(expression) };
      if (await page.evaluate('document.hidden')) throw new Error(`Requested page remained hidden: ${row.page}`);
      results.push(row);
      const failed = row.failed.length || row.headerClipped.length || row.overflow;
      if (failed || (theme === 'dark' && ((scale === 1 && width === 1180) || (scale === 2 && width === 480)))) {
        await page.screenshot(`${group}-${destination}-${theme}-${width}-${scale}`);
      }
      if (Date.now() - lastUpdate > 15000) {
        console.log(`Checked ${results.length} desktop page/scale combinations.`);
        lastUpdate = Date.now();
      }
    }
  }
} finally {
  for (const [index, page] of [nest, settings].entries()) {
    await page.call('Emulation.clearDeviceMetricsOverride');
    await page.evaluate(`document.documentElement.setAttribute('data-theme',${JSON.stringify(oldThemes[index] ?? 'dark')})`);
  }
  await island.invoke('open_nest_window', { tab: 'flock', full: false });
  for (const page of [island, nest, settings]) page.close();
  await writeFile(`${output}/matrix.json`, JSON.stringify(results, null, 2));
}
const failures = results.filter(row => row.failed.length || row.headerClipped.length || row.overflow);
const summary = { cases: results.length, clickTargets: results.reduce((total, row) => total + row.checked, 0), failures: failures.length,
  scales, viewports, nativeDpiAcceptance: 'Renderer emulation; native multi-monitor DPI requires a desktop test.', samples: failures.slice(0, 8) };
await writeFile(`${output}/summary.json`, JSON.stringify(summary, null, 2));
console.log(JSON.stringify(summary, null, 2));
if (failures.length) process.exitCode = 1;
