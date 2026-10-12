import { expect, test, type Page } from "@playwright/test";

// Settings (L6) wears the Living Flock prototype's look: the Nest's frame, the sidebar in groups
// with icons, a kicker over each page's title, rows in cards. Its page talks to a mocked bridge,
// as tests/visual/shoot.mjs does; every reply it does not know is null.
const MOCK: Record<string, unknown> = {
  api_key_status: false,
  app_settings: { sounds: true, autostart: false, foldAfter: 15, volume: 50, zeca: true, lang: "en", zecaSpecies: "atratus", settingsPath: "~/.config/vults/settings.json", dataPath: "~/.local/share/vults/" },
  rules_list: [],
  connectors_status: [],
  shortcut_keys: {},
  current_view: { sessions: [], approval: null, alerts: [] },
  nest_is_full: false,
  monitors: [],
  voice_status: { models: [], selected: null, language: null, system: "en", downloading: [], ready: false },
  agents_doctor: [],
};

/** A page that threw drew only part of itself: the picture would hide it. */
function errorsOf(page: Page): string[] {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  return errors;
}

async function open(page: Page, section: string) {
  await page.setViewportSize({ width: 1040, height: 760 });
  await page.addInitScript((mock) => {
    const status = (agent: string) => ({
      agent,
      configPath: `~/.${agent}/settings.json`,
      hookPath: "~/.local/share/vults/bin/vults-hook",
      hookReady: true,
      installed: true,
      error: null,
      codex: agent === "codex" ? { hooksDisabled: false, untrusted: 0, total: 10, error: null } : null,
    });
    (window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
      invoke: async (cmd: string, args: { agent: string }) => {
        if (cmd === "install_status" && !(cmd in mock)) return status(args.agent);
        if (cmd === "plugin:app|version") return "0.1.0";
        return cmd in mock ? mock[cmd] : null;
      },
      transformCallback: () => 0,
      metadata: { currentWindow: { label: "nest-window" }, currentWebview: { label: "settings" } },
    };
  }, MOCK);
  await page.goto(`/settings.html?still=1#${section}`);
}

test("settings: the sidebar in groups, the page under the Nest's frame", async ({ page }) => {
  const errors = errorsOf(page);
  await open(page, "general");
  await expect(page.locator(".nav-group")).toHaveText(["Experience", "Work", "System"]);
  await expect(page.locator(".nav.on")).toHaveText("General");
  await expect(page.locator(".nav.on")).toHaveAttribute("aria-current", "page");
  await expect(page.locator(".page-kicker")).toHaveText("Settings");
  await expect(page.locator("h1")).toHaveText("General");
  // The frame: the Settings tab chosen, no search (the palette lists sessions: the Nest's).
  await expect(page.locator(".suite-tab.on .nav-label")).toHaveText("Settings");
  await expect(page.getByRole("button", { name: "Command palette (Ctrl+K)" })).toHaveCount(0);
  await expect(page).toHaveScreenshot("settings-general.png");
  expect(errors).toEqual([]);
});

test("settings: a page from the sidebar", async ({ page }) => {
  const errors = errorsOf(page);
  await open(page, "general");
  await page.locator(".nav", { hasText: "Agents" }).click();
  await expect(page.locator("h1")).toHaveText("Agents");
  await expect(page.locator(".nav.on")).toHaveText("Agents");
  await expect(page).toHaveScreenshot("settings-agents.png");
  expect(errors).toEqual([]);
});
