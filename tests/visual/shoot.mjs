// Ad-hoc screenshots of the settings window with a mocked bridge, for review outside Tauri.
// usage: PAGES=agents,chat node tests/visual/shoot.mjs <outdir> [base]
//   MOCK=<file.json>  merges more command replies into the mock (e.g. an install_preview)
//   CLICK=<text>      clicks the first button with that text before the screenshot
import { readFileSync } from "node:fs";
import { chromium } from "@playwright/test";

const out = process.argv[2];
const base = process.argv[3] ?? "http://127.0.0.1:1431";
const CONFIG = { claude: "~/.claude/settings.json", codex: "~/.codex/hooks.json" };
const MOCK = {
  api_key_status: false,
  app_settings: { sounds: true, autostart: false, foldAfter: 15 },
  rules_list: [],
  connectors_status: [],
  shortcut_keys: {},
  current_view: null,
  ...(process.env.MOCK ? JSON.parse(readFileSync(process.env.MOCK, "utf8")) : {}),
};
const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: 980, height: 720 }, deviceScaleFactor: 2 });
await context.addInitScript(
  ([mock, config]) => {
    const status = (agent) => ({
      agent,
      configPath: config[agent],
      hookPath: "~/.local/share/vultures-ai/bin/vultures-ai-hook",
      hookReady: true,
      installed: true,
      error: null,
      codex: agent === "codex" ? { hooksDisabled: false, untrusted: 0, total: 10 } : null,
    });
    window.__TAURI_INTERNALS__ = {
      invoke: async (cmd, args) => {
        if (cmd === "install_status" && !(cmd in mock)) return status(args.agent);
        if (cmd === "plugin:app|version") return "0.1.0";
        return cmd in mock ? mock[cmd] : null;
      },
      transformCallback: () => 0,
      metadata: { currentWindow: { label: "settings" }, currentWebview: { label: "settings" } },
    };
  },
  [MOCK, CONFIG],
);
for (const name of (process.env.PAGES ?? "agents,chat").split(",")) {
  const page = await context.newPage();
  page.on("pageerror", (e) => console.error(`${name}: ${e.message}`));
  await page.goto(`${base}/settings.html#${name}`);
  await page.waitForTimeout(600);
  if (process.env.CLICK) {
    await page.getByRole("button", { name: process.env.CLICK }).last().click();
    await page.waitForTimeout(400);
  }
  await page.screenshot({ path: `${out}/settings-${name}.png`, fullPage: true });
  await page.close();
}
await browser.close();
