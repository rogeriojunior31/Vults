import type { VoiceConversation, VoiceRoutes, VoiceRouteSelection } from "../../../bridge";
import { createVoiceRoute } from "./voice-route";
import { el } from "../../../dom";
import { t } from "../../../i18n";

export interface ConversationBackend {
  options?(): Promise<{ semantic: boolean }>;
  routes?(): Promise<VoiceRoutes>;
  start(autoFinish: boolean, route?: VoiceRouteSelection, semantic?: boolean): Promise<VoiceConversation>;
  status(id: string): Promise<VoiceConversation>;
  control(id: string, action: "finish" | "cancel" | "next" | "interrupt", response?: string, capture?: string): Promise<void>;
  copy(text: string): Promise<void>;
}
const running = (view: VoiceConversation | null) => !!view && ["preparing", "starting", "recording", "transcribing", "responding", "speaking", "interrupting", "waiting"].includes(view.phase);
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
  automatic.dataset.voiceAutomatic = ""; automatic.type = "checkbox"; automatic.checked = false;
  const automaticChoice = el("label", { class: "voice-conversation-automatic" }, automatic, automaticTitle);
  const semantic = el("input"), semanticTitle = el("span"), semanticNote = el("p", { class: "dictation-note" }), semanticFeedback = el("p", { class: "dictation-note" });
  semantic.type = "checkbox"; semantic.checked = false; semantic.dataset.voiceSemantic = "";
  semanticFeedback.setAttribute("role", "status");
  const semanticChoice = el("label", { class: "voice-conversation-automatic" }, semantic, semanticTitle);
  const route = createVoiceRoute(api?.routes, () => { if (!route.manual()) { automatic.checked = false; semantic.checked = false; } draw(); });
  const content = el("div", { class: "voice-conversation-content" }, privacy, semanticFeedback, reply, review, route.element, automaticChoice, automaticNote, semanticChoice, semanticNote);
  content.tabIndex = 0; content.setAttribute("role", "region");
  const element = el("section", { class: "voice-conversation" }, el("header", { class: "dictation-heading" }, el("div", {}, title, about)), status, error, actions, content);
  let view: VoiceConversation | null = null, active = false, pending = false, epoch = 0;
  let semanticAvailable = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let failed: "start" | "status" | null = null;
  let copyState: "idle" | "done" | "failed" = "idle";
  function draw() {
    route.update(active, pending || running(view));
    title.textContent = t("Voice conversation");
    content.setAttribute("aria-label", t("Voice conversation"));
    about.textContent = t("Finish each recording to hear Zeca. Record another turn to continue the same conversation.");
    privacy.textContent = t("Microphone audio and cached flock context may go to OpenAI. API charges apply. Voice never answers permission cards.");
    start.textContent = view?.phase === "waiting" ? t("Record another turn") : t("Start recording"); finish.textContent = view?.phase === "speaking" || view?.phase === "interrupting" ? t("Stop response") : t("Finish recording"); cancel.textContent = view?.phase === "waiting" ? t("End conversation") : t("Cancel"); settings.textContent = t("Voice conversation settings");
    start.disabled = !api || !active || pending || (running(view) && view?.phase !== "waiting") || (!running(view) && !route.ready());
    finish.disabled = !active || pending || (view?.phase !== "recording" && view?.phase !== "speaking");
    cancel.disabled = !active || !running(view);
    settings.disabled = pending || running(view);
    automatic.disabled = !active || pending || running(view) || !route.manual();
    automaticChoice.hidden = automaticNote.hidden = !route.manual();
    semanticChoice.hidden = semanticNote.hidden = !semanticAvailable || !automatic.checked || !route.manual();
    semantic.disabled = !active || pending || running(view) || !semanticAvailable || !automatic.checked || !route.manual();
    semanticTitle.textContent = t("Use local semantic turn detection (preview)");
    semanticNote.textContent = t("Off by default. Requires the checked local model and runtime. May wait up to 1.2 seconds beyond the silence policy. One failure switches this conversation to local silence detection. No downloads or microphone reopening.");
    semanticFeedback.textContent = view?.semanticFallback ? t("Semantic finish is unavailable. This conversation uses local silence detection.") : "";
    semanticFeedback.hidden = !view?.semanticFallback || view.reason === "auto-unavailable";
    automaticTitle.textContent = t("Finish automatically after a pause");
    automaticNote.textContent = t("Off by default for each app run. Uses local GPU preview and the installed speech detector. If unavailable, use Finish recording. Each new turn still needs your click.");
    status.textContent = !api ? t("Voice conversation is unavailable.")
      : view?.phase === "preparing" ? t("Preparing local semantic detection. Microphone is off.")
      : view?.phase === "starting" ? t("Starting microphone…")
      : view?.phase === "recording" && view.reason === "auto-unavailable" ? t("Automatic finish is unavailable. Use Finish recording.")
      : view?.phase === "recording" && automatic.checked ? t("Recording… Pause to finish automatically, or press Finish recording.")
      : view?.phase === "recording" ? t("Recording… Finish when you are done.")
      : view?.phase === "transcribing" && view.routed ? t("Transcribing locally. Your microphone remains open.")
      : view?.phase === "transcribing" ? t("Transcribing…")
      : view?.phase === "speaking" && view.routed ? t("Zeca is speaking. Your microphone is listening for an interruption.")
      : view?.phase === "speaking" ? t("Zeca is speaking…")
      : view?.phase === "interrupting" ? t("Stopping the response…")
      : view?.phase === "responding" && view.routed ? t("Zeca is responding. Your microphone remains open.")
      : view?.phase === "responding" ? t("Zeca is responding…")
      : view?.phase === "waiting" && view.routed ? t("Microphone remains open for this conversation. Record another turn, or end the conversation to close it.")
      : view?.phase === "waiting" ? t("Microphone off. Record another turn to continue, or end the conversation.")
      : view?.phase === "recovery" ? t("Voice could not finish. Review your local transcript below.")
      : view?.phase === "finished" ? t("Turn finished.")
      : view?.phase === "cancelled" ? t("Cancelled.")
      : view?.phase === "failed" ? t("Voice conversation failed. Check your settings before trying again.")
      : t("Ready to record. Recording starts only when you press Start.");
    error.textContent = failed === "start" && !route.manual() ? t("Could not start recording. Check voice settings, refresh audio devices and confirm the route again.") : failed === "start" ? t("Could not start recording. Check voice settings and try again.") : failed ? t("Could not confirm voice status. Recording will stop automatically. Check your settings before trying again.") : "";
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
    if (running(view) && !running(value)) { route.clearConsent(); semantic.checked = false; }
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
      stop(id); view = null; route.clearConsent(); failed = "status"; draw();
    }
  }
  start.onclick = async () => {
    if (!api || start.disabled) return;
    clearTimeout(timer);
    const mine = ++epoch; pending = true; failed = null; copyState = "idle"; draw();
    try {
      const id = view?.phase === "waiting" ? view.id : null;
      const value = id ? await api.control(id, "next").then(() => api.status(id)) : await api.start(automatic.checked, route.selection(), semantic.checked);
      if (!active || mine !== epoch) { stop(value.id); return; }
      draft.value = ""; accept(value);
    } catch { if (mine === epoch) { if (view && running(view)) stop(view.id); view = null; route.expire(); semantic.checked = false; failed = "start"; } }
    finally { if (mine === epoch) { pending = false; draw(); } }
  };
  finish.onclick = async () => {
    if (!api || finish.disabled || !view) return;
    const mine = epoch, id = view.id; pending = true; draw();
    const action = view.phase === "speaking" ? "interrupt" : "finish", recording = view.capture;
    try {
      await api.control(id, action, action === "interrupt" ? view.response ?? undefined : undefined, action === "finish" ? recording ?? undefined : undefined);
      if (mine === epoch && action === "finish" && view?.phase === "recording" && view.capture === recording) view = { ...view, phase: "transcribing", capture: null };
      if (mine === epoch && action === "interrupt" && view?.phase === "speaking") view = { ...view, phase: "interrupting" };
    }
    catch { if (mine === epoch) { stop(id); view = null; route.clearConsent(); failed = "status"; } }
    finally { if (mine === epoch) { pending = false; draw(); } }
  };
  cancel.onclick = () => {
    if (cancel.disabled || !view) return;
    ++epoch; clearTimeout(timer); stop(view.id); route.clearConsent(); semantic.checked = false; view = { ...view, phase: "cancelled", draft: null, reply: null }; pending = false; draw();
  };
  copy.onclick = async () => {
    if (!api || copy.disabled) return;
    const mine = epoch, text = draft.value;
    let outcome: "done" | "failed";
    try { await api.copy(text); outcome = "done"; } catch { outcome = "failed"; }
    if (mine === epoch && text === draft.value) { copyState = outcome; draw(); }
  };
  automatic.addEventListener("change", () => { if (!automatic.checked) semantic.checked = false; draw(); });
  semantic.addEventListener("change", draw);
  draft.addEventListener("input", () => { copyState = "idle"; draw(); });
  function setActive(on: boolean) {
    if (active === on) return;
    active = on;
    if (on && api?.options) {
      const mine = epoch;
      void api.options().then(value => { if (active && mine === epoch) { semanticAvailable = value.semantic; draw(); } }).catch(() => {});
    }
    if (!on) {
      ++epoch; clearTimeout(timer); pending = false; route.leave(); semantic.checked = false;
      if (view) { stop(view.id); if (running(view)) view = { ...view, phase: "cancelled", draft: null, reply: null }; }
    }
    draw();
  }
  draw();
  return { element, relabel: draw, setActive };
}
