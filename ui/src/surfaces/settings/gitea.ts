import { el } from "../../dom";
import { locale, t } from "../../i18n";
import { connectorError } from "../../connectors";
import { button, row } from "./pieces";

type Connection = { baseUrl: string; generation: string };
type Native = { read(): Promise<Connection>; save(base: string, token: string, generation: string, clear: boolean): Promise<Connection> };

/** Retain the form across Settings refreshes; never persist or echo the token. */
export function createGitea(native: Native) {
  const url = el("input", { class: "field" });
  url.type = "url"; url.placeholder = "https://git.example.com/"; url.maxLength = 1024;
  url.dataset.setting = "gitea-url"; url.autocomplete = "off";
  const token = el("input", { class: "field" });
  token.type = "password"; token.maxLength = 4096; token.autocomplete = "new-password";
  token.dataset.setting = "gitea-token"; token.spellcheck = false;
  const status = el("p", { class: "note" }); status.setAttribute("role", "status");
  let current: Connection | null = null;
  let busy = false, enabled = false, active = false, asked = false;
  const load = async () => {
    if (busy) return;
    token.value = "";
    busy = true; update();
    try { current = await native.read(); url.value = current.baseUrl; status.textContent = ""; }
    catch { status.textContent = t("Could not read the Gitea connection."); }
    finally { busy = false; update(); }
  };
  const write = async (clear: boolean) => {
    if (busy || enabled || !active || !current) return;
    busy = true;
    const secret = clear ? "" : token.value;
    token.value = ""; status.textContent = t("Saving…"); update();
    try {
      current = await native.save(url.value, secret, current.generation, clear);
      url.value = current.baseUrl;
      status.textContent = t("Saved. Turn on Gitea separately to start checking.");
    } catch (error) { status.textContent = connectorError(String(error)); }
    finally { busy = false; update(); }
  };
  const save = button(t("Save connection"), () => void write(false));
  const clear = button(t("Remove connection"), () => void write(true));
  const retry = button(t("Reload connection"), () => void load());
  function update() {
    url.disabled = token.disabled = busy || enabled || !current;
    save.disabled = busy || enabled || !current;
    clear.disabled = busy || enabled || !current?.generation;
    retry.disabled = busy || enabled;
    form.setAttribute("aria-busy", String(busy));
  }
  const form = el("div", { class: "gitea-connection" },
    row(t("Gitea base URL"), t("Use HTTPS, including the port and subfolder when needed. Do not append /api/v1."), url),
    row(t("Gitea access token"), t("Stored only in the OS keyring. Leave blank to keep the saved token for the same URL. Changing the URL requires a token."), token),
    el("p", { class: "note", text: t("Turn off Gitea before editing. Use a token with read:user, read:issue and read:repository scopes. Saving does not enable the connector.") }),
    el("div", { class: "head-right" }, save, clear, retry), status);
  let language = locale();
  function relabel() {
    if (language === locale()) return;
    language = locale();
    save.textContent = t("Save connection"); clear.textContent = t("Remove connection"); retry.textContent = t("Reload connection");
    const rows = form.querySelectorAll<HTMLElement>(".row");
    rows[0].querySelector<HTMLElement>(".row-title")!.textContent = t("Gitea base URL");
    rows[0].querySelector<HTMLElement>(".row-about")!.textContent = t("Use HTTPS, including the port and subfolder when needed. Do not append /api/v1.");
    rows[1].querySelector<HTMLElement>(".row-title")!.textContent = t("Gitea access token");
    rows[1].querySelector<HTMLElement>(".row-about")!.textContent = t("Stored only in the OS keyring. Leave blank to keep the saved token for the same URL. Changing the URL requires a token.");
    form.querySelector<HTMLElement>("p.note")!.textContent = t("Turn off Gitea before editing. Use a token with read:user, read:issue and read:repository scopes. Saving does not enable the connector.");
  }
  return {
    context(onPage: boolean, on: boolean) {
      relabel();
      active = onPage; enabled = on;
      if (!active) token.value = "";
      update();
      if (active && !asked) { asked = true; void load(); }
    },
    hide() { active = false; token.value = ""; },
    element: form,
  };
}
