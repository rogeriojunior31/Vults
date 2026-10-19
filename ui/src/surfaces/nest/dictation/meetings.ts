import { createMeetingSummary, type MeetingSummaryBackend } from "./summary";
import type { MeetingStatus, MeetingTranscript, MeetingTrack } from "../../../bridge";
import { el } from "../../../dom";
import { t, tk } from "../../../i18n";

export interface MeetingBackend {
  summary?: MeetingSummaryBackend;
  transcript(id: number): Promise<MeetingTranscript | null>;
  status(id: number | null): Promise<MeetingStatus>;
  prepare(): Promise<MeetingStatus>;
  begin(id: number, consent: boolean, identifySpeakers?: boolean): Promise<MeetingStatus>;
  end(id: number, cancel: boolean): Promise<MeetingStatus>;
  recover(): Promise<MeetingStatus>;
}
const phases: Record<MeetingStatus["phase"], string> = {
  idle: tk("Ready to record and transcribe"), recovering: tk("Cleaning interrupted recording files…"),
  preparing: tk("Reading audio sources…"), ready: tk("Review the audio sources before starting"),
  starting: tk("Starting recording…"), recording: tk("Recording microphone and system audio"),
  stopping: tk("Stopping recording before local transcription…"), cancelling: tk("Cancelling and deleting temporary audio…"),
  transcribing: tk("Transcribing locally; dictation has priority…"),
  finished: tk("Transcript ready. Temporary audio was deleted."),
  cancelled: tk("Recording check cancelled. Temporary audio was deleted."),
  failed: tk("Recording check needs attention"), unsupported: tk("Meeting recording checks are available on Linux only."),
};
const errors = {
  recovery: tk("Could not clean interrupted recording files. Retry cleanup before recording."),
  sources: tk("Could not read a microphone and system-audio monitor. Check your audio devices and try again."),
  model: tk("Choose an installed local model in Dictation > Voice before recording."),
  transcription: tk("Local transcription failed. Temporary audio cleanup was attempted; review the sources before recording again."),
  capture: tk("Could not complete the recording check. Review the sources again; audio cleanup will be retried before the next recording."),
};
const pending = (phase?: string) => ["recovering", "preparing", "starting", "recording", "stopping", "transcribing", "cancelling"].includes(phase ?? "");

function readableSegments(track: MeetingTrack | undefined) {
  if (!track?.attribution) return track?.segments ?? [];
  const rows: MeetingTrack["segments"] = [];
  for (const word of track.segments) {
    const last = rows.at(-1);
    if (last && last.speaker === word.speaker && word.startMs - last.endMs <= 1400 && word.endMs - last.startMs <= 15000) {
      last.text += ` ${word.text}`; last.endMs = word.endMs;
    } else rows.push({ ...word });
  }
  return rows;
}

export function createMeetings(backend?: MeetingBackend, configure: () => void = () => {}, summarySettings: () => void = configure) {
  const notes = createMeetingSummary(backend?.summary, summarySettings);
  let selected: "transcript" | "summary" | "todos" = "transcript";
  const heading = el("h2"), about = el("p"), disclosure = el("p");
  const phase = el("p");
  const timer = el("output");
  const line = el("div", { class: "meeting-recording-line" });
  const sources = el("dl", { class: "meeting-sources" });
  const fallback = el("p"), model = el("p"), progress = el("p");
  const transcriptTitle = el("summary"), transcriptNote = el("p"), transcriptRows = el("div", { class: "meeting-transcript-rows" });
  const microphone = el("button"), system = el("button");
  const transcriptSources = el("div", { class: "meeting-transcript-sources" }, microphone, system);
  const transcriptDetails = el("details", {}, transcriptTitle, transcriptNote);
  const transcriptPanel = el("section", { class: "meeting-transcript" }, transcriptSources, transcriptRows, transcriptDetails);
  const checkbox = el("input"); checkbox.type = "checkbox";
  const consentText = el("span");
  const consent = el("label", { class: "meeting-consent" }, checkbox, consentText);
  const identify = el("input"); identify.type = "checkbox";
  const identifyText = el("span"), identifyNote = el("p");
  const identification = el("div", {}, el("label", { class: "meeting-consent" }, identify, identifyText), identifyNote);
  const attributionNote = el("p");
  transcriptPanel.append(attributionNote);
  const feedback = el("p", { class: "meeting-feedback" });
  const review = el("button"), begin = el("button"), stop = el("button"), cancel = el("button"), recover = el("button"), retry = el("button"), voice = el("button");
  const actions = el("div", { class: "meeting-actions" }, review, begin, stop, cancel, recover, retry, voice);
  const transcriptPage = el("div", {}, about, disclosure, sources, model, fallback, consent, identification, transcriptPanel);
  const summaryPage = el("div"), todosPage = el("div");
  const panels = { transcript: transcriptPage, summary: summaryPage, todos: todosPage };
  for (const [key, panel] of Object.entries(panels)) panel.id = `meeting-panel-${key}`;
  const tabs = { transcript: el("button"), summary: el("button"), todos: el("button") };
  const tabList = el("div", { class: "meeting-tabs" }, ...Object.values(tabs)); tabList.setAttribute("role", "tablist");
  const select = (next: typeof selected) => { selected = next; if (next !== "transcript") notes.select(next); draw(); };
  for (const [key, button] of Object.entries(tabs) as [typeof selected, HTMLButtonElement][]) {
    button.id = `meeting-tab-${key}`; button.setAttribute("role", "tab"); button.setAttribute("aria-controls", panels[key].id);
    panels[key].setAttribute("role", "tabpanel"); panels[key].setAttribute("aria-labelledby", button.id);
    button.addEventListener("click", () => select(key));
    button.addEventListener("keydown", event => {
      if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
      event.preventDefault();
      const keys = Object.keys(tabs) as (typeof selected)[], index = keys.indexOf(key);
      const next = event.key === "Home" ? keys[0] : event.key === "End" ? keys.at(-1)! : keys[(index + (event.key === "ArrowRight" ? 1 : -1) + keys.length) % keys.length];
      select(next); tabs[next].focus();
    });
  }
  const body = el("div", { class: "meeting-body" }, feedback, transcriptPage, summaryPage, todosPage);
  const stateRegion = el("div", { class: "meeting-state" }, line, phase, timer, progress);
  const element = el("section", { class: "meeting-check" }, el("header", {}, heading), stateRegion, tabList, body, notes.actions, actions);
  body.tabIndex = 0;
  phase.setAttribute("role", "status"); phase.setAttribute("aria-live", "polite");
  feedback.setAttribute("role", "alert"); timer.setAttribute("aria-live", "off"); line.setAttribute("aria-hidden", "true");
  let sourceKey = "", transcriptKey = "";
  let transcript: MeetingTranscript | null = null;
  let track: "microphone" | "system" = "microphone";
  let transcriptFailed = false, transcriptReading = false;
  const clock = (ms: number) => `${Math.floor(ms / 60000).toString().padStart(2, "0")}:${Math.floor(ms / 1000 % 60).toString().padStart(2, "0")}`;
  const range = (start: number, end: number) => {
    if (clock(start) !== clock(end)) return `${clock(start)}–${clock(end)}`;
    const digits = Math.floor(start / 10) === Math.floor(end / 10) ? 3 : 2;
    const at = (ms: number) => `${clock(ms)}.${Math.floor(ms % 1000 / (digits === 2 ? 10 : 1)).toString().padStart(digits, "0")}`;
    return `${at(start)}–${at(end)}`;
  };
  let value: MeetingStatus | null = null;
  let active = false, busy = false, reading = false, failed = false, actionFailed = false, generation = 0;
  let interval = 0;
  const visible = () => active && !document.hidden;
  const draw = () => {
    heading.textContent = t("Meetings");
    tabList.setAttribute("aria-label", t("Meeting content"));
    tabs.transcript.textContent = t("Transcript"); tabs.summary.textContent = t("Summary"); tabs.todos.textContent = t("To-dos");
    for (const [key, button] of Object.entries(tabs)) {
      button.setAttribute("aria-selected", String(key === selected)); button.tabIndex = key === selected ? 0 : -1;
      panels[key as keyof typeof panels].hidden = key !== selected;
    }
    if (selected !== "transcript" && notes.element.parentElement !== panels[selected]) panels[selected].append(notes.element);
    body.setAttribute("role", "region"); body.setAttribute("aria-label", selected === "transcript" ? t("Recording details") : t("Meeting content"));
    about.textContent = t("Record up to one minute and transcribe both audio sources locally.");
    disclosure.textContent = t("Tell everyone before recording. Temporary audio is deleted after transcription or cancellation. The transcript and optional notes stay only on this page and are discarded when you leave, close the window or review a new recording. Summaries require separate consent. Saved meetings and export are not available yet.");
    const label = value?.phase === "transcribing" && value.progress?.stage === "speakers" ? t("Identifying voices locally; dictation has priority…") : value ? t(phases[value.phase]) : t("Reading recording status…");
    if (phase.textContent !== label) phase.textContent = label;
    timer.textContent = `${Math.floor((value?.elapsedMs ?? 0) / 60000).toString().padStart(2, "0")}:${Math.floor((value?.elapsedMs ?? 0) / 1000 % 60).toString().padStart(2, "0")}`;
    timer.setAttribute("aria-label", t("Recording time"));
    line.hidden = !["starting", "recording", "stopping", "transcribing", "cancelling"].includes(value?.phase ?? "");
    const key = JSON.stringify([value?.microphone, value?.monitor, t("Microphone"), t("System audio")]);
    if (key !== sourceKey) {
      sourceKey = key; sources.replaceChildren();
      if (value?.microphone && value.monitor) {
      sources.append(el("dt", { text: t("Microphone") }), el("dd", { text: value.microphone }),
        el("dt", { text: t("System audio") }), el("dd", { text: value.monitor }));
    }
    }
    sources.hidden = !value?.microphone || !!transcript;
    about.hidden = !!transcript; disclosure.hidden = !!transcript;
    fallback.hidden = !value?.fallback || !!transcript;
    fallback.textContent = t("The default output monitor was unavailable. Review the fallback monitor shown above.");
    model.hidden = !value?.modelId || !!transcript;
    model.textContent = t("Local model: {model}. Spoken language: {language}.", { model: value?.modelId ?? "", language: value?.language ?? t("Auto-detect language") });
    progress.hidden = value?.phase !== "transcribing";
    progress.textContent = value?.progress ? t("{source}: {completed} of {total} processed", {
      source: value.progress.track === "microphone" ? t("Microphone") : t("System audio"),
      completed: clock(value.progress.completedMs), total: clock(value.progress.totalMs),
    }) : t("Preparing the local model; no audio is being recorded.");
    consentText.textContent = t("I have informed the participants and agree to temporary audio capture and local transcription of these sources.");
    consent.hidden = value?.phase !== "ready";
    checkbox.disabled = busy || value?.phase !== "ready";
    identification.hidden = !value?.identificationAvailable || value.phase !== "ready";
    identify.disabled = busy || identification.hidden;
    identifyText.textContent = t("Also identify voices for this recording (experimental)");
    identifyNote.textContent = t("Voice labels may be wrong. Uncertain words stay unknown. Each audio source has separate labels.");
    review.textContent = t("Review audio sources");
    begin.textContent = t("Record and transcribe");
    stop.textContent = t("Stop recording"); cancel.textContent = value?.phase === "transcribing" ? t("Cancel transcription") : value?.hasTranscript ? t("Discard transcript") : t("Cancel recording");
    recover.textContent = t("Retry audio cleanup"); retry.textContent = t("Try again");
    const running = pending(value?.phase);
    notes.actions.hidden = selected === "transcript" || running;
    stateRegion.hidden = selected !== "transcript" && !running;
    actions.hidden = selected !== "transcript" && !running;
    const unavailable = !backend || !value || value.phase === "unsupported";
    review.hidden = running || value?.error === "recovery" || unavailable;
    review.disabled = busy; review.classList.toggle("meeting-primary", selected === "transcript" && value?.phase !== "ready");
    begin.hidden = value?.phase !== "ready"; begin.disabled = busy || !checkbox.checked;
    begin.classList.add("meeting-primary"); stop.classList.add("meeting-primary");
    stop.hidden = value?.phase !== "recording"; stop.disabled = busy;
    cancel.hidden = !["preparing", "starting", "recording", "stopping", "transcribing", "cancelling", "finished"].includes(value?.phase ?? "");
    cancel.disabled = busy || value?.phase === "cancelling";
    recover.hidden = value?.error !== "recovery" || running; recover.disabled = busy;
    voice.textContent = t("Open Voice settings"); voice.hidden = running;
    retry.hidden = !failed && !transcriptFailed; retry.disabled = busy || reading;
    const message = transcriptFailed ? t("Could not read the transcript. Try again while staying on this page.") : actionFailed ? t("Could not complete the recording action. Review the audio sources again before starting.") : failed ? t("Could not update recording status. The last view is kept; recording cancels automatically if this page cannot stay connected.")
      : value?.error ? t(errors[value.error]) : "";
    if (feedback.textContent !== message) feedback.textContent = message; feedback.hidden = !message;
    transcriptPanel.hidden = !transcript;
    transcriptTitle.textContent = t("About this transcript");
    transcriptPanel.setAttribute("aria-label", t("Transcript"));
    const attributed = transcript?.[track].attribution;
    transcriptNote.textContent = attributed ? t("Times are estimated by the local models. Voice numbers belong only to this source and recording; they do not identify people.") : t("Each source has its own recording clock. These are estimated segment times, without speaker identification.");
    attributionNote.hidden = attributed !== "unavailable";
    attributionNote.textContent = t("Voice identification was unavailable. The transcript is kept without labels.");
    microphone.textContent = t("Microphone"); system.textContent = t("System audio");
    microphone.setAttribute("aria-pressed", String(track === "microphone")); system.setAttribute("aria-pressed", String(track === "system"));
    const nextKey = JSON.stringify([transcript?.id, track, attributed, t("Unknown voice"), t("No speech was detected in this source.")]);
    if (nextKey !== transcriptKey) {
      transcriptKey = nextKey; transcriptRows.replaceChildren();
      const segments = readableSegments(transcript?.[track]);
      if (!segments.length) transcriptRows.append(el("p", { text: t("No speech was detected in this source.") }));
      for (const segment of segments) {
        const content = el("p");
        if (attributed) content.append(el("strong", { text: segment.speaker == null ? t("Unknown voice") : t("Voice {number}", { number: segment.speaker + 1 }) }), document.createTextNode(" "));
        content.append(document.createTextNode(segment.text));
        transcriptRows.append(el("div", { class: "meeting-segment" },
          el("span", { class: "meeting-segment-time", text: range(segment.startMs, segment.endMs) }), content));
      }
    }
    body.classList.toggle("meeting-with-transcript", !!transcript || selected !== "transcript");
    element.setAttribute("aria-busy", String(busy));
    const focused = document.activeElement;
    if ([begin, stop, cancel].includes(focused as HTMLButtonElement) && (focused as HTMLElement).hidden) {
      const next = !cancel.hidden && !cancel.disabled ? cancel : !review.hidden ? review : retry;
      if (!next.hidden) next.focus({ preventScroll: true });
    }
  };
  const accept = (next: MeetingStatus) => {
    if (next.phase === "failed" && next.error !== value?.error) body.scrollTop = 0;
    if (next.id !== value?.id || next.phase !== value?.phase) actionFailed = false;
    if (next.id !== value?.id || next.phase !== "ready") { checkbox.checked = false; identify.checked = false; }
    if (next.id !== value?.id || !next.hasTranscript) { transcript = null; transcriptKey = ""; transcriptFailed = false; }
    value = next; failed = false; notes.meeting(next.id, !!transcript); draw();
    if (next.hasTranscript && !transcript && !transcriptFailed) void loadTranscript();
  };
  const loadTranscript = async () => {
    if (!visible() || !backend || !value?.hasTranscript || transcriptReading) return;
    const current = generation, id = value.id; transcriptReading = true;
    try {
      const result = await backend.transcript(id);
      if (visible() && current === generation && value?.id === id && value.hasTranscript) {
        if (!result || result.id !== id) throw new Error("Transcript unavailable");
        transcript = result; transcriptFailed = false; transcriptKey = ""; notes.meeting(id, true); draw();
      }
    } catch { if (current === generation && visible()) { transcriptFailed = true; draw(); } }
    finally { transcriptReading = false; }
  };
  const read = async () => {
    if (!visible() || reading || busy || !backend) return;
    const current = generation; reading = true;
    try { const next = await backend.status(value?.id ?? null); if (current === generation && visible()) accept(next); }
    catch { if (current === generation) { failed = true; draw(); } }
    finally { reading = false; }
  };
  const act = async (action: () => Promise<MeetingStatus>) => {
    if (!visible() || busy) return;
    busy = true; actionFailed = false; const current = ++generation; draw();
    try {
      const next = await action();
      if (current === generation && visible()) accept(next);
      else if (backend && (pending(next.phase) || next.hasTranscript || next.phase === "ready")) void backend.end(next.id, true).catch(() => {});
    } catch { if (current === generation) { actionFailed = true; checkbox.checked = false; } }
    finally { busy = false; draw(); }
  };
  checkbox.addEventListener("change", draw);
  review.addEventListener("click", () => { select("transcript"); if (backend) void act(backend.prepare); });
  begin.addEventListener("click", () => { if (backend && value && checkbox.checked) void act(() => backend.begin(value!.id, true, identify.checked)); });
  stop.addEventListener("click", () => { if (backend && value) void act(() => backend.end(value!.id, false)); });
  cancel.addEventListener("click", () => { if (backend && value) void act(() => backend.end(value!.id, true)); });
  recover.addEventListener("click", () => { if (backend) void act(backend.recover); });
  retry.addEventListener("click", () => { if (transcriptFailed) { transcriptFailed = false; void loadTranscript(); } else void read(); });
  voice.addEventListener("click", configure);
  microphone.addEventListener("click", () => { track = "microphone"; draw(); });
  system.addEventListener("click", () => { track = "system"; draw(); });
  const suspend = () => {
    notes.setActive(false); notes.meeting(null, false);
    generation++; checkbox.checked = false; identify.checked = false; transcript = null; transcriptKey = ""; transcriptFailed = false; draw();
    if (interval) window.clearInterval(interval); interval = 0;
    if (value && (pending(value.phase) || value.hasTranscript || value.phase === "ready") && backend) void backend.end(value.id, true).catch(() => {});
  };
  const follow = () => {
    if (!visible()) { suspend(); return; }
    notes.setActive(true);
    if (!interval) interval = window.setInterval(() => void read(), 250);
    void read();
  };
  document.addEventListener("visibilitychange", follow);
  window.addEventListener("pagehide", suspend);
  if (!backend) value = { id: 0, phase: "unsupported", elapsedMs: 0, maxSeconds: 60, microphone: null, monitor: null, fallback: false, error: null, modelId: null, language: null, hasTranscript: false };
  draw();
  return { element, relabel() { draw(); notes.relabel(); }, refresh() { notes.refresh(); }, setActive(on: boolean) { if (active === on) return; active = on; follow(); } };
}
