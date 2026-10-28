import type { CommandPreview, SessionView } from "./bridge";
import { el } from "./dom";
import { t, tk } from "./i18n";

const ACTIONS: Record<string, string> = {
  pause: tk("Pause playback"), resume: tk("Resume playback"), "next-track": tk("Next track"), "previous-track": tk("Previous track"),
  "player-volume": tk("Set player volume"), "player-quieter": tk("Lower player volume"), "player-louder": tk("Raise player volume"),
  "computer-mute": tk("Mute computer audio"), "computer-unmute": tk("Unmute computer audio"), "computer-volume": tk("Set computer volume"),
  "computer-quieter": tk("Lower computer volume"), "computer-louder": tk("Raise computer volume"),
  "next-session": tk("Next session"), "previous-session": tk("Previous session"), focus: tk("Focus session"), open: tk("Open target"),
  application: tk("Open application"), terminal: tk("Open terminal"), folder: tk("Open folder"),
  "notifications-mute": tk("Mute project notifications"), "notifications-unmute": tk("Unmute project notifications"),
  pin: tk("Pin project"), unpin: tk("Unpin project"), hide: tk("Hide project"), unhide: tk("Show project"),
  waiting: tk("List waiting sessions"), session: tk("Read session status"), usage: tk("Read plan usage"), "pull-requests": tk("Read pull requests"),
  repeat: tk("Repeat flock action"), undo: tk("Undo flock action"),
};
const STATUS: Record<string, string> = {
  refused: tk("Permission answers cannot be checked as executable voice actions."),
  volume: tk("The requested volume is invalid."), quoted: tk("Quoted requests stay local."), negated: tk("Negated requests stay local."),
  scope: tk("Say whether you mean player audio or computer audio."), "media-target": tk("Named players and scheduled playback commands are unsupported."),
  unknown: tk("No supported command was recognized."), locked: tk("Unlock the desktop before checking command targets."),
  recognized: tk("Recognized. Availability has not been tested."), missing: tk("No matching target is available in this snapshot."),
  ambiguous: tk("More than one target matches. A real command would need clarification."),
  "apps-off": tk("Application voice commands are off or unavailable. Enable them in Command settings to match installed apps."),
  context: tk("Repeat and undo need live voice-command context. This check does not read or change it."),
};

export function createCommandCheck(check: (text: string) => Promise<CommandPreview>) {
  const element = el("details", { class: "voice-command-check" });
  element.dataset.setting = "command-check";
  const title = el("summary"); title.dataset.setting = "command-check-summary";
  const about = el("p");
  const input = el("textarea"); input.rows = 2; input.maxLength = 4096;
  input.dataset.setting = "command-check-phrase";
  const submit = el("button", { onclick: () => void run() }); submit.dataset.setting = "command-check-submit";
  const cancel = el("button", { onclick: () => {
    if (!active || !pending) return;
    epoch++; pending = false; failed = false; changed = null; cancelled = true; preview = null; paint();
  } });
  cancel.dataset.setting = "command-check-cancel";
  const feedback = el("p"); feedback.setAttribute("role", "status");
  feedback.id = "command-check-feedback";
  input.setAttribute("aria-describedby", feedback.id);
  const results = el("ol");
  let active = false, pending = false, failed = false, cancelled = false, epoch = 0;
  let changed: "setup" | "targets" | null = null;
  let targetSnapshot: string | undefined;
  let preview: CommandPreview | null = null;
  const inputError = () => {
    // Match the native control/whitespace boundary so rejected text never starts a query.
    if (/[\u0000-\u0008\u000e-\u001f\u007f-\u0084\u0086-\u009f]/u.test(input.value)) return "control";
    return new TextEncoder().encode(input.value).length > 4096 ? "length" : null;
  };
  const valid = () => input.value.trim().length > 0 && inputError() === null;
  const paint = () => {
    const cancelFocused = document.activeElement === cancel;
    const error = inputError();
    input.setAttribute("aria-invalid", String(error !== null));
    title.textContent = t("Check a command");
    about.textContent = t("Type a phrase to inspect its local interpretation. Nothing is recorded, executed, saved to history or sent to Chat.");
    input.setAttribute("aria-label", t("Command phrase"));
    submit.textContent = pending ? t("Checking command…") : t("Check phrase");
    submit.disabled = pending || !valid();
    cancel.textContent = t("Cancel phrase check");
    cancel.hidden = !pending;
    results.setAttribute("aria-busy", String(pending));
    if (cancelFocused && !pending && active) submit.focus({ preventScroll: true });
    feedback.textContent = error === "control" ? t("Remove control characters before checking this phrase.") : error === "length" ? t("This phrase is too long. Shorten it before checking.") : cancelled ? t("Check cancelled. You can check again.") : changed === "targets" ? t("Command targets changed. Check the phrase again.") : changed ? t("Command setup changed. Check the phrase again.") : failed ? t("Could not check this command. Try again.") : preview
      ? preview.status === "parsed" ? t("Interpretation only. Nothing was executed or sent to Chat.") : t(STATUS[preview.status] ?? STATUS.unknown!) : "";
    results.replaceChildren(...(preview?.actions ?? []).map(action => {
      const item = el("li", {}, el("strong", { text: t(ACTIONS[action.action] ?? STATUS.unknown!) }),
        ...(action.value === null ? [] : [el("span", { text: t("{n}%", { n: action.value }) })]),
        ...(action.target ? [el("p", { text: action.target })] : action.action.startsWith("computer-")
          ? [el("p", { text: t("Target: default computer output.") })]
          : ["pause", "resume", "next-track", "previous-track", "player-volume", "player-quieter", "player-louder"].includes(action.action)
            ? [el("p", { text: t("Target: selected media player.") })] : []),
        el("p", { text: t(STATUS[action.status] ?? STATUS.unknown!) }));
      if (action.candidates.length) item.append(el("ul", {}, ...action.candidates.map(name => el("li", { text: name }))));
      return item;
    }));
  };
  const edit = () => { epoch++; pending = false; failed = false; changed = null; cancelled = false; preview = null; paint(); };
  input.addEventListener("input", edit);
  async function run() {
    if (!active || pending || !valid()) return;
    const generation = ++epoch;
    pending = true; failed = false; changed = null; cancelled = false; preview = null; paint();
    try {
      const result = await check(input.value);
      if (generation === epoch && active) preview = result;
    } catch {
      if (generation === epoch && active) failed = true;
    } finally {
      if (generation === epoch && active) { pending = false; paint(); }
    }
  }
  element.append(title, about, input, el("div", { class: "voice-command-actions" }, submit, cancel), feedback, results);
  paint();
  const invalidate = (reason: "setup" | "targets" = "setup") => {
    const hadReading = pending || preview !== null;
    epoch++; pending = false; failed = false; cancelled = false; preview = null;
    if (hadReading) changed = reason;
    paint();
  };
  return { element, relabel: paint, invalidate,
    fill(phrase: string) { input.value = phrase; edit(); element.open = true; input.focus(); },
    targets(sessions: SessionView[]) {
      // Activity and list order change often; only the resolver's names/identities invalidate results.
      const snapshot = sessions.map(session => JSON.stringify([session.agent, session.id, session.project, session.cwd, session.agent_name ?? null])).sort().join("\n");
      if (snapshot !== targetSnapshot) { targetSnapshot = snapshot; invalidate("targets"); }
    }, visible(on: boolean) {
    if (on === active) return;
    active = on;
    if (!on) { epoch++; pending = false; failed = false; changed = null; cancelled = false; preview = null; paint(); }
  } };
}
