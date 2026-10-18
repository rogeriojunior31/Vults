import { el } from "../dom";
import { locale, t, tk } from "../i18n";
import { Bridge, type DictationStatus, type ViewModel } from "../bridge";
import { icon, type IconName } from "./icons";
import { dictationCaptionTop } from "./geometry";

const ended = new Set(["session", "held", "pasted", "copied", "empty", "cancelled", "failed"]);
const delivered = new Set(["session", "held", "pasted", "copied"]);

const copyErrors = new Set([
  tk("Could not read dictation words."),
  tk("Dictation text processing exceeded its size limit."),
  tk("This Wayland desktop does not provide background clipboard access."),
  tk("The X11 clipboard is unavailable."),
  tk("Dictation text contains a NUL character."),
  tk("The Wayland clipboard refused the text."),
  tk("The X11 clipboard refused the text."),
  tk("The Wayland clipboard did not confirm the text."),
  tk("The Wayland clipboard could not be read back."),
  tk("The X11 clipboard did not confirm the text."),
  tk("The clipboard changed before the copy could be confirmed."),
  tk("The clipboard owner is unavailable."),
]);

export function dictationStatus(inNest = false, mount?: (line: HTMLElement | null) => void, onStart?: () => void) {
  const line = el("div", { class: "dictation-status" });
  const mark = el("span", { class: "dictation-mark" });
  const bars = Array.from({ length: 9 }, () => el("i"));
  const bands = el("span", { class: "dictation-bands" }, ...bars);
  bands.setAttribute("aria-hidden", "true");
  bands.hidden = true;
  const words = el("bdi", { class: "dictation-words" });
  const metrics = el("bdi", { class: "dictation-metrics" });
  metrics.hidden = true;
  line.append(mark, bands, words, metrics);
  line.classList.toggle("in-nest", inNest);
  line.classList.toggle("in-island", !!mount);
  line.setAttribute("role", "status");
  line.setAttribute("aria-live", "polite");
  line.setAttribute("aria-atomic", "true");
  line.hidden = true;
  if (!mount) document.body.append(line);
  const caption = el("div", { class: "dictation-caption" });
  caption.setAttribute("dir", "auto");
  caption.hidden = true;
  document.body.append(caption);
  let visible = true;
  let soundEnabled = false;
  let soundSettingChanged = false;
  let captionsEnabled = false;
  let captionSettingChanged = false;
  let captionExpired = false;
  let captionDeadline: number | null = null;
  let anchor: DOMRect | undefined;
  let anchorVisible = false;
  let requestedHidden: boolean | undefined;
  let blocked = false;
  let expired = true;
  let current: DictationStatus | undefined;
  let drawnPhase: DictationStatus["phase"] | undefined;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let captionTimer: ReturnType<typeof setTimeout> | undefined;
  function paintCaption() {
    const hide = () => {
      caption.hidden = true;
      if (caption.textContent) caption.textContent = "";
      if (inNest) document.body.classList.remove("dictation-caption-open");
    };
    if (!captionsEnabled || !visible || blocked || expired || line.hidden || !current || captionExpired
      || (captionDeadline !== null && Date.now() >= captionDeadline) || (mount && !anchorVisible)) {
      hide(); return;
    }
    const text = delivered.has(current.phase) && current.text ? current.text
      : words.clientWidth > 0 && words.scrollWidth > words.clientWidth ? words.textContent : "";
    if (!text) { hide(); return; }
    const rect = mount ? anchor : line.getBoundingClientRect();
    if (!rect || rect.width < 120) { hide(); return; }
    caption.style.width = `${Math.min(rect.width, Math.max(0, window.innerWidth - 24))}px`;
    if (caption.textContent !== text) caption.textContent = text;
    caption.hidden = false;
    const height = caption.offsetHeight;
    const top = dictationCaptionTop(rect, height, window.innerHeight,
      inNest || document.body.classList.contains("dock-bottom"));
    if (top === null) { hide(); return; }
    caption.style.top = `${top}px`;
    if (inNest) {
      document.body.style.setProperty("--dictation-caption-space", `${height + line.offsetHeight + 8}px`);
      document.body.classList.add("dictation-caption-open");
    }
  }
  const relabel = () => {
    if (!current) return;
    const text = {
      listening: () => current?.partial ? t("Listening: {text}", { text: current.partial }) : t("Dictation: listening…"),
      locked: () => current?.partial ? t("Hands-free: {text}", { text: current.partial }) : t("Dictation: hands-free listening…"),
      finishing: () => t("Dictation: finishing recording…"),
      working: () => t("Dictation: transcribing locally…"),
      retrying: () => t("Dictation: retrying with the installed Base model…"),
      pasted: () => t("Pasted in {app}. Review it in your editor.", { app: current?.app ?? "" }),
      session: () => t("In session {session}. Review it in your terminal.", { session: current?.session_name ?? "" }),
      held: () => t("Copied. This session is waiting; review its card or terminal."),
      copied: () => current?.app
        ? current.terminal
          ? t("Copied from dictation in {app}. Use Ctrl+Shift+V to paste.", { app: current.app })
          : t("Copied from dictation in {app}. Paste it in your app.", { app: current.app })
        : t("Copied. Use your app's Paste command."),
      empty: () => t("No dictation text to copy."),
      cancelled: () => t("Dictation cancelled."),
      failed: () => t("Dictation failed: {error}", { error: current?.error ? (copyErrors.has(current.error) ? t(current.error) : current.error) : "" }),
    }[current.phase];
    const glyph: Record<DictationStatus["phase"], IconName> = {
      listening: "mic", locked: "pin", finishing: "stop", working: "bars", retrying: "retry",
      pasted: "check", session: "flock", held: "pause", copied: "copy", empty: "mute",
      cancelled: "close", failed: "close",
    };
    if (drawnPhase !== current.phase) {
      mark.replaceChildren(icon(glyph[current.phase], 12));
      drawnPhase = current.phase;
    }
    const sentence = text();
    if (words.textContent !== sentence) words.textContent = sentence;
    const count = current.words;
    const measured = delivered.has(current.phase) && typeof count === "number" && Number.isSafeInteger(count) && count >= 0;
    metrics.hidden = !measured;
    metrics.textContent = measured ? count === 1 ? t("1 word") : t("{count} words", { count: count.toLocaleString(locale()) }) : "";
    line.dataset.phase = current.phase;
    paintCaption();
  };
  const show = () => {
    const hidden = !visible || blocked || expired;
    const changed = requestedHidden !== hidden;
    // Meter frames must not undo renderer priority or remeasure an unchanged row.
    if (changed) {
      requestedHidden = hidden;
      line.hidden = hidden;
      mount?.(hidden ? null : line);
    }
    bands.hidden = line.hidden || !current || !["listening", "locked"].includes(current.phase);
    if (!bands.hidden) for (const [i, bar] of bars.entries()) {
      const value = current?.bands?.[i];
      bar.style.transform = `scaleY(${typeof value === "number" && Number.isFinite(value) ? Math.max(0.08, Math.min(1, value)) : 0.08})`;
    }
    if (changed) paintCaption();
  };
  const view = {
    relabel,
    startSound(on: boolean, initial = false) {
      if (!initial || !soundSettingChanged) soundEnabled = on;
      if (!initial) soundSettingChanged = true;
      return soundEnabled;
    },
    captions(on: boolean, initial = false) {
      if (!initial || !captionSettingChanged) captionsEnabled = on;
      if (!initial) captionSettingChanged = true;
      paintCaption();
      return captionsEnabled;
    },
    layout(rect: DOMRect, shown: boolean) { anchor = rect; anchorVisible = shown; paintCaption(); },
    visible(on: boolean) { visible = on; show(); },
    context(view: Pick<ViewModel, "locked" | "approval" | "sessions" | "inbox">, chatWaiting = false) {
      // A new card wins immediately, including one arriving halfway through a recording.
      blocked = chatWaiting || !!view.locked || !!view.approval || view.sessions.some(s => s.card || s.waiting)
        || !!view.inbox?.some(item => item.source.kind === "chat" && item.source.count > 0);
      show();
    },
    update(status: DictationStatus, initial = false) {
      if (current && status.sequence <= current.sequence) return;
      // Snapshots and meter frames must never replay a recording's start cue.
      if (!initial && status.generation !== current?.generation && status.phase === "listening" && soundEnabled) onStart?.();
      const meterOnly = current && current.phase === status.phase && current.partial === status.partial
        && current.text === status.text && current.app === status.app && current.terminal === status.terminal
        && current.session_name === status.session_name && current.error === status.error
        && current.caption_until === status.caption_until && current.words === status.words;
      current = status;
      clearTimeout(timer);
      clearTimeout(captionTimer);
      captionDeadline = ended.has(status.phase) ? Math.min(Date.now() + 2000, status.caption_until ?? 0) : null;
      captionExpired = captionDeadline !== null && captionDeadline <= Date.now();
      if (captionDeadline !== null && !captionExpired) captionTimer = setTimeout(() => {
        captionExpired = true;
        paintCaption();
      }, Math.max(0, captionDeadline - Date.now()));
      if (!meterOnly) relabel();
      expired = false;
      show();
      if (!meterOnly) paintCaption();
      if (ended.has(status.phase)) {
        timer = setTimeout(() => { expired = true; show(); }, status.phase === "failed" ? 8000 : 3000);
      }
    },
  };
  Bridge.onDictation(status => view.update(status));
  window.addEventListener("resize", paintCaption);
  void Bridge.dictationStatus().then(status => { if (status) view.update(status, true); }, () => {});
  return view;
}
