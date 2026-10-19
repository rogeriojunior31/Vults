import type { ApiProvider, MeetingSummaryStatus } from "../../../bridge";
import { el } from "../../../dom";
import { t, tk } from "../../../i18n";

export interface MeetingSummaryBackend {
  settings(): Promise<{ meetingSummary: boolean }>;
  providers(): Promise<{ selected: string; providers: ApiProvider[] }>;
  status(id: number): Promise<MeetingSummaryStatus>;
  begin(id: number): Promise<MeetingSummaryStatus>;
  cancel(id: number): Promise<void>;
}
const errors = {
  quota: tk("The summary request limit or its local ledger blocked this attempt. Try later; restarting does not reset the limit."),
  configuration: tk("The selected API model or key is unavailable. Check Chat & voice settings, then try again."),
  timeout: tk("Summary generation timed out. Your transcript is kept; you can try again."),
  provider: tk("The model could not complete this summary. Your transcript is kept; you can try again."),
  invalid: tk("The model returned notes that could not be validated. Your transcript is kept; you can try again."),
  cancelled: tk("Summary generation cancelled. Your transcript is kept."),
};

export function createMeetingSummary(api: MeetingSummaryBackend | undefined, configure: () => void) {
  const state = el("p"), destination = el("p"), policy = el("p");
  const feedback = el("p", { class: "meeting-feedback" });
  const content = el("div", { class: "meeting-notes-content" });
  const generate = el("button"), cancel = el("button"), reload = el("button"), settings = el("button");
  const actions = el("div", { class: "meeting-actions" }, generate, cancel, reload, settings);
  const detailsTitle = el("summary"), details = el("details", {}, detailsTitle, destination, policy);
  let hadNotes: boolean | null = null;
  const element = el("section", { class: "meeting-notes" }, feedback, content, state, details, actions);
  state.setAttribute("role", "status"); feedback.setAttribute("role", "alert");
  let active = false, id: number | null = null, ready = false, mode: "summary" | "todos" = "summary";
  let prefs: { meetingSummary: boolean } | null = null, provider: ApiProvider | null = null;
  let value: MeetingSummaryStatus | null = null;
  let epoch = 0, prefsEpoch = 0, loading = false, reading = false, busy = false;
  let prefsFailed = false, readFailed = false, actionFailed = false, cancelling = false, contentKey = "";
  const live = () => active && !document.hidden;
  function draw() {
    const running = value?.phase === "generating";
    const on = !!prefs?.meetingSummary;
    const hasNotes = on && !!value?.notes;
    if (hasNotes !== hadNotes) { details.open = !hasNotes; hadNotes = hasNotes; }
    detailsTitle.textContent = t("Summary details"); details.hidden = !on && !ready;
    const available = !!provider?.model?.trim() && (provider.local || provider.hasKey);
    const label = !api || prefsFailed && !prefs ? t("Meeting summaries are unavailable.") : !prefs || loading ? t("Reading meeting settings…")
      : !on ? t("Meeting summaries are off. Enable them in Meeting settings to send transcript text to your chosen model.")
      : running ? t("Generating meeting notes… Your local transcript is kept.")
      : value?.phase === "cancelled" ? t("Summary generation cancelled. Your transcript is kept.")
      : !ready ? t("Finish a local transcript before generating meeting notes.")
      : value?.notes ? t("Review these model-generated notes against the transcript. They may contain mistakes.")
      : t("Generate a summary to extract key points and explicitly agreed actions.");
    if (state.textContent !== label) state.textContent = label;
    destination.textContent = provider
      ? t("Transcript text goes to {provider} ({model}). No audio is sent.", { provider: provider.label, model: provider.model || t("Choose a model") })
      : t("Choose an API provider and model in Chat & voice before enabling meeting summaries.");
    destination.hidden = !on;
    policy.textContent = on ? t("Up to 20 attempts per rolling 24 hours, including local calls and failures. Provider charges may apply. Leaving Meetings discards the transcript and notes. Notes never execute tasks.")
      : t("Enable summaries before recording. Opening settings leaves Meetings and discards this temporary transcript.");
    policy.hidden = !on && !ready;
    const error = prefsFailed ? t("Could not read meeting settings or the provider. Reload to try again.")
      : readFailed ? t("Could not update summary status. The last notes are kept. Reload before starting another attempt.")
      : actionFailed ? t("Could not confirm the summary action. Reload status or check Meeting settings before trying again.")
      : value?.error ? t(errors[value.error]) : "";
    feedback.textContent = error; feedback.hidden = !error;
    generate.textContent = t("Generate summary"); generate.classList.add("meeting-primary");
    generate.hidden = !api || !on || running || prefsFailed || readFailed;
    generate.disabled = busy || reading || loading || prefsFailed || readFailed || !ready || !available || !value;
    cancel.textContent = t("Cancel summary"); cancel.hidden = !running; cancel.disabled = busy;
    reload.textContent = t("Reload summary status"); reload.hidden = !api || !(prefsFailed || readFailed || actionFailed);
    reload.disabled = loading || reading || busy;
    settings.textContent = t("Meeting settings");
    const key = JSON.stringify([on, mode, value?.notes, t("Key points"), t("Owner: {owner}", { owner: "" })]);
    if (key !== contentKey) {
      contentKey = key; content.replaceChildren();
      const notes = on ? value?.notes : null;
      if (notes && mode === "summary") {
        const list = el("ul");
        for (const point of notes.key_points) list.append(el("li", { text: point }));
        content.append(el("h3", { text: notes.title }), el("p", { class: "meeting-note-text", text: notes.summary }),
          el("h4", { text: t("Key points") }), notes.key_points.length ? list : el("p", { text: t("No key points were returned.") }));
      } else if (notes) {
        const list = el("ol");
        for (const todo of notes.todos) list.append(el("li", {},
          el("p", { class: "meeting-note-text", text: todo.text }),
          el("p", { class: "meeting-todo-metadata", text: t("Owner: {owner}", { owner: todo.owner ?? t("Not assigned") }) }),
          el("p", { class: "meeting-todo-metadata", text: t("Due: {due}", { due: todo.due ?? t("Not stated") }) })));
        content.append(notes.todos.length ? list : el("p", { text: t("No explicitly agreed actions were found.") }));
      }
    }
    content.setAttribute("aria-busy", String(busy || reading || loading || running));
    const focused = document.activeElement;
    if ([generate, cancel, reload].includes(focused as HTMLButtonElement) && (focused as HTMLElement).hidden) {
      (!cancel.hidden && !cancel.disabled ? cancel : !generate.hidden && !generate.disabled ? generate : settings).focus({ preventScroll: true });
    }
  }
  async function preferences() {
    if (!api || !live()) return;
    const mine = ++prefsEpoch; loading = true; draw();
    const [saved, providers] = await Promise.allSettled([api.settings(), api.providers()]);
    if (mine !== prefsEpoch || !live()) return;
    prefsFailed = saved.status === "rejected" || providers.status === "rejected";
    if (saved.status === "fulfilled") prefs = saved.value;
    provider = providers.status === "fulfilled" ? providers.value.providers.find(p => p.id === providers.value.selected) ?? null : null;
    loading = false; draw();
  }
  async function poll(force = false) {
    if (!api || !live() || id === null || !ready || reading || busy || (!force && value && value.phase !== "generating" && !readFailed)) return;
    const mine = epoch, meeting = id; reading = true;
    try {
      const next = await api.status(meeting);
      if (mine !== epoch || !live()) return;
      if (next.id !== meeting) throw new Error("Stale meeting notes");
      value = next; readFailed = false;
    } catch { if (mine === epoch && live()) readFailed = true; }
    finally { reading = false; if (mine === epoch && live()) draw(); }
  }
  async function act(stop: boolean) {
    if (!api || !live() || id === null || busy || !ready) return;
    const mine = ++epoch, meeting = id; busy = true; actionFailed = false; cancelling = stop; draw();
    try {
      if (stop) await api.cancel(meeting);
      else {
        const next = await api.begin(meeting);
        if (mine === epoch && live() && id === meeting) {
          if (next.id !== meeting) throw new Error("Stale summary attempt");
          value = next;
        } else void api.cancel(meeting).catch(() => {});
      }
    } catch { if (mine === epoch && live()) actionFailed = true; }
    finally {
      busy = false; cancelling = false;
      if (mine === epoch && live()) { draw(); void poll(true); }
    }
  }
  generate.addEventListener("click", () => { if (!generate.disabled) void act(false); });
  cancel.addEventListener("click", () => void act(true));
  reload.addEventListener("click", () => { actionFailed = false; void preferences(); void poll(true); });
  settings.addEventListener("click", configure);
  draw();
  return {
    element, actions, relabel: draw, poll,
    select(next: "summary" | "todos") { mode = next; draw(); },
    refresh() { if (live()) { void preferences(); void poll(true); } },
    meeting(next: number | null, complete: boolean) {
      if (id !== next || ready !== complete) {
        ++epoch; value = null; readFailed = false; actionFailed = false; contentKey = "";
        id = next; ready = complete; draw();
      }
      void poll();
    },
    setActive(on: boolean) {
      if (active === on) return;
      active = on; ++epoch; ++prefsEpoch; loading = false;
      if (on) { void preferences(); void poll(true); }
      else {
        if (api && id !== null && (value?.phase === "generating" || busy && !cancelling)) void api.cancel(id).catch(() => {});
        value = null; contentKey = ""; draw();
      }
    },
  };
}
