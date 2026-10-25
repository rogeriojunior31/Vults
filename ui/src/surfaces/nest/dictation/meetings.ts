import { createMeetingSummary, type MeetingSummaryBackend } from "./summary";
import { createMeetingArchive, type MeetingArchiveBackend } from "./archive";
import type { MeetingStatus, MeetingTranscript, MeetingTrack } from "../../../bridge";
import { el } from "../../../dom";
import { t, tk } from "../../../i18n";
import { icon } from "../../../island/icons";

export interface MeetingBackend {
  archive?: MeetingArchiveBackend;
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

function createRecording(backend: MeetingBackend | undefined, configure: () => void, summarySettings: () => void, openSaved: (id: string) => void) {
  const notes = createMeetingSummary(backend?.summary, summarySettings);
  let selected: "transcript" | "summary" | "todos" = "transcript";
  const heading = el("span"), about = el("p"), disclosure = el("p");
  const phase = el("p");
  const timer = el("output");
  const line = el("div", { class: "meeting-recording-line" });
  const sources = el("dl", { class: "meeting-sources" });
  const fallback = el("p"), model = el("p"), progress = el("p");
  const transcriptTitle = el("summary"), transcriptNote = el("p"), transcriptRows = el("div", { class: "meeting-transcript-rows" });
  const microphone = el("button"), system = el("button");
  const transcriptSources = el("div", { class: "meeting-transcript-sources" }, microphone, system);
  const finishedNote = el("p");
  const transcriptDetails = el("details", {}, transcriptTitle, finishedNote, transcriptNote);
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
  const saveTitle = el("input"); saveTitle.type = "text"; saveTitle.maxLength = 512;
  const saveLabel = el("span"), save = el("button"), viewSaved = el("button"), saveNote = el("p"), saveError = el("p", { class: "meeting-feedback" });
  const saveStatus = el("p"), saveDetailsTitle = el("summary"), saveDetails = el("details", { class: "meeting-recording-options" }, saveDetailsTitle, saveNote);
  saveError.setAttribute("role", "alert"); saveStatus.setAttribute("role", "status");
  const savingControls = el("div", { class: "meeting-actions meeting-save" }, el("label", {}, saveLabel, saveTitle), save, viewSaved);
  const saveRegion = el("div", { class: "meeting-save-region" }, savingControls);
  const recordingOptionsTitle = el("summary"), recordingOptions = el("details", { class: "meeting-recording-options" }, recordingOptionsTitle);
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
  const body = el("div", { class: "meeting-body" }, feedback, saveError, saveStatus, transcriptPage, summaryPage, todosPage, saveDetails, recordingOptions);
  const stateRegion = el("div", { class: "meeting-state" }, line, phase, timer, progress);
  const footer = el("div", { class: "meeting-footer" }, saveRegion, notes.actions);
  const element = el("section", { class: "meeting-check" }, el("header", { class: "dictation-heading" }, el("h2", { class: "dictation-title" }, icon("mic", 18), heading)), stateRegion, tabList, body, footer, actions);
  body.tabIndex = 0;
  phase.setAttribute("role", "status"); phase.setAttribute("aria-live", "polite");
  feedback.setAttribute("role", "alert"); timer.setAttribute("aria-live", "off"); line.setAttribute("aria-hidden", "true");
  let sourceKey = "", transcriptKey = "";
  let transcript: MeetingTranscript | null = null;
  let track: "microphone" | "system" = "microphone";
  let transcriptFailed = false, transcriptReading = false;
  const clock = (ms: number) => {
    const hours = Math.floor(ms / 3600000);
    const minutes = Math.floor(ms / 60000 % 60).toString().padStart(2, "0");
    const seconds = Math.floor(ms / 1000 % 60).toString().padStart(2, "0");
    return hours ? `${hours}:${minutes}:${seconds}` : `${minutes}:${seconds}`;
  };
  const range = (start: number, end: number) => {
    if (clock(start) !== clock(end)) return `${clock(start)}–${clock(end)}`;
    const digits = Math.floor(start / 10) === Math.floor(end / 10) ? 3 : 2;
    const at = (ms: number) => `${clock(ms)}.${Math.floor(ms % 1000 / (digits === 2 ? 10 : 1)).toString().padStart(digits, "0")}`;
    return `${at(start)}–${at(end)}`;
  };
  let value: MeetingStatus | null = null;
  let active = false, busy = false, reading = false, failed = false, actionFailed = false, generation = 0;
  let interval = 0;
  let saving = false, savedId: string | null = null, saveFailed = false;
  const visible = () => active && !document.hidden;
  const draw = () => {
    element.classList.toggle("meeting-before-transcript", selected === "transcript" && !transcript);
    heading.textContent = t("Meetings");
    tabList.setAttribute("aria-label", t("Meeting content"));
    tabs.transcript.textContent = t("Transcript"); tabs.summary.textContent = t("Summary"); tabs.todos.textContent = t("To-dos");
    for (const [key, button] of Object.entries(tabs)) {
      button.setAttribute("aria-selected", String(key === selected)); button.tabIndex = key === selected ? 0 : -1;
      panels[key as keyof typeof panels].hidden = key !== selected;
    }
    if (selected !== "transcript" && notes.element.parentElement !== panels[selected]) panels[selected].append(notes.element);
    body.setAttribute("role", "region"); body.setAttribute("aria-label", selected === "transcript" ? t("Recording details") : t("Meeting content"));
    about.textContent = t("Record up to four hours. Both audio sources are transcribed locally after you stop.");
    disclosure.textContent = t("Tell everyone before recording. Temporary audio uses up to 1 GiB and is deleted after transcription or cancellation. Unsaved text and notes are discarded when you leave or review another recording. Summaries require separate consent. Save meeting keeps a text-only snapshot on this computer.");
    const label = value?.phase === "transcribing" && value.progress?.stage === "speakers" ? t("Identifying voices locally; dictation has priority…") : value ? t(phases[value.phase]) : t("Reading recording status…");
    if (phase.textContent !== label) phase.textContent = label;
    timer.textContent = clock(value?.elapsedMs ?? 0);
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
    identifyNote.textContent = t("Voice labels may be wrong and are limited to 30 minutes per source. Longer recordings keep their transcript without voice labels.");
    review.textContent = t("Review audio sources");
    begin.textContent = t("Record and transcribe");
    stop.textContent = t("Stop recording"); cancel.textContent = value?.phase === "transcribing" ? t("Cancel transcription") : value?.hasTranscript ? t("Discard transcript") : t("Cancel recording");
    recover.textContent = t("Retry audio cleanup"); retry.textContent = t("Try again");
    const running = pending(value?.phase);
    notes.actions.hidden = selected === "transcript" || running;
    stateRegion.hidden = !running && (selected !== "transcript" || !!transcript);
    actions.hidden = selected !== "transcript" && !running;
    recordingOptionsTitle.textContent = t("Recording actions");
    recordingOptions.hidden = !transcript || selected !== "transcript";
    if (transcript && actions.parentElement !== recordingOptions) recordingOptions.append(actions);
    else if (!transcript && actions.parentElement !== element) element.append(actions);
    const unavailable = !backend || !value || value.phase === "unsupported";
    review.hidden = running || value?.error === "recovery" || unavailable;
    review.disabled = busy || saving; review.classList.toggle("meeting-primary", !transcript && selected === "transcript" && value?.phase !== "ready");
    begin.hidden = value?.phase !== "ready"; begin.disabled = busy || !checkbox.checked;
    begin.classList.add("meeting-primary"); stop.classList.add("meeting-primary");
    stop.hidden = value?.phase !== "recording"; stop.disabled = busy;
    cancel.hidden = !["preparing", "starting", "recording", "stopping", "transcribing", "cancelling", "finished"].includes(value?.phase ?? "");
    cancel.disabled = busy || saving || value?.phase === "cancelling";
    recover.hidden = value?.error !== "recovery" || running; recover.disabled = busy;
    voice.textContent = t("Open Voice settings"); voice.hidden = running;
    retry.hidden = !failed && !transcriptFailed; retry.disabled = busy || reading;
    const message = transcriptFailed ? t("Could not read the transcript. Try again while staying on this page.") : actionFailed ? t("Could not complete the recording action. Review the audio sources again before starting.") : failed ? t("Could not update recording status. The last view is kept; recording cancels automatically if this page cannot stay connected.")
      : value?.error ? t(errors[value.error]) : "";
    if (feedback.textContent !== message) feedback.textContent = message; feedback.hidden = !message;
    transcriptPanel.hidden = !transcript;
    transcriptTitle.textContent = t("About this transcript");
    finishedNote.textContent = t(phases.finished);
    transcriptPanel.setAttribute("aria-label", t("Transcript"));
    const attribution = transcript?.[track].attribution;
    const attributed = attribution === "analyzed";
    transcriptNote.textContent = attributed ? t("Times are estimated by the local models. Voice numbers belong only to this source and recording; they do not identify people.") : t("Each source has its own recording clock. These are estimated segment times, without speaker identification.");
    attributionNote.hidden = attribution !== "unavailable";
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
    saveRegion.hidden = !backend?.archive || !transcript || value?.phase !== "finished";
    saveDetails.hidden = saveRegion.hidden; saveError.hidden = saveRegion.hidden || !saveFailed;
    saveStatus.hidden = saveRegion.hidden || !savedId; saveDetailsTitle.textContent = t("Saving details");
    saveLabel.textContent = t("Meeting title"); saveTitle.placeholder = t("Meeting title"); saveTitle.setAttribute("aria-label", t("Meeting title")); saveTitle.disabled = saving || !!savedId;
    save.textContent = saving ? t("Saving meeting…") : t("Save meeting"); save.classList.toggle("meeting-primary", selected === "transcript");
    save.disabled = saving || busy || !!savedId || !saveTitle.value.trim();
    viewSaved.textContent = t("View saved meeting"); viewSaved.hidden = !savedId;
    saveStatus.textContent = t("Saved. Further changes on this recording do not update the saved copy.");
    saveNote.textContent = t("Save freezes the current transcript and completed notes. Audio is never kept. Clear all history also removes saved meetings.");
    saveError.textContent = t("Could not save this meeting. Finish any summary, check the title and try again while staying on this page.");
    footer.hidden = saveRegion.hidden && notes.actions.hidden;
    element.setAttribute("aria-busy", String(busy || saving));
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
    if (next.id !== value?.id || !next.hasTranscript) { transcript = null; transcriptKey = ""; transcriptFailed = false; savedId = null; saveTitle.value = ""; saveFailed = false; }
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
    if (!visible() || busy || saving) return;
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
  saveTitle.addEventListener("input", draw);
  save.addEventListener("click", async () => {
    if (!visible() || !backend?.archive || !transcript || !value || save.disabled) return;
    const current = generation, id = value.id; saving = true; saveFailed = false; draw();
    try {
      const saved = await backend.archive.save(id, saveTitle.value.trim());
      if (current === generation && visible() && value?.id === id) savedId = saved;
    } catch { if (current === generation && visible()) saveFailed = true; }
    finally { saving = false; if (current === generation && visible()) draw(); }
  });
  viewSaved.addEventListener("click", () => { if (savedId && !saving) openSaved(savedId); });
  const suspend = () => {
    notes.setActive(false); notes.meeting(null, false);
    generation++; checkbox.checked = false; identify.checked = false; transcript = null; transcriptKey = ""; transcriptFailed = false;
    savedId = null; saveTitle.value = ""; saveFailed = false; draw();
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
  if (!backend) value = { id: 0, phase: "unsupported", elapsedMs: 0, maxSeconds: 14400, microphone: null, monitor: null, fallback: false, error: null, modelId: null, language: null, hasTranscript: false };
  draw();
  return { element, relabel() { draw(); notes.relabel(); }, refresh() { notes.refresh(); }, setActive(on: boolean) { if (active === on) return; active = on; follow(); } };
}

export function createMeetings(backend?: MeetingBackend, configure: () => void = () => {}, summarySettings: () => void = configure) {
  const archive = backend?.archive ? createMeetingArchive(backend.archive) : null;
  let active = false, saved = false;
  const recording = createRecording(backend, configure, summarySettings, id => { choose(true); archive?.open(id); });
  if (!archive) return { ...recording, invalidate() {} };
  const current = el("button"), retained = el("button");
  const navigation = el("nav", { class: "meeting-workspace-navigation" }, current, retained);
  const element = el("div", { class: "meetings-workspace" }, navigation, recording.element, archive.element);
  function draw() {
    navigation.setAttribute("aria-label", t("Meetings")); current.textContent = t("New recording"); retained.textContent = t("Saved meetings");
    current.setAttribute("aria-pressed", String(!saved)); retained.setAttribute("aria-pressed", String(saved));
    recording.element.hidden = saved; archive!.element.hidden = !saved;
  }
  function choose(next: boolean) {
    saved = next; recording.setActive(active && !saved); archive!.setActive(active && saved); draw();
  }
  current.addEventListener("click", () => choose(false)); retained.addEventListener("click", () => choose(true));
  document.addEventListener("visibilitychange", () => archive.setActive(active && saved && !document.hidden));
  window.addEventListener("pagehide", () => archive.setActive(false));
  draw();
  return { element, relabel() { draw(); recording.relabel(); archive.relabel(); },
    refresh() { if (saved) void archive.refresh(); else recording.refresh(); },
    invalidate() { archive.invalidate(); recording.setActive(false); recording.setActive(active && !saved); if (active && saved) void archive.refresh(); },
    setActive(on: boolean) { active = on; recording.setActive(on && !saved); archive.setActive(on && saved && !document.hidden); },
  };
}
