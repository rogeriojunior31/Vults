import { el } from "../../dom";
import { t } from "../../i18n";
import type { InstallPreview, QuestionInstallStatus } from "../../bridge";
import { button, diff, row } from "./pieces";

interface Backend {
  settings(): Promise<{ questionAgents: boolean }>;
  set(on: boolean): Promise<void>;
  status(): Promise<QuestionInstallStatus[]>;
  preview(agent: string, install: boolean): Promise<InstallPreview>;
  apply(agent: string, install: boolean, fingerprint: string): Promise<string | null>;
}
/** The saved switch grants questions; only a separately reviewed click edits a client config. */
export function createQuestionAgents(backend: Backend) {
  const element = el("div", { class: "question-agents" });
  let visible = false, epoch = 0, events = 0, enabled = false, known = false;
  let reading = 0;
  let loading = false, saving = false, busy = false, failed = false, operationFailed = false;
  let statuses: QuestionInstallStatus[] = [];
  let pending: { agent: string; install: boolean; preview: InstallPreview } | null = null;
  let backup: string | null | undefined;
  async function refresh() {
    if (!visible) return;
    const asked = epoch, version = events, read = ++reading;
    loading = true; failed = false; pending = null; render();
    const [settings, status] = await Promise.allSettled([backend.settings(), backend.status()]);
    if (!visible || asked !== epoch || read !== reading) return;
    loading = false;
    if (settings.status === "fulfilled") { if (version === events) enabled = settings.value.questionAgents === true; known = true; }
    else if (version === events) { failed = true; known = false; }
    if (status.status === "fulfilled") statuses = status.value;
    else failed = true;
    render();
  }
  async function set(on: boolean) {
    if (!known || saving || busy) return;
    const version = events;
    saving = true; operationFailed = false; pending = null; render();
    try { await backend.set(on); if (events === version) enabled = on; }
    catch { operationFailed = true; }
    finally { saving = false; if (visible) render(); }
  }
  async function review(agent: string, install: boolean) {
    if (busy || saving || loading || (install && !enabled)) return;
    const asked = epoch, version = events;
    busy = true; pending = null; operationFailed = false; backup = undefined; render();
    try {
      const preview = await backend.preview(agent, install);
      if (visible && asked === epoch && version === events) pending = { agent, install, preview };
    } catch { if (asked === epoch) operationFailed = true; }
    finally { busy = false; if (visible) render(); }
  }
  async function apply() {
    if (!pending || busy || saving || loading || (pending.install && !enabled)) return;
    const plan = pending, asked = epoch;
    busy = true; pending = null; operationFailed = false; render();
    try {
      const result = await backend.apply(plan.agent, plan.install, plan.preview.fingerprint);
      if (visible && asked === epoch) { backup = result; await refresh(); }
    } catch { if (asked === epoch) operationFailed = true; }
    finally { busy = false; if (visible) render(); }
  }
  function feedback(text: string, error = false) {
    const note = el("p", { class: error ? "note error" : "note", text });
    note.setAttribute("role", error ? "alert" : "status");
    return note;
  }
  function render() {
    const active = document.activeElement as HTMLElement | null;
    const focus = active && element.contains(active) ? active.dataset.setting : null;
    const checkbox = el("input", { class: "toggle" }); checkbox.type = "checkbox";
    checkbox.checked = enabled; checkbox.disabled = !known || saving || busy;
    checkbox.addEventListener("change", () => void set(checkbox.checked));
    const consent = row(t("Agents may ask for a spoken answer"),
      t("Allow local agents to show a text question. You choose whether to dictate, review the words and send separately. This never approves a tool."), checkbox);
    checkbox.dataset.setting = "question-agents-access";
    const retry = button(t("Refresh"), () => { if (!loading && !busy) void refresh(); });
    retry.dataset.setting = "question-agents-refresh";
    (retry as HTMLButtonElement).disabled = loading || busy || saving;
    element.replaceChildren(consent,
      el("p", { class: "note", text: t("Off by default. Turning this off cancels pending external questions; their configuration files stay in place until you review removal.") }),
      el("p", { class: "note", text: t("Setup is available for Claude Code and Codex. Restart the agent after changing its configuration. Your agent may still ask before calling an MCP tool.") }), retry);
    if (loading || saving || busy) element.append(feedback(loading ? t("Loading…") : saving ? t("Saving…") : t("Working…")));
    if (failed) element.append(feedback(t("Could not load agent question settings. Refresh to try again."), true));
    if (operationFailed) element.append(feedback(t("Could not complete the change. Refresh the status and review a new diff before trying again."), true));
    if (backup !== undefined) element.append(feedback(backup
      ? t("Configuration saved. Backup: {path}", { path: backup }) : t("Configuration saved.")));
    for (const status of statuses) {
      const label = status.agent === "claude" ? "Claude Code" : "Codex";
      const section = el("section", { class: "card" }, el("h2", { text: label }),
        el("p", { class: "path", text: status.path }),
        el("p", { class: "note", text: status.error ? t("This configuration could not be read or safely edited. Check the file before trying again.")
          : status.installed ? t("Installed") : t("Not installed") }));
      if (status.error) section.append(el("details", {}, el("summary", { text: t("Technical details") }), el("pre", { text: status.error })));
      const action = button(status.installed ? t("Review removal…") : t("Review setup…"), () => void review(status.agent, !status.installed));
      (action as HTMLButtonElement).disabled = !!status.error || busy || saving || loading || (!status.installed && (!known || !enabled));
      action.dataset.setting = `question-${status.agent}`;
      const actions = el("div", { class: "actions" }, action);
      if (status.installed) {
        const setup = button(t("Review setup…"), () => void review(status.agent, true));
        (setup as HTMLButtonElement).disabled = !!status.error || busy || saving || loading || !known || !enabled;
        setup.dataset.setting = `question-setup-${status.agent}`;
        actions.append(setup);
      }
      section.append(actions);
      if (pending?.agent === status.agent) {
        const cancel = button(t("Cancel"), () => { pending = null; render(); }); cancel.dataset.setting = "question-review-cancel";
        const write = button(t("Write the file"), () => void apply(), true); write.dataset.setting = "question-review-write";
        (write as HTMLButtonElement).disabled = loading || saving || busy;
        const patch = diff(pending.preview.diff);
        patch.tabIndex = 0; patch.setAttribute("role", "region"); patch.setAttribute("aria-label", t("Configuration diff"));
        const review = el("div", { class: "review" },
          el("h3", { text: pending.install ? t("Review the install") : t("Review the removal") }),
          pending.preview.diff ? patch : el("p", { class: "note", text: t("Nothing to change.") }),
          el("div", { class: "actions" }, cancel, pending.preview.diff ? write : null));
        section.append(review);
      }
      element.append(section);
    }
    element.setAttribute("aria-busy", String(loading || saving || busy));
    if (focus) element.querySelector<HTMLElement>(`[data-setting="${focus}"]`)?.focus({ preventScroll: true });
  }
  render();
  return { element, relabel: render, refresh,
    visible(on: boolean) {
      if (visible === on) return;
      visible = on; epoch++; pending = null;
      if (on) void refresh();
    },
    changed(on: boolean) { events++; enabled = on; known = true; pending = null; if (visible) render(); },
  };
}
