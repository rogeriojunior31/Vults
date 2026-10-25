import type { MeetingEntry, MeetingSource, SavedMeeting } from "../../../bridge";
import { el } from "../../../dom";
import { locale, t } from "../../../i18n";
import { meetingNotes } from "./content";

export interface MeetingArchiveBackend {
  save(id: number, title: string): Promise<string>;
  list(): Promise<{ epoch: number; rows: MeetingEntry[] }>;
  read(id: string): Promise<{ epoch: number; meeting: SavedMeeting | null }>;
  remove(id: string): Promise<boolean>;
  rename(id: string, source: MeetingSource, speaker: number, name: string | null): Promise<boolean>;
  export(id: string, format: "markdown" | "srt" | "vtt", source: MeetingSource | null, title: string): Promise<boolean>;
}

export function createMeetingArchive(api: MeetingArchiveBackend) {
  const heading = el("h2"), back = el("button"), reload = el("button");
  const header = el("header", {}, back, heading, reload);
  const notice = el("p"), feedback = el("p", { class: "meeting-feedback" });
  feedback.setAttribute("role", "alert"); notice.setAttribute("role", "status");
  const search = el("input"); search.type = "search";
  const list = el("div", { class: "meeting-archive-list" });
  const title = el("h3"), date = el("p"), detailHeader = el("div", {}, title, date);
  const tabs = { transcript: el("button"), summary: el("button"), todos: el("button") };
  const tabList = el("div", { class: "meeting-tabs" }, ...Object.values(tabs)); tabList.setAttribute("role", "tablist");
  const microphone = el("button"), system = el("button");
  const sources = el("div", { class: "meeting-transcript-sources" }, microphone, system);
  const content = el("div", { class: "meeting-notes" });
  content.id = "saved-meeting-content"; content.setAttribute("role", "tabpanel");
  const speakerTitle = el("summary"), speakerNote = el("p"), speakers = el("div");
  const people = el("details", {}, speakerTitle, speakerNote, speakers);
  const detail = el("div", {}, detailHeader, tabList, sources, content, people);
  const confirmText = el("p"), confirmDelete = el("button"), keep = el("button");
  confirmDelete.className = "dictation-danger";
  const confirmation = el("div", { class: "meeting-delete-confirmation" }, confirmText, keep, confirmDelete);
  const results = el("p", { class: "meeting-archive-results" }); results.setAttribute("role", "status");
  const clearSearch = el("button", { onclick: () => { search.value = ""; draw(); search.focus(); } });
  const sidebar = el("div", { class: "meeting-archive-sidebar" }, search, results, list);
  const browser = el("div", { class: "meeting-archive-browser" }, sidebar, el("div", { class: "meeting-archive-detail" }, confirmation, detail));
  const body = el("div", { class: "meeting-body" }, feedback, notice, browser);
  body.tabIndex = 0; body.setAttribute("role", "region");
  const format = el("select"), exportSource = el("select"), exportButton = el("button"), remove = el("button");
  const formatText = el("span"), sourceText = el("span");
  const formatLabel = el("label", {}, formatText, format), sourceLabel = el("label", {}, sourceText, exportSource);
  const actions = el("div", { class: "meeting-actions meeting-export" }, formatLabel, sourceLabel, exportButton, remove);
  const element = el("section", { class: "meeting-archive" }, header, body, actions);
  for (const [value, text] of [["markdown", "Markdown"], ["srt", "SRT"], ["vtt", "VTT"]]) {
    const option = el("option", { text }); option.value = value; format.append(option);
  }
  for (const value of ["", "microphone", "system"]) { const option = el("option"); option.value = value; exportSource.append(option); }
  let active = false, generation = 0, reading = false, busy = false, failed = false, actionError = "", exported: boolean | null = null;
  let rows: MeetingEntry[] | null = null, selected: string | null = null, meeting: SavedMeeting | null = null;
  let mode: keyof typeof tabs = "transcript", source: MeetingSource = "microphone", confirming = false;
  let contentKey = "", speakerKey = "", listKey = "";
  const drafts = new Map<string, string>();
  const wide = window.matchMedia?.("(min-width: 1100px) and (min-height: 561px)");
  wide?.addEventListener("change", () => draw());
  const live = () => active && !document.hidden;
  const voice = (id: number) => t("Voice {number}", { number: id + 1 });
  const sourceName = (key: MeetingSource) => key === "microphone" ? t("Microphone") : t("System audio");
  const time = (ms: number) => `${Math.floor(ms / 3600000).toString().padStart(2, "0")}:${Math.floor(ms / 60000 % 60).toString().padStart(2, "0")}:${Math.floor(ms / 1000 % 60).toString().padStart(2, "0")}.${Math.floor(ms % 1000).toString().padStart(3, "0")}`;
  const dated = (at: number) => new Date(at).toLocaleString(locale(), { dateStyle: "medium", timeStyle: "short" });

  function draw() {
    heading.textContent = t("Saved meetings"); back.textContent = t("Back to saved meetings"); reload.textContent = t("Reload");
    back.hidden = selected === null; back.disabled = busy; reload.disabled = reading || busy;
    const split = !!wide?.matches && selected !== null;
    element.classList.toggle("meeting-archive-split", split);
    sidebar.hidden = selected !== null && !split;
    search.hidden = selected !== null && !split; search.placeholder = t("Search saved meetings"); search.setAttribute("aria-label", t("Search saved meetings"));
    body.setAttribute("aria-label", t("Saved meetings")); element.setAttribute("aria-busy", String(reading || busy));
    feedback.textContent = actionError || (failed ? t("Could not load saved meetings. The last view is kept. Reload to try again.") : "");
    feedback.hidden = !feedback.textContent;
    notice.textContent = exported === true ? t("Meeting exported. This file is a separate copy; deleting history will not remove it.")
      : exported === false ? t("Export cancelled. No file was written.")
      : reading && !meeting && !rows ? t("Reading saved meetings…")
      : selected && !meeting && !reading && !failed ? t("This saved meeting no longer exists.")
      : selected ? t("This is the saved text snapshot. Notes never execute tasks.")
      : t("Only explicitly saved text is kept here. Clear all history also removes these meetings. Search covers the newest 500 meetings.");
    list.hidden = selected !== null && !split; detail.hidden = !meeting || selected === null;
    const nextList = JSON.stringify([rows, search.value, locale(), selected, busy]);
    if (nextList !== listKey) {
      listKey = nextList; list.replaceChildren();
      const query = search.value.trim().toLocaleLowerCase(locale());
      const matches = rows?.filter(row => row.title.toLocaleLowerCase(locale()).includes(query)) ?? [];
      results.hidden = rows === null;
      results.textContent = matches.length === 1 ? t("1 entry") : t("{count} entries", { count: matches.length });
      clearSearch.textContent = t("Clear search");
      if (rows && !matches.length) {
        list.append(el("p", { text: query ? t("No saved meetings match your search.") : t("No saved meetings yet.") }));
        if (query) list.append(clearSearch);
      }
      for (const row of matches) {
        const button = el("button", { class: "meeting-archive-row", onclick: () => open(row.id) },
          el("strong", { text: row.title }), el("span", { text: dated(row.recorded_ms) }));
        button.disabled = busy; button.setAttribute("aria-pressed", String(row.id === selected));
        button.setAttribute("aria-label", t("Open {title}, recorded {date}", { title: row.title, date: dated(row.recorded_ms) }));
        list.append(button);
      }
    }
    confirmation.hidden = !confirming || !meeting;
    confirmation.setAttribute("role", "group"); confirmation.setAttribute("aria-label", t("Delete saved meeting"));
    confirmText.textContent = t("Delete this saved meeting, its transcript, notes and voice names? Exported files stay.");
    confirmDelete.textContent = t("Delete saved meeting"); keep.textContent = t("Keep meeting");
    confirmDelete.disabled = busy; keep.disabled = busy;
    title.textContent = meeting?.title ?? ""; date.textContent = meeting ? dated(meeting.recorded_ms) : "";
    tabList.setAttribute("aria-label", t("Meeting content"));
    tabs.transcript.textContent = t("Transcript"); tabs.summary.textContent = t("Summary"); tabs.todos.textContent = t("To-dos");
    for (const [key, button] of Object.entries(tabs)) {
      button.setAttribute("aria-selected", String(key === mode)); button.tabIndex = key === mode ? 0 : -1;
    }
    content.setAttribute("aria-labelledby", tabs[mode].id);
    sources.hidden = mode !== "transcript";
    microphone.textContent = t("Microphone"); system.textContent = t("System audio");
    microphone.disabled = busy; system.disabled = busy;
    microphone.setAttribute("aria-pressed", String(source === "microphone")); system.setAttribute("aria-pressed", String(source === "system"));
    const nextContent = JSON.stringify([meeting, mode, source, locale()]);
    if (nextContent !== contentKey) {
      contentKey = nextContent; content.replaceChildren();
      if (meeting && mode !== "transcript") {
        content.append(...(meeting.notes ? meetingNotes(meeting.notes, mode) : [el("p", { text: t("No notes were saved with this meeting.") })]));
      } else if (meeting) {
        content.append(el("p", { text: t("Each source has its own recording clock. Times and voice labels are estimates; names apply only to this source in this meeting.") }));
        const track = meeting[source];
        if (!track.segments.length) content.append(el("p", { text: t("No speech was detected in this source.") }));
        for (const segment of track.segments) {
          const name = segment.speaker === null ? t("Unknown voice") : meeting.speakers.find(s => s.source === source && s.id === segment.speaker)?.name || voice(segment.speaker);
          content.append(el("div", { class: "meeting-segment" }, el("span", { class: "meeting-segment-time", text: `${time(segment.start_ms)}–${time(segment.end_ms)}` }),
            el("p", {}, el("strong", { text: name }), document.createTextNode(` ${segment.text}`))));
        }
      }
    }
    people.hidden = !meeting?.speakers.some(s => s.source === source) || mode !== "transcript";
    speakerTitle.textContent = t("Voice names"); speakerNote.textContent = t("Names apply only to this source in this meeting. Leave a name blank to restore its anonymous label.");
    const nextSpeakers = JSON.stringify([meeting?.id, source, locale()]);
    if (nextSpeakers !== speakerKey) {
      speakerKey = nextSpeakers; speakers.replaceChildren();
      for (const speaker of meeting?.speakers.filter(s => s.source === source) ?? []) {
        const draftKey = `${meeting!.id}:${source}:${speaker.id}`;
        const input = el("input"); input.type = "text"; input.maxLength = 120; input.value = drafts.get(draftKey) ?? speaker.name ?? "";
        input.addEventListener("input", () => drafts.set(draftKey, input.value));
        input.setAttribute("aria-label", t("Name for {voice} ({source})", { voice: voice(speaker.id), source: sourceName(source) }));
        const save = el("button", { text: t("Save name"), onclick: () => {
          const name = input.value.trim() || null, id = meeting?.id, selectedSource = source;
          if (!id || busy || new TextEncoder().encode(name ?? "").length > 120) {
            if (!busy) { actionError = t("Use a voice name of at most 120 bytes."); draw(); } return;
          }
          void act(async () => {
            if (!await api.rename(id, selectedSource, speaker.id, name)) throw new Error("Voice no longer exists");
            return () => { if (meeting?.id === id) { speaker.name = name; input.value = name ?? ""; drafts.set(draftKey, input.value); contentKey = ""; } };
          }, t("Could not save this voice name. Your draft is kept; reload or try again."));
        } });
        speakers.append(el("div", { class: "meeting-speaker-editor" }, el("label", {}, el("span", { text: voice(speaker.id) }), input), save));
      }
    }
    for (const control of speakers.querySelectorAll<HTMLInputElement | HTMLButtonElement>("input,button")) control.disabled = busy;
    actions.hidden = !meeting || selected === null;
    formatText.textContent = t("Export format"); sourceText.textContent = t("Subtitle source");
    exportSource.options[0].textContent = t("Choose a source"); exportSource.options[1].textContent = t("Microphone"); exportSource.options[2].textContent = t("System audio");
    sourceLabel.hidden = format.value === "markdown";
    format.disabled = busy; exportSource.disabled = busy;
    exportButton.textContent = t("Export…"); exportButton.disabled = busy || (format.value !== "markdown" && !exportSource.value);
    remove.textContent = t("Delete…"); remove.disabled = busy;
  }
  async function load() {
    if (!live() || busy) return;
    const mine = ++generation, id = selected; reading = true; failed = false; draw();
    try {
      if (id) {
        const result = await api.read(id);
        if (mine !== generation || !live()) return;
        if (result.meeting && result.meeting.id !== id) throw new Error("Wrong saved meeting");
        meeting = result.meeting; speakerKey = "";
      } else {
        const result = await api.list();
        if (mine !== generation || !live()) return;
        rows = result.rows;
      }
    } catch { if (mine === generation && live()) failed = true; }
    finally { if (mine === generation && live()) { reading = false; draw(); } }
  }
  function open(id: string | null) {
    if (busy) return;
    ++generation; selected = id; meeting = null; failed = false; confirming = false; exported = null; actionError = "";
    body.scrollTop = 0; contentKey = ""; speakerKey = ""; people.open = false; draw();
    if (id) back.focus({ preventScroll: true });
    void load();
  }
  async function act(action: () => Promise<() => void>, error: string) {
    if (!live() || busy || !meeting) return;
    const mine = ++generation; reading = false; busy = true; actionError = ""; exported = null; draw();
    try {
      const apply = await action();
      if (mine === generation && live()) apply();
    } catch { if (mine === generation && live()) actionError = error; }
    finally { if (mine === generation && live()) { busy = false; draw(); } }
  }
  for (const [key, button] of Object.entries(tabs) as [typeof mode, HTMLButtonElement][]) {
    button.id = `saved-meeting-tab-${key}`; button.setAttribute("role", "tab"); button.setAttribute("aria-controls", content.id);
    button.addEventListener("click", () => { mode = key; draw(); });
    button.addEventListener("keydown", event => {
      if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
      event.preventDefault(); const keys = Object.keys(tabs) as (typeof mode)[];
      mode = event.key === "Home" ? keys[0] : event.key === "End" ? keys.at(-1)! : keys[(keys.indexOf(key) + (event.key === "ArrowRight" ? 1 : -1) + keys.length) % keys.length];
      draw(); tabs[mode].focus();
    });
  }
  back.addEventListener("click", () => { open(null); search.focus(); });
  reload.addEventListener("click", () => { actionError = ""; void load(); });
  search.addEventListener("keydown", event => {
    if (event.key === "Escape" && search.value) { event.preventDefault(); event.stopPropagation(); search.value = ""; draw(); }
  });
  search.addEventListener("input", draw); format.addEventListener("change", draw); exportSource.addEventListener("change", draw);
  microphone.addEventListener("click", () => { if (!busy) { source = "microphone"; draw(); } });
  system.addEventListener("click", () => { if (!busy) { source = "system"; draw(); } });
  remove.addEventListener("click", () => { if (!busy) { confirming = true; draw(); body.scrollTop = 0; keep.focus(); } });
  keep.addEventListener("click", () => { if (!busy) { confirming = false; draw(); remove.focus(); } });
  confirmation.addEventListener("keydown", event => { if (event.key === "Escape" && !busy) { event.preventDefault(); confirming = false; draw(); remove.focus(); } });
  confirmDelete.addEventListener("click", () => {
    if (!meeting || !confirming || busy) return;
    const id = meeting.id;
    void act(async () => { await api.remove(id); return () => {
      rows = rows?.filter(row => row.id !== id) ?? null; selected = null; meeting = null; confirming = false;
      // Invalidate all old reads even when deletion did not change the global history epoch.
      ++generation; busy = false; draw(); search.focus(); void load();
    }; }, t("Could not delete this meeting. Reload to check its current state before trying again."));
  });
  exportButton.addEventListener("click", () => {
    if (!meeting || exportButton.disabled) return;
    const id = meeting.id, chosenFormat = format.value as "markdown" | "srt" | "vtt";
    const chosenSource = chosenFormat === "markdown" ? null : exportSource.value as MeetingSource;
    void act(async () => { const written = await api.export(id, chosenFormat, chosenSource, t("Export meeting")); return () => { exported = written; }; },
      t("Could not export this meeting. Reload to check the saved text, then try again."));
  });
  function invalidate() {
    ++generation; reading = false; busy = false; rows = null; meeting = null; confirming = false; actionError = ""; exported = null;
    contentKey = ""; speakerKey = ""; listKey = ""; search.value = ""; drafts.clear(); draw();
  }
  draw();
  return { element, open, relabel: draw, refresh: load, invalidate,
    setActive(on: boolean) { if (active === on) return; active = on; if (on) void load(); else invalidate(); },
  };
}
