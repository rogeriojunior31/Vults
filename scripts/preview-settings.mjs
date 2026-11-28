// Builds the real Settings page (its HTML, CSS and code) with a stand-in backend of realistic data,
// so each page can be looked at in a browser without the app: a design review aid, not a test.
//   node scripts/preview-settings.mjs <out-dir>
// then open <out-dir>/settings.html#appearance (or #zeca, #chat, #agents, #dictationvoice…), or
// shoot it with a headless Chromium. Nothing here reaches the network or the real app.
import { fileURLToPath } from "node:url";
import { mkdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { build } from "vite";

const out = resolve(process.argv[2] ?? "preview-settings");
mkdirSync(out, { recursive: true });
const ui = fileURLToPath(new URL("../ui/", import.meta.url));

// The stand-in backend: every Bridge call answers with what a set-up computer would.
const MOCK = `
const settings = { questionAgents: false, antigravityApprovals: false, labs: {}, theme: document.documentElement.dataset.theme || "dark",
  history: true, sounds: true, volume: 50, autostart: true, foldAfter: 15, openOnHover: false, monitor: null, nowPlaying: false,
  codexUsage: true, claudeUsage: true, priceUpdates: false, nativeChat: true, dictationKeep: "30", dictationSound: true,
  dictationCommands: true, dictationCommandMedia: false, dictationCommandApps: false, dictationCommandOutput: false,
  dictationCleanup: false, meetingSummary: false, dictationStyle: "auto", dictationWriteIn: null, dictationSessionsEnglish: false,
  dictationCommandChat: true, dictationMusic: "lower", dictationCancelByVoice: true, dictationSpokenCode: true,
  dictationCutRepeats: true, dictationRetryBase: true, dictationVocabulary: true, dictationProjectVocabulary: true,
  dictationMicrophone: null, dictationDoubleTap: true, dictationSpaceLock: true, dictationEscapeCancel: true,
  dictationCaption: true, dictationInsert: false, dictationInsertAvailable: true, dictationWayland: false, chatAccess: "ask",
  memory: true, journalSummary: false, localEmbeddings: false, memorySuggest: true, sleep: false, zecaSpecies: "atratus",
  zecaLook: "auto", flock: "brazil", visitors: true, presence: "island", notifications: true, quietWhenWatching: true, keepAwake: true,
  zeca: true, widget: null, dndUntil: null, language: null, lang: "en", settingsPath: "/home/me/.config/vults/settings.json",
  dataPath: "/home/me/.local/share/vults" };
const fixtures = {
  appSettings: settings,
  chatEngines: { claude: "signed-in", codex: "installed", acp: [{ id: "acp:antigravity", title: "Antigravity" }, { id: "acp:opencode", title: "OpenCode" }] },
  apiProviders: { selected: "anthropic", providers: [
    { id: "anthropic", label: "Anthropic", local: false, hasKey: true, keyHint: "sk-ant-…", defaultModel: "claude-opus-5-5", model: "claude-opus-5-5" },
    { id: "openai", label: "OpenAI", local: false, hasKey: false, keyHint: "sk-…", defaultModel: "gpt-5", model: "gpt-5" },
    { id: "openrouter", label: "OpenRouter", local: false, hasKey: true, keyHint: "sk-or-…", defaultModel: "", model: "" },
    { id: "gemini", label: "Google Gemini", local: false, hasKey: false, keyHint: "", defaultModel: "", model: "" },
    { id: "ollama", label: "Ollama", local: true, hasKey: false, keyHint: "", defaultModel: "", model: "" } ] },
  apiModels: ["claude-opus-5-5", "claude-sonnet-5-5", "claude-haiku-5-5"],
  installStatus: { installed: true, hookReady: true, statusLine: "ours", configPath: "/home/me/.claude/settings.json" },
  voiceStatus: { ready: true, selected: "small", language: "pt", system: "pt", downloading: [], models: [
    { id: "base", label: "Base: fast, but weak outside English", size: 59707625, installed: false },
    { id: "small", label: "Small: good in Portuguese and other languages", size: 190085487, installed: true },
    { id: "turbo", label: "Large v3 Turbo: the most accurate; best with a GPU", size: 574041195, installed: false } ] },
  webSettings: { on: false, engine: "brave", searxngUrl: null, hasKey: false },
  phoneSettings: { on: false, transport: "relay", relay: null, devices: [] },
  connectorsStatus: [{ id: "github", enabled: true, lastOk: Date.now() - 120000, error: null, watching: 3, overBudget: false, account: "me", host: "github.com" }],
  spendToday: { usd: 1.25, tokens: 120000, budget: 5, subscription: true },
  activity: { history: true }, updateStatus: { consent: { state: "yes" }, current: "0.1.9", latest: null, checking: false, error: null },
  modelPriceStatus: { enabled: false, updating: false, fetchedMs: null, error: null }, shortcutKeys: { talk: "Meta+Space", "talk-command": "Meta+Alt+Space", next: "Meta+]", previous: "Meta+[", open: "Meta+V", mini: "Meta+1", quick: "Meta+2", room: "Meta+3", allow: "Meta+Y", deny: "Meta+N" },
  dictationMicrophones: [{ id: "default", name: "Built-in microphone", default: true }],
  monitors: [], rulesList: [], policiesList: [], agentsDoctor: [], nestAudit: [], projectPrefs: {}, questionInstallStatus: [],
  dictationStatus: null, dictationPermission: "granted", nestIsFull: false, windowsShortcuts: null,
};
window.__TAURI_INTERNALS__ = { invoke: async () => ({}), transformCallback: () => 0, metadata: { currentWindow: { label: "settings" }, currentWebview: { label: "settings" } } };
export const Bridge = new Proxy({}, { get: (_o, name) => {
  if (typeof name !== "string") return undefined;
  if (name.startsWith("on")) return () => () => {};
  return async () => (name in fixtures ? fixtures[name] : /^(list|load|read)/.test(name) ? [] : {});
} });
`;

await build({
  configFile: false, root: ui, logLevel: "warn", base: "./",
  plugins: [{
    name: "settings-preview", enforce: "pre",
    resolveId(id) {
      if (/\/bridge(?:\.ts)?$/.test(id)) return "\0preview-bridge";
      if (id === "@tauri-apps/api/app") return "\0preview-version";
    },
    load(id) {
      if (id === "\0preview-bridge") return MOCK;
      if (id === "\0preview-version") return 'export const getVersion = async () => "0.1.9";';
    },
  }],
  build: { outDir: out, emptyOutDir: true, rollupOptions: { input: join(ui, "settings.html") } },
});
writeFileSync(join(out, "README.txt"), "Open settings.html#<page>; <html data-theme> picks the theme.\n");
console.log(`preview-settings: ${out}`);
