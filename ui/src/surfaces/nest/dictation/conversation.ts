import type { VoiceConversation } from "../../../bridge";
import { el } from "../../../dom";
import { t } from "../../../i18n";

export interface ConversationBackend {
  start(autoFinish: boolean): Promise<VoiceConversation>;
  status(id: string): Promise<VoiceConversation>;
  control(id: string, action: "finish" | "cancel" | "next"): Promise<void>;
  copy(text: string): Promise<void>;
}
const running = (view: VoiceConversation | null) => !!view && ["starting", "recording", "transcribing", "responding", "waiting"].includes(view.phase);
export function createVoiceConversation(api: ConversationBackend | undefined, configure: () => void) {
  const title = el("h2"), about = el("p"), privacy = el("p", { class: "dictation-note" });
  const status = el("p"), error = el("p", { class: "meeting-feedback" });
  status.setAttribute("role", "status"); status.setAttribute("aria-live", "polite"); error.setAttribute("role", "alert");
  const start = el("button"), finish = el("button"), cancel = el("button"), settings = el("button", { onclick: configure });
  const actions = el("div", { class: "voice-conversation-actions" }, start, finish, cancel, settings);
  const reply = el("p", { class: "voice-conversation-text" });
  const draftTitle = el("span"), draft = el("textarea"), copy = el("button"), copied = el("p");
  draft.maxLength = 4096; draft.rows = 5;
  const review = el("section", { class: "voice-conversation-review" }, el("label", {}, draftTitle, draft), copy, copied);
  const automatic = el("input"), automaticTitle = el("span"), automaticNote = el("p", { class: "dictation-note" });
  automatic.type = "checkbox"; automatic.checked = false;
  const automaticChoice = el("label", { class: "voice-conversation-automatic" }, automatic, automaticTitle);
  const content = el("div", { class: "voice-conversation-content" }, privacy, automaticChoice, automaticNote, reply, review);
  content.tabIndex = 0; content.setAttribute("role", "region");
  const element = el("section", { class: "voice-conversation" }, el("header", { class: "dictation-heading" }, el("div", {}, title, about)), status, error, actions, content);
  let view: VoiceConversation | null = null, active = false, pending = false, epoch = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let failed: "start" | "status" | null = null;
  let copyState: "idle" | "done" | "failed" = "idle";
  function draw() {
    title.textContent = t("Voice conversation");
    content.setAttribute("aria-label", t("Voice conversation"));
    about.textContent = t("Finish each recording to hear Zeca. Record another turn to continue the same conversation.");
    privacy.textContent = t("Microphone audio and cached flock context may go to OpenAI. API charges apply. Voice never answers permission cards.");
    start.textContent = view?.phase === "waiting" ? t("Record another turn") : t("Start recording"); finish.textContent = t("Finish recording"); cancel.textContent = view?.phase === "waiting" ? t("End conversation") : t("Cancel"); settings.textContent = t("Voice conversation settings");
    start.disabled = !api || !active || pending || (running(view) && view?.phase !== "waiting");
    finish.disabled = !active || pending || view?.phase !== "recording";
    cancel.disabled = !active || !running(view);
    settings.disabled = pending || running(view);
    automatic.disabled = !active || pending || running(view);
    automaticTitle.textContent = t("Finish automatically after a pause");
    automaticNote.textContent = t("Off by default for each app run. Uses local GPU preview and the installed speech detector. If unavailable, use Finish recording. Each new turn still needs your click.");
    status.textContent = !api ? t("Voice conversation is unavailable.")
      : view?.phase === "starting" ? t("Starting microphone…")
      : view?.phase === "recording" && view.reason === "auto-unavailable" ? t("Automatic finish is unavailable. Use Finish recording.")
      : view?.phase === "recording" && automatic.checked ? t("Recording… Pause to finish automatically, or press Finish recording.")
      : view?.phase === "recording" ? t("Recording… Finish when you are done.")
      : view?.phase === "transcribing" ? t("Transcribing…")
      : view?.phase === "responding" ? t("Zeca is responding…")
      : view?.phase === "waiting" ? t("Microphone off. Record another turn to continue, or end the conversation.")
      : view?.phase === "recovery" ? t("Voice could not finish. Review your local transcript below.")
      : view?.phase === "finished" ? t("Turn finished.")
      : view?.phase === "cancelled" ? t("Cancelled.")
      : view?.phase === "failed" ? t("Voice conversation failed. Check your settings before trying again.")
      : t("Ready to record. Recording starts only when you press Start.");
    error.textContent = failed === "start" ? t("Could not start recording. Check voice settings and try again.") : failed ? t("Could not confirm voice status. Recording will stop automatically. Check your settings before trying again.") : "";
    error.hidden = !failed;
    reply.textContent = view?.reply ?? ""; reply.hidden = !reply.textContent;
    review.hidden = !draft.value;
    draftTitle.textContent = t("Review recovered text");
    copy.textContent = t("Copy text"); copy.disabled = !draft.value.trim();
    copied.textContent = copyState === "done" ? t("Copied") : copyState === "failed" ? t("Could not copy the text. Select and copy it manually.") : "";
  }
  function stop(id: string) { void api?.control(id, "cancel").catch(() => {}); }
  function schedule() {
    clearTimeout(timer);
    if (active && running(view)) timer = setTimeout(() => void poll(), 500);
  }
  function accept(value: VoiceConversation) {
    view = value;
    if (value.draft !== null) draft.value = value.draft;
    draw(); schedule();
  }
  async function poll() {
    if (!active || !api || !view || !running(view)) return;
    const mine = epoch, id = view.id;
    try {
      const value = await api.status(id);
      if (active && mine === epoch && view?.id === id) accept(value);
    } catch {
      if (mine !== epoch) return;
      stop(id); view = null; failed = "status"; draw();
    }
  }
  start.onclick = async () => {
    if (!api || start.disabled) return;
    clearTimeout(timer);
    const mine = ++epoch; pending = true; failed = null; copyState = "idle"; draw();
    try {
      const id = view?.phase === "waiting" ? view.id : null;
      const value = id ? await api.control(id, "next").then(() => api.status(id)) : await api.start(automatic.checked);
      if (!active || mine !== epoch) { stop(value.id); return; }
      draft.value = ""; accept(value);
    } catch { if (mine === epoch) { if (view && running(view)) stop(view.id); view = null; failed = "start"; } }
    finally { if (mine === epoch) { pending = false; draw(); } }
  };
  finish.onclick = async () => {
    if (!api || finish.disabled || !view) return;
    const mine = epoch, id = view.id; pending = true; draw();
    try { await api.control(id, "finish"); }
    catch { if (mine === epoch) { stop(id); view = null; failed = "status"; } }
    finally { if (mine === epoch) { pending = false; draw(); } }
  };
  cancel.onclick = () => {
    if (cancel.disabled || !view) return;
    ++epoch; clearTimeout(timer); stop(view.id); view = { ...view, phase: "cancelled", draft: null, reply: null }; pending = false; draw();
  };
  copy.onclick = async () => {
    if (!api || copy.disabled) return;
    const mine = epoch, text = draft.value;
    let outcome: "done" | "failed";
    try { await api.copy(text); outcome = "done"; } catch { outcome = "failed"; }
    if (mine === epoch && text === draft.value) { copyState = outcome; draw(); }
  };
  automatic.addEventListener("change", draw);
  draft.addEventListener("input", () => { copyState = "idle"; draw(); });
  function setActive(on: boolean) {
    if (active === on) return;
    active = on;
    if (!on) {
      ++epoch; clearTimeout(timer); pending = false;
      if (view) { stop(view.id); if (running(view)) view = { ...view, phase: "cancelled", draft: null, reply: null }; }
    }
    draw();
  }
  draw();
  return { element, relabel: draw, setActive };
}
