import { el } from "./dom";
import { t } from "./i18n";
import type { QuestionAction, TextQuestionView } from "./bridge";
import "./questions.css";

export interface QuestionBackend {
  act(token: string, action: QuestionAction, text: string, capture?: number): Promise<void>;
  keyboard(on: boolean): void;
  changed(): void;
}
/** One retained editor; a server snapshot never overwrites keystrokes in the live draft. */
export function questionCard(backend: QuestionBackend) {
  const root = el("section", { class: "text-question" });
  root.hidden = true;
  root.setAttribute("role", "region");
  const title = el("strong");
  const owner = el("p", { class: "text-question-owner" });
  const prompt = el("p", { class: "text-question-prompt" });
  const label = el("label", { class: "text-question-label" });
  const input = el("textarea");
  input.rows = 3;
  input.maxLength = 2000;
  const note = el("p", { class: "text-question-note" });
  const voice = el("p", { class: "text-question-note" });
  voice.setAttribute("role", "status");
  const stop = el("button", { onclick: () => void stopDictation() });
  const error = el("p", { class: "text-question-error" });
  error.setAttribute("role", "alert");
  const cancel = el("button", { onclick: () => void act("cancel") });
  const send = el("button", { class: "primary", onclick: () => void act("send") });
  const close = el("button", { onclick: () => { hiddenSeq = view?.seq ?? 0; hide(); } });
  const labelText = el("span");
  label.append(labelText, input);
  root.append(title, owner, el("div", { class: "text-question-body" }, prompt, label, note, voice, error), el("div", { class: "text-question-actions" }, stop, cancel, close, send));
  let view: TextQuestionView | null = null;
  let token: string | null = null;
  let hiddenSeq = -1;
  let busy = false;
  let composing = false;
  let key = "";
  let applied = -1;
  const recording = () => !!view?.offer?.dictation && view.offer.dictation.phase !== "failed";
  let writes: Promise<void> = Promise.resolve();
  let timer: ReturnType<typeof setTimeout> | undefined;
  function hide() {
    root.hidden = true;
    input.value = "";
    prompt.textContent = owner.textContent = "";
    input.blur();
    backend.keyboard(false);
    backend.changed();
  }
  const valid = () => !!input.value.trim() && new TextEncoder().encode(input.value).length <= 2000;
  const controls = () => {
    send.disabled = busy || composing || recording() || !valid();
    cancel.disabled = busy;
    input.disabled = busy;
  };
  function save() {
    clearTimeout(timer);
    if (!token || busy) return;
    const asked = token, text = input.value;
    writes = writes.catch(() => {}).then(() => { if (token === asked) return backend.act(asked, "edit", text); });
    void writes.catch(() => {
      if (token === asked) error.textContent = t("Could not save this draft. Keep it within 2000 bytes and try again.");
    });
  }
  async function stopDictation() {
    if (!token) return;
    const asked = token;
    try { await backend.act(asked, "stop-dictation", ""); }
    catch { if (token === asked) error.textContent = t("Could not stop dictation. Try again."); }
  }
  function applyDictation() {
    const result = view?.offer?.dictation;
    if (!token || composing || result?.phase !== "ready" || result.id === applied || !result.text) return;
    const asked = token;
    applied = result.id;
    const text = input.value + (input.value && !/\s$/.test(input.value) ? " " : "") + result.text;
    if (new TextEncoder().encode(text).length > 2000) {
      error.textContent = t("The transcription does not fit. Your draft was kept; cancel dictation and shorten it.");
      return;
    }
    input.value = text;
    clearTimeout(timer);
    writes = writes.catch(() => {}).then(() => {
      if (token === asked) return backend.act(asked, "apply-dictation", text, result.id);
    });
    void writes.catch(() => {
      if (token === asked) error.textContent = t("Could not apply the transcription. Your draft was kept; cancel dictation to continue.");
    });
  }
  async function act(action: "send" | "cancel") {
    if (!token || busy || composing || (action === "send" && (!valid() || recording()))) return;
    clearTimeout(timer);
    const asked = token;
    const text = input.value;
    busy = true;
    error.textContent = "";
    controls();
    try {
      // Earlier edits finish before Send, so a late save cannot replace the confirmed text.
      await writes.catch(() => {});
      if (token !== asked) return;
      await backend.act(asked, action, text);
      if (token === asked) { input.value = ""; token = null; backend.keyboard(false); }
    } catch {
      if (token === asked) error.textContent = action === "send"
        ? t("The reply could not be delivered. Check the question status before doing anything else.")
        : t("Could not cancel the question. Try again.");
    } finally {
      if (token === asked) { busy = false; controls(); }
      backend.changed();
    }
  }
  input.addEventListener("input", () => { controls(); clearTimeout(timer); timer = setTimeout(save, 160); });
  input.addEventListener("focus", () => backend.keyboard(true));
  input.addEventListener("blur", () => { backend.keyboard(false); save(); });
  input.addEventListener("compositionstart", () => { composing = true; controls(); });
  input.addEventListener("compositionend", () => { composing = false; applyDictation(); controls(); });
  input.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && !event.shiftKey && !event.ctrlKey && !event.altKey && !event.metaKey
      && !event.isComposing && !composing && event.keyCode !== 229 && !event.repeat) {
      event.preventDefault(); event.stopPropagation(); void act("send");
    }
    if (event.key === "Escape" && !composing) { event.preventDefault(); event.stopPropagation(); void act("cancel"); }
  });
  function update(next: TextQuestionView) {
    if (view && next.seq < view.seq) return;
    const previous = token;
    view = next;
    const offer = next.offer;
    if (!next.surface || !next.phase || next.seq === hiddenSeq) { token = null; hide(); return; }
    root.hidden = false;
    root.setAttribute("aria-label", t("Text question"));
    title.textContent = t("Text question");
    const pending = next.phase === "pending" && !!offer;
    if (pending && offer.token !== token) {
      token = offer.token; input.value = offer.draft; error.textContent = ""; busy = false; composing = false; applied = -1;
    } else if (!pending) { token = null; input.value = ""; busy = next.phase === "sending"; composing = false; }
    owner.textContent = offer ? t("Requested by process {pid} · {seconds}s remaining", { pid: offer.clientPid, seconds: offer.seconds }) : "";
    owner.hidden = !pending;
    prompt.textContent = offer?.question ?? (next.phase === "sent" ? t("Reply sent.")
      : next.phase === "sending" ? t("Sending your confirmed reply…")
      : next.phase === "expired" ? t("The question expired. Your unsent draft was cleared.")
      : next.phase === "failed" ? t("Delivery could not be confirmed. The reply was consumed; ask a new question instead of resending.")
      : next.phase === "unavailable" ? t("The requesting agent disconnected. Your unsent draft was cleared.")
      : t("The question was cancelled. Your unsent draft was cleared."));
    labelText.textContent = t("Your reply");
    input.setAttribute("aria-label", t("Your reply"));
    note.textContent = t("Your reply is text. It does not approve a tool.");
    const dictation = offer?.dictation;
    voice.textContent = dictation?.phase === "listening" ? t("Listening… Release the dictation key to review your words.")
      : dictation?.phase === "transcribing" ? t("Transcribing locally… Your reply will not be sent.")
      : dictation?.phase === "ready" ? t("Adding your words to the draft…")
      : dictation?.phase === "failed" ? t("Dictation could not produce a reply. Check your local model and microphone, or type your reply.")
      : key ? t("Hold {key} to dictate here, then review and send separately.", { key }) : "";
    voice.hidden = !pending || !voice.textContent;
    stop.textContent = t("Cancel dictation");
    stop.hidden = !pending || !recording();
    applyDictation();
    note.hidden = !pending || !!dictation;
    label.hidden = !pending;
    error.hidden = !pending;
    cancel.textContent = t("Cancel"); send.textContent = t("Send reply"); close.textContent = t("Close");
    cancel.hidden = send.hidden = !pending;
    close.hidden = pending || next.phase === "sending";
    if (!pending && previous) { input.blur(); backend.keyboard(false); }
    controls();
    backend.changed();
  }
  return { element: root, update, shortcut: (value: string) => { key = value; if (view) update(view); }, token: () => token, clear: () => { token = null; view = null; clearTimeout(timer); hide(); } };
}
