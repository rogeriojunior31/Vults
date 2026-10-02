// Ad-hoc screenshots of the settings window with a mocked bridge, for review outside Tauri.
// usage: PAGES=agents,chat node tests/visual/shoot.mjs <outdir> [base]
import { chromium } from "@playwright/test";

const out = process.argv[2];
const base = process.argv[3] ?? "http://127.0.0.1:1431";
const MOCK = {
  install_status: { agent: "claude", configPath: "~/.claude/settings.json", hookPath: "~/.local/bin/vultures-ai-hook", hookReady: true, installed: true, error: null, codex: null },
  api_key_status: false,
  app_settings: { sounds: true, autostart: false, foldAfter: 15 },
  rules_list: [],
  connectors_status: [],
  shortcut_keys: {},
  current_view: null,
};
const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: 980, height: 720 }, deviceScaleFactor: 2 });
await context.addInitScript((mock) => {
  window.__TAURI_INTERNALS__ = {
    invoke: async (cmd) => (cmd in mock ? mock[cmd] : cmd === "plugin:app|version" ? "0.1.0" : null),
    transformCallback: () => 0,
    metadata: { currentWindow: { label: "settings" }, currentWebview: { label: "settings" } },
  };
}, MOCK);
for (const name of (process.env.PAGES ?? "agents,chat").split(",")) {
  const page = await context.newPage();
  await page.goto(`${base}/settings.html#${name}`);
  await page.waitForTimeout(600);
  await page.screenshot({ path: `${out}/settings-${name}.png` });
  await page.close();
}
await browser.close();
