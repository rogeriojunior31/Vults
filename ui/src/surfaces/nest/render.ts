import { specialistBackend } from "./specialist";
import { createProfiles } from "./profiles";
import { DICTATION_WORK_PAGES } from "./dictation/navigation";
import { createDictationArea, type DictationAreaBackend } from "./dictation/area";
import { createDictationStatistics } from "./dictation/statistics";
import { sidePanel } from "./panels";
// The Nest's page takes its backend: four tabs and the gear (ADRs 0021, 0031). Chat keeps
// Memory beside it; Activity holds Week, By project, History, Cost, Traces and Audit.
// Specialist recipients remain available with Zeca off.
import type { AgentProfileState, AgentProfile, MediaAction, NowPlaying, Size, SpendToday } from "../../bridge";
import { el } from "../../dom";
import { createPalette, paletteItems } from "../../palette";
import { t } from "../../i18n";
import { ChatPanel, type ChatBackend } from "../../island/chat";
import { Clock } from "../../clock";
import { FOCUS_SCENE, Scene } from "../../island/scene";
import { setZecaLook, setZecaSpecies } from "../../island/flock";
import { icon } from "../../island/icons";
import { agentName, skipButton, statusText } from "../../island/views";
import { renderFull } from "../../island/markdown";
import type { SessionView, ViewModel } from "../../view.gen";
import { createConnections, type ConnectionsActions } from "./connections";
import { createFlock, needsYou, type FlockActions } from "./flock";
import { createNestFoot, createNestHead, NEST_TABS, tabByKey, type NestTab } from "./head";
import { spentLine } from "../../cost";
import { viewState } from "./view-state";
import { createMemory, type MemoryBackend } from "./memory";
import { createRecent } from "./recent";
import {
  createAudit,
  createHistory,
  createTraces,
  createUsage,
  type RecordsBackend,
} from "./records";
import { createRoosts, type RoostsBackend } from "./roosts";
import { createWeek, type WeekBackend } from "./week";

export interface Nest {
  flushState: () => Promise<void>;
  chat: ChatPanel;
  /** The flock's view, as each surface gets it. */
  view: (v: ViewModel) => void;
  /** The header's words again, in the language just set. */
  relabel: () => void;
  /** The memory changed: its panel reads it again, when open. */
  memoryChanged: () => void;
  dictationChanged: (erased?: boolean) => void;
  meetingsErased: () => void;
  /** Spend or its budget changed: refresh the open records. */
  spendChanged: () => void;
  /** Zeca switched on or off: the Chat tab comes or goes. */
  setZeca: (on: boolean) => void;
  /** Opens a tab by its id (`flock`, `chat`, `activity`, `connections`, or an Activity
   *  sub-tab such as `cost`), as `open_nest` asks; an unknown id is ignored. */
  open: (id: string) => void;
  /** The tab shown now. */
  current: () => string;
  /** The window went Full, or back to Room. */
  setFull: (full: boolean) => void;
  /** The sounds went on or off. */
  setSound: (on: boolean) => void;
  /** The song your player is playing, or none. */
  setMedia: (now: NowPlaying | null) => void;
  /** Zeca's species, for the footer's mark. */
  setSpecies: (id: string) => void;
  /** The plans' usage changed. */
  usageChanged: () => void;
  /** The keys the desktop bound for the shortcuts, for the cards' buttons. */
  setKeys: (keys: Record<string, string>) => void;
  /** Allow or Deny's key while the user is in the Nest: answers the card on screen, if one is
   *  (the Chat's in its tab, the session in front's on the Flock). */
  shortcut: (id: string) => void;
  profileStateChanged: (profile: string, state: AgentProfileState) => void;
  receiveSpecialistChat: (profile: string, delta: import("../../bridge").ChatDelta) => void;
  syncSpecialistChat: (profile: string, sync: import("../../bridge").ChatSync) => void;
}

export interface NestBackend {
  profiles?: () => Promise<AgentProfile[]>;
  profileSave?: typeof import("../../bridge").Bridge.agentProfileSave;
  profileState?: typeof import("../../bridge").Bridge.agentProfileStateSet;
  profileGithub?: typeof import("../../bridge").Bridge.setSpecialistGithub;
  profileAccess?: typeof import("../../bridge").Bridge.setSpecialistChatAccess;
  profileChat?: import("./specialist").SpecialistChatBackend;
  chat: ChatBackend;
  dictation: DictationAreaBackend;
  roosts: RoostsBackend;
  memory: MemoryBackend;
  records: RecordsBackend;
  spend: () => Promise<SpendToday>;
  week: WeekBackend;
  flock: FlockActions;
  connections: ConnectionsActions;
  /** The gear: Settings. */
  settings: (section?: string) => void;
  /** The footer's switch. */
  setZeca: (on: boolean) => void;
  /** The footer's way back. */
  island: () => void;
  /** The header's sound button. */
  sound: (on: boolean) => void;
  /** The header's song controls. */
  media: (action: MediaAction) => void;
  /** The header's moon: ends do not disturb. */
  endDnd: () => void;
  /** The footer's *Zeca's look*: Settings › Flock & Zeca. */
  look: () => void;
  /** The size switch (ADR 0021), with the tab shown now to keep the place. */
  size: (size: Size, tab: string) => void;
  savePlace?: (tab: string) => Promise<void>;
  openDictation?: () => void;
}

/** How often By project may ask again while the flock moves: each ask reads the repositories. */
const ROOSTS_EVERY_MS = 1500;

type Tab = NestTab;
type Sub = "week" | "projects" | "history" | "cost" | "traces" | "audit" | "dictation-statistics";
const TABS = NEST_TABS;
const SUBS: Sub[] = ["week", "dictation-statistics", "projects", "history", "cost", "traces", "audit"];

/** Activity owns figures; only Settings edits the daily budget (ADR 0031). */
function costPart(
  today: () => Promise<SpendToday>,
  usage: { element: HTMLElement; refresh: () => void },
  settings: () => void,
): { element: HTMLElement; refresh: () => void } {
  const title = el("h2");
  const about = el("p");
  const spend = el("section", { class: "cost-today" });
  const configure = el("button", { class: "btn", onclick: settings });
  let loaded: SpendToday | null = null;
  let asked = 0;
  const draw = (error?: unknown) => {
    spend.replaceChildren(
      ...(loaded ? [el("p", { text: spentLine(loaded) }),
        ...(loaded.budget !== null && loaded.usd >= loaded.budget
          ? [el("p", { text: t("Today's budget is spent: Vults starts no new session until tomorrow.") })] : [])] : []),
      ...(error !== undefined ? [viewState("error", t("Could not load {name}.", { name: t("Cost") }),
        loaded ? t("The last loaded view is kept and may be out of date.") : undefined,
        el("button", { class: "btn", text: t("Try again"), onclick: refresh }), String(error))]
        : loaded ? [] : [viewState("loading", t("Loading {name}…", { name: t("Cost") }))]),
    );
  };
  const refresh = () => {
    title.textContent = t("Cost");
    about.textContent = t("Your plan windows, recorded cost and daily budget on this computer.");
    configure.textContent = t("Budget settings");
    const mine = ++asked;
    draw();
    void today().then(value => { if (mine === asked) { loaded = value; draw(); } },
      error => { if (mine === asked) draw(error); });
    usage.refresh();
  };
  return {
    element: el("div", { class: "nest-cost" },
      el("header", { class: "activity-heading" }, el("div", {}, title, about), configure),
      spend, usage.element,
    ),
    refresh,
  };
}

export function createNest(
  root: HTMLElement,
  backend: NestBackend,
  zeca: boolean,
  startFull = false,
): Nest {
  const portrait = new Scene(FOCUS_SCENE);
  portrait.setActive(false);
  let mounted = false;
  // A change in the conversation (a message, New, another one opened) may change Recent.
  let recentTimer: number | undefined;
  const chatChanged = () => {
    if (mounted) paintPortrait();
    window.clearTimeout(recentTimer);
    recentTimer = window.setTimeout(() => refreshRecent(), 400);
  };
  let chat = new ChatPanel(backend.chat, chatChanged, {
    window: "nest",
    render: renderFull,
  });
  const companionChat = chat;
  let activeProfile: AgentProfile | null = null;
  const profilePanels = new Map<string, ChatPanel>();
  const panelHeads = new Map<ChatPanel, HTMLElement>();
  const recipientProfiles = new Map<string, AgentProfile>();
  const rememberProfile = (profile: AgentProfile) => {
    const current = recipientProfiles.get(profile.id);
    if ((current?.state?.version ?? 0) > (profile.state?.version ?? 0)) profile = { ...profile, state: current!.state };
    recipientProfiles.set(profile.id, profile);
    return profile;
  };
  let recipientRequest = 0;
  let chatKeys: Record<string, string> = {};
  const portraitButton = el("button", { class: "chat-character", onclick: () => {
    if (chat.clip(Clock.now()) === "idle" && !window.matchMedia("(prefers-reduced-motion: reduce)").matches
      && !document.body.classList.contains("still")) portrait.react("hello");
  } }, el("span", { class: "perch" }, portrait.canvas));
  portraitButton.addEventListener("pointerenter", () => portrait.lookAt("front"));
  portraitButton.addEventListener("pointerleave", () => portrait.lookAt(null));
  portrait.canvas.setAttribute("aria-hidden", "true");
  chat.perchSlot.append(portraitButton);
  // Outside the scrolling log, the same scene introduces an empty chat and perches by its draft.
  chat.element.querySelector(".log-box")?.prepend(chat.perchSlot);
  const flock = createFlock({
    ...backend.flock,
    // Ask Zeca about a session: the Chat, with the question ready to finish or send.
    ask: (s) => {
      void askCompanion(t("What is {project} doing now?", { project: s.project || agentName(s) }));
    },
  });
  const memory = createMemory(backend.memory);
  // A row's question for Zeca opens the Chat with it in the composer; nothing is sent.
  const connections = createConnections({
    ...backend.connections,
    traces: () => open("traces"),
    zeca: () =>
      zecaOn
        ? (text: string) => {
            void askCompanion(text);
          }
        : null,
  });
  const dictationArea = createDictationArea(backend.dictation, section => backend.settings(section ?? "dictation"), () => open("dictation-statistics"), () => { if (mounted) paintPortrait(); });
  // Activity's parts, each read again when its sub-tab opens.
  const parts: Record<Sub, { element: HTMLElement; refresh: () => void; flush?: () => Promise<void> }> = {
    week: createWeek(backend.week),
    "dictation-statistics": createDictationStatistics(backend.dictation.statistics, () => backend.settings("dictation")),
    projects: createRoosts(backend.roosts),
    history: createHistory(backend.records.history, backend.records.query),
    cost: (() => {
      const usage = createUsage(backend.records.usage);
      return costPart(backend.spend, usage, () => backend.settings("activity"));
    })(),
    traces: createTraces(backend.records.traces),
    audit: createAudit(backend.records.audit, backend.records.query),
  };

  // Chat: the conversation, and Zeca's memory beside it when asked for.
  const memoryBack = el("button", { class: "nest-memory-back", text: t("Back to Chat") });
  const memoryPane = el("aside", { class: "nest-memory" }, memoryBack, memory.element);
  const memoryPanel = sidePanel(memoryPane, "memory", "Memory", 340, "right", 220, 440);
  memoryPane.hidden = true;
  const memoryLabel = el("span", { class: "nest-memory-label", text: t("Memory") });
  const memoryButton = el(
    "button",
    { class: "chat-ico nest-memory-toggle" },
    icon("sideRight", 15, 1.6),
    memoryLabel,
  );
  const setMemory = (open: boolean, returnFocus = false) => {
    if (open && activeProfile) return;
    memoryPane.hidden = !open;
    memoryButton.classList.toggle("on", open);
    memoryButton.setAttribute("aria-pressed", String(open));
    if (open) {
      if (window.innerWidth <= 1100) setRecent(false);
      memory.refresh();
      if (matchMedia("(max-width: 600px)").matches) {
        chat.cancelVoice();
        memoryBack.focus();
      }
    } else if (returnFocus) memoryButton.focus();
    if (mounted) paintPortrait();
  };
  memoryButton.addEventListener("click", () => setMemory(memoryPane.hasAttribute("hidden")));
  memoryBack.addEventListener("click", () => setMemory(false, true));
  memoryButton.setAttribute("aria-pressed", "false");
  // Recent, beside the conversation in a wide window: the saved ones and a new one.
  const recent = createRecent({
    list: () => chat.conversations(),
    open: (id) => { chat.openConversation(id); if (!wide.matches) setRecent(false, true); },
    fresh: () => { chat.newConversation(); if (!wide.matches) setRecent(false, true); },
    busy: () => chat.isBusy(),
    close: () => setRecent(false, true),
  });
  /** A sidebar when wide, a separate view on explicit request when narrow. */
  const wide = window.matchMedia("(min-width: 900px)");
  // The title bar, as the prototype draws it: the conversations' column, the conversation's title,
  // the chat's own pickers (moved here from its head) and the Memory panel.
  const chatTitle = el("h2", { class: "chat-title" });
  const recentButton = el("button", {
    class: "chat-ico recent-toggle",
    onclick: () => {
      setRecent(wide.matches ? chatPane.classList.contains("no-recent") : !chatPane.classList.contains("recent-open"));
    },
  }, icon("sideLeft", 15, 1.6));
  recentButton.setAttribute("aria-pressed", String(wide.matches));
  recentButton.setAttribute("aria-controls", "nest-recent");
  recent.element.id = "nest-recent";
  const recentPanel = sidePanel(recent.element, "recent", "Conversations", 248, "left", 180, 360);
  /** The conversation going on, for the title: asked with Recent, whatever the window's width. */
  let titled = 0;
  const refreshTitle = () => {
    const mine = ++titled;
    void chat.conversations().then(
      (rows) => {
        if (mine === titled) chatTitle.textContent = rows.find((c) => c.current)?.title || t("New conversation");
      },
      () => {},
    );
  };
  const refreshRecent = () => {
    if (tab !== "chat") return;
    refreshTitle();
    if (wide.matches && !chatPane.classList.contains("no-recent") || !wide.matches && chatPane.classList.contains("recent-open")) recent.refresh();
  };
  wide.addEventListener("change", () => {
    recentButton.setAttribute("aria-pressed", String(wide.matches ? !chatPane.classList.contains("no-recent") : chatPane.classList.contains("recent-open")));
    refreshRecent();
    if (!wide.matches && !chatPane.classList.contains("recent-open") && recent.element.contains(document.activeElement)) recentButton.focus();
    if (mounted) paintPortrait();
  });
  const recipientPicker = el("select", { class: "place chat-recipient-picker" });
  recipientPicker.setAttribute("aria-label", t("Recipient"));
  const recipientControl = el("div", { class: "chat-recipient-control" }, recipientPicker, icon("chevronDown", 11, 2));
  const recipientError = el("p", { class: "nest-close-error" }); recipientError.hidden = true; recipientError.setAttribute("role", "alert");
  const fillRecipients = () => {
    const zeca = el("option", { text: "Zeca" }); zeca.value = "zeca"; zeca.disabled = !zecaOn;
    recipientPicker.replaceChildren(zeca, ...[...recipientProfiles.values()].filter(profile => !profile.state?.archived || profile.id === activeProfile?.id).map(profile => {
      const option = el("option", { text: profile.built_in ? t(profile.title) : profile.title }); option.value = profile.id; return option;
    }));
    recipientPicker.value = activeProfile?.id ?? "zeca";
  };
  recipientPicker.addEventListener("focus", () => {
    void backend.profiles?.().then(rows => { for (const profile of rows) rememberProfile(profile); fillRecipients(); }, error => { recipientError.textContent = String(error); recipientError.hidden = false; });
  });
  recipientPicker.addEventListener("change", () => { void selectRecipient(recipientProfiles.get(recipientPicker.value) ?? null); });
  const companionHead = chat.element.querySelector<HTMLElement>(".chat-head")!;
  panelHeads.set(chat, companionHead);
  const chatControls = el("div", { class: "chat-controls" }, companionHead);
  const chatMain = el("div", { class: "nest-chat-main" }, chat.element);
  const chatPane = el(
    "div",
    { class: "nest-chat" },
    el(
      "header",
      { class: "chat-titlebar" },
      recentButton,
      chatTitle,
      // The chat's pickers and buttons, moved once: they keep their listeners and repaint in place.
      recipientControl, chatControls,
      memoryButton,
    ),
    el(
      "div",
      { class: "nest-chat-body" },
      // After the conversation in the page, so the keyboard meets the composer first; drawn left.
      chatMain,
      recent.element,
      memoryPane,
    ),
    recipientError,
  );

  async function selectRecipient(profile: AgentProfile | null): Promise<boolean> {
    const mine = ++recipientRequest;
    const previous = chat;
    recipientPicker.disabled = true;
    try {
      if (!profile && !zecaOn) throw new Error(t("Zeca is off (Settings → Flock & Zeca)."));
      await previous.flushDraft();
      if (mine !== recipientRequest) return false;
      let next = companionChat;
      if (profile) {
        const selectedProfile = rememberProfile(profile);
        if (!backend.profileChat) throw new Error(t("Could not load {name}.", { name: t("Chat") }));
        const existing = profilePanels.get(selectedProfile.id);
        if (existing) next = existing;
        else {
          const identity = { github:selectedProfile.github, name: selectedProfile.title, builtIn: selectedProfile.built_in, revision: selectedProfile.revision, latestRevision: selectedProfile.revision, state: selectedProfile.state, providers: selectedProfile.providers, species: selectedProfile.species, look: selectedProfile.look };
          let panel: ChatPanel | undefined;
          const adapter = specialistBackend(backend.profileChat, selectedProfile, identity);
          adapter.setAccess = async access => { await backend.profileAccess?.(selectedProfile.id, access); selectedProfile.access = access; profiles.updateAccess(selectedProfile.id, access); };
          adapter.openSettings = () => { profilesShown = true; profiles.select(selectedProfile.id); show("flock"); };
          panel = new ChatPanel(adapter, () => { if (panel && chat === panel) chatChanged(); }, { window: "nest", render: renderFull, specialist: identity });
          panel.setKeys(chatKeys);
          panel.setAccess(selectedProfile.access);
          profilePanels.set(selectedProfile.id, panel);
          panelHeads.set(panel, panel.element.querySelector<HTMLElement>(".chat-head")!);
          next = panel;
        }
        next.setAccess(selectedProfile.access);
        next.setProfileRevision(selectedProfile.revision);
        next.setProfileState(selectedProfile.state);
        next.setProfileGithub(selectedProfile.github);
      }
      await next.restore();
      if (profile) next.setFolders(last?.sessions.flatMap(s => s.cwd ? [{cwd:s.cwd,project:s.project}] : []) ?? [], null);
      if (mine !== recipientRequest) return false;
      previous.toggle(false);
      chat = next; activeProfile = profile;
      recent.clear();
      chatTitle.textContent = t("New conversation");
      chatControls.replaceChildren(panelHeads.get(chat)!);
      chatMain.replaceChildren(chat.element);
      chat.element.querySelector(".log-box")?.prepend(chat.perchSlot);
      chat.perchSlot.append(portraitButton);
      memoryButton.hidden = !!profile; setMemory(false);
      head.setZeca(zecaOn || !!profile);
      recipientError.hidden = true;
      fillRecipients();
      show("chat");
      return true;
    } catch (error) {
      if (mine !== recipientRequest) return false;
      recipientError.textContent = String(error); recipientError.hidden = false; fillRecipients();
      return false;
    } finally { if (mine === recipientRequest) recipientPicker.disabled = false; }
  }

  async function askCompanion(text: string): Promise<void> {
    const selected = await selectRecipient(null);
    if (selected && chat === companionChat && zecaOn) await companionChat.askZeca(text);
  }

  function setRecent(open: boolean, returnFocus = false): void {
    if (open && window.innerWidth <= 1100) setMemory(false);
    if (wide.matches) chatPane.classList.toggle("no-recent", !open);
    else {
      chatPane.classList.toggle("recent-open", open);
      if (open) chat.cancelVoice();
    }
    recentButton.setAttribute("aria-pressed", String(open));
    if (open) {
      recent.refresh();
      recent.element.querySelector<HTMLInputElement>(".nest-recent-search")?.focus();
    } else if (returnFocus) recentButton.focus();
    if (mounted) paintPortrait();
  }

  const subButtons = SUBS.map((id) => {
    const b = el("button", { class: "nest-sub", onclick: () => showSub(id) });
    b.dataset.sub = id;
    return b;
  });
  const part = (id: Sub) => {
    const p = el(
      "div",
      { class: `nest-part nest-${id}-part` },
      parts[id].element,
    );
    p.hidden = true;
    return p;
  };
  const subPanes: Record<Sub, HTMLElement> = {
    week: part("week"),
    "dictation-statistics": part("dictation-statistics"),
    projects: part("projects"),
    history: part("history"),
    cost: part("cost"),
    traces: part("traces"),
    audit: part("audit"),
  };
  const activityPane = el(
    "div",
    { class: "nest-activity" },
    el("nav", { class: "nest-subs" }, ...subButtons),
    ...SUBS.map((id) => subPanes[id]),
  );

  let profilesShown = false;
  const profilesLink = el("button", { class: "btn agent-profiles-open", onclick: () => {
    profilesShown = true; show("flock");
  } });
  flock.element.querySelector(".habitat-header")?.append(profilesLink);
  const profiles = createProfiles(() => backend.profiles?.() ?? Promise.reject(), () => {
    profilesShown = false; show("flock"); profilesLink.focus({ preventScroll: true });
  }, backend.profileSave, backend.profileAccess, async profile => {
    if (!await selectRecipient(profile)) throw new Error(recipientError.textContent ?? t("Could not load {name}.", { name: t("Chat") }));
  }, profile => {
    profile = rememberProfile(profile);
    profilePanels.get(profile.id)?.setAccess(profile.access);
    profilePanels.get(profile.id)?.setProfileRevision(profile.revision);
    profilePanels.get(profile.id)?.setProfileState(profile.state);
    profilePanels.get(profile.id)?.setProfileGithub(profile.github);
    if (activeProfile?.id === profile.id) activeProfile = profile;
    fillRecipients();
  }, backend.profileState, backend.profileGithub);
  const panes: Record<Tab, HTMLElement> = {
    flock: el("div", { class: "nest-pane nest-flock-pane" }, flock.element, profiles.element),
    chat: chatPane,
    activity: activityPane,
    dictation: el("div", { class: "nest-pane nest-dictation-pane" }, dictationArea.element),
    connections: el("div", { class: "nest-pane nest-connections-pane" }, connections.element),
  };
  for (const id of TABS) {
    panes[id].id = `nest-pane-${id}`;
    panes[id].setAttribute("role", "tabpanel");
    panes[id].setAttribute("aria-labelledby", `nest-tab-${id}`);
  }
  const head = createNestHead({
    tab: (id) => chooseTab(id),
    gear: () => backend.settings(),
    size: (s) => backend.size(s, currentPlace()),
    search: () => openPalette(),
    sound: backend.sound,
    media: backend.media,
    endDnd: backend.endDnd,
  });
  let zecaOn = zeca;
  /** The session in front, whose terminal the footer opens. */
  const front = () => {
    const f = last?.focus;
    return f ? last?.sessions.find((s) => s.agent === f.agent && s.id === f.id) : undefined;
  };
  const foot = createNestFoot({
    zeca: (on) => backend.setZeca(on),
    look: backend.look,
    terminal: () => {
      const s = front();
      if (s) backend.flock.jump(s);
    },
    island: backend.island,
  });
  let full = startFull;

  let tab: Tab = "flock";
  let last: ViewModel | null = null;
  let sub: Sub = "week";
  const currentPlace = () => tab === "activity" ? sub : tab === "flock" && profilesShown ? "flock-profiles" : tab;
  const portraitSpace = window.matchMedia("(min-height: 561px)");
  const portraitNarrow = window.matchMedia("(max-width: 600px)");
  function paintPortrait(): void {
    const voice = tab === "dictation" ? dictationArea.character() : null;
    dictationArea.perch.hidden = !voice || !zecaOn;
    const destination = voice?.element ?? portraitButton.querySelector(".perch")!;
    // Only the visible conversation owns this canvas; voice feedback never opens another surface.
    if (portrait.canvas.parentElement !== destination) destination.append(portrait.canvas);
    const available = voice ? zecaOn : zecaOn || !!activeProfile;
    const name = voice ? "Zeca" : chat.recipientName();
    portraitButton.setAttribute("aria-label", t("Greet {name}", { name }));
    portraitButton.title = t("Greet {name}", { name });
    portrait.setActive(available && !document.hidden && !last?.locked
      && root.isConnected && portraitSpace.matches && (!!voice || tab === "chat"
        && !(portraitNarrow.matches && !memoryPane.hidden)
        && (wide.matches || !chatPane.classList.contains("recent-open"))));
    portrait.update([], null, available ? { clip: voice?.clip ?? chat.clip(Clock.now()), agent: voice ? "other" : chat.agent(), immediate: !!voice, set: voice ? undefined : chat.specialistAppearance() ?? undefined } : null);
  }
  portraitSpace.addEventListener("change", paintPortrait);
  portraitNarrow.addEventListener("change", paintPortrait);
  let navigationReady = false;
  let placeSaved: Promise<void> = Promise.resolve();
  const rememberPlace = () => {
    if (!navigationReady) return;
    const place = currentPlace();
    placeSaved = placeSaved.catch(() => {}).then(() => backend.savePlace?.(place));
    void placeSaved.catch(() => {});
  };

  function showSub(id: Sub): void {
    sub = id;
    for (const s of SUBS) subPanes[s].hidden = s !== id;
    subButtons.forEach((b) => {
      const selected = b.dataset.sub === id;
      b.classList.toggle("on", selected);
      b.setAttribute("aria-pressed", String(selected));
      if (selected) b.scrollIntoView({ block: "nearest", inline: "nearest" });
    });
    parts[id].refresh();
    rememberPlace();
  }
  function chooseTab(id: Tab): void {
    if (id === "flock") profilesShown = false;
    if (id === "dictation" && backend.openDictation) backend.openDictation();
    else show(id);
  }
  function show(id: Tab): void {
    if (id === "chat" && !zecaOn && !activeProfile) id = "flock";
    tab = id;
    if (id !== "activity") rememberPlace();
    for (const k of TABS) panes[k].hidden = k !== id;
    head.set(id);
    panes.flock.classList.toggle("show-profiles", profilesShown);
    flock.element.hidden = id === "flock" && profilesShown;
    profilesLink.hidden = profilesShown;
    profiles.element.hidden = !profilesShown;
    profiles.visible(id === "flock" && profilesShown);
    paintNeeds();
    // The habitat draws only while it is in sight.
    flock.setActive(id === "flock" && !profilesShown && !document.hidden && !last?.locked);
    if (id === "activity") showSub(sub);
    if (id === "connections") connections.refresh();
    dictationArea.setActive(id === "dictation" && !last?.locked);
    if (id === "dictation") dictationArea.refresh();
    if (id === "chat") {
      chat.toggle(true);
      refreshRecent();
    } else chat.toggle(false);
    paintPortrait();
  }
  const relabel = () => {
    recipientPicker.setAttribute("aria-label", t("Recipient"));
    profilesLink.textContent = t("Agent profiles");
    profiles.relabel();
    const subWords: Record<Sub, string> = {
      week: t("Week"),
      "dictation-statistics": t("Dictation"),
      projects: t("By project"),
      history: t("History"),
      cost: t("Cost"),
      traces: t("Traces"),
      audit: t("Audit"),
    };
    for (const b of subButtons) b.textContent = subWords[b.dataset.sub as Sub];
    memoryButton.title = t("Memory");
    memoryButton.setAttribute("aria-label", t("Memory"));
    memoryLabel.textContent = t("Memory");
    memoryBack.textContent = t("Back to Chat");
    recentButton.title = t("Recent");
    recentButton.setAttribute("aria-label", t("Recent"));
    memoryPanel.relabel(); recentPanel.relabel();
    foot.relabel();
    head.relabel();
    companionChat.relabel();
    for (const panel of profilePanels.values()) panel.relabel();
    fillRecipients();
    // What the tabs drew is in the old language too.
    if (last) flock.update(last);
    connections.redraw();
    dictationArea.relabel();
    if (tab === "activity") parts[sub].refresh();
    refreshRecent();
    paintPortrait();
  };
  const setZeca = (on: boolean) => {
    zecaOn = on;
    companionChat.setEnabled(on);
    head.setZeca(on || !!activeProfile);
    fillRecipients();
    foot.setZeca(on);
    flock.setZeca(on);
    paintPortrait();
    relabel();
    if (!on && tab === "chat" && !activeProfile) show("flock");
  };

  // Over every tab but the Flock (which shows them on the wire), who needs you: a tag each, as the
  // prototype draws it; a click puts that session in front on the Flock, where its card waits.
  const needStrip = el("div", { class: "need-strip nest-need" });
  const paintNeeds = () => {
    // Drawn on every view: the tag that had the focus gets it back.
    const had = needStrip.contains(document.activeElement) ? (document.activeElement as HTMLElement).dataset.key : null;
    const wanting = tab === "flock" && !profilesShown ? [] : (last?.sessions ?? []).filter((s) => !s.skipped && (needsYou(s) || s.status === "failed" || s.attention === "failed"));
    needStrip.hidden = !wanting.length;
    needStrip.setAttribute("aria-label", wanting.length === 1 ? t("1 session needs you") : t("{n} sessions need you", { n: wanting.length }));
    needStrip.replaceChildren(
      ...wanting.map((s: SessionView) => {
        const failed = s.status === "failed" || s.attention === "failed";
        const b = el(
          "button",
          {
            class: `need-tag${failed ? " failed" : ""}`,
            onclick: () => {
              backend.flock.focus(s);
              chooseTab("flock");
            },
          },
          el("i", { text: failed ? "×" : "!" }),
          el("span", { text: s.project || agentName(s) }),
        );
        b.dataset.key = `${s.agent}:${s.id}`;
        b.title = [s.project || agentName(s), statusText(s)].join(" · ");
        const skip = s.request ? skipButton(s.request, backend.flock.skip, t("Skip {project} for now", { project: s.project || agentName(s) })) : null;
        if (skip) skip.dataset.key = b.dataset.key;
        return el("span", { class: "need-chip" }, b, skip);
      }),
      ...(wanting.filter(s => s.request).length > 1 ? [(() => {
        const all = el("button", { class: "needs-skip-all", text: t("Skip all"), onclick: () => wanting.forEach(s => s.request && backend.flock.skip(s.request)) });
        all.dataset.key = "skip-all";
        return all;
      })()] : []),
    );
    if (had) (needStrip.querySelector<HTMLElement>(`[data-key="${CSS.escape(had)}"]`)
      ?? needStrip.querySelector<HTMLElement>(".need-tag") ?? root.querySelector<HTMLElement>(".suite-tab.on"))?.focus();
  };
  root.append(
    ...head.elements,
    needStrip,
    ...TABS.map((id) => panes[id]),
    foot.element,
  );
  mounted = true;
  setZeca(zeca);
  show("flock");
  navigationReady = true;

  // Hidden (minimized, covered) or locked, nobody is looking: the habitat rests.
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) chat.cancelVoice();
    dictationArea.visible(!document.hidden);
    flock.setActive(tab === "flock" && !profilesShown && !document.hidden && !last?.locked);
    paintPortrait();
  });
  let waiting: number | null = null;
  const view = (v: ViewModel) => {
    last = v;
    for (const panel of profilePanels.values()) panel.setFolders(v.sessions.flatMap(s => s.cwd ? [{cwd:s.cwd,project:s.project}] : []), null);
    dictationArea.setActive(tab === "dictation" && !v.locked);
    if (v.locked) chat.cancelVoice();
    setZecaLook(v.look ?? null);
    paintPortrait();
    head.setDnd(!!v.dnd);
    head.setCounts({
      sessions: v.sessions.length,
      needs: v.sessions.filter(needsYou).length,
      connections: (v.boards ?? []).reduce((n, b) => n + b.rows.filter((r) => r.group !== "releases").length, 0),
    });
    const f = front();
    foot.setTerminal(!!f && f.raise !== false);
    flock.setActive(tab === "flock" && !profilesShown && !document.hidden && !v.locked);
    flock.update(v);
    connections.update(v);
    paintNeeds();
    // A moving flock may move By project; it reads the repositories, so not on every view.
    if (tab !== "activity" || sub !== "projects" || waiting !== null) return;
    waiting = window.setTimeout(() => {
      waiting = null;
      if (tab === "activity" && sub === "projects") parts.projects.refresh();
    }, ROOSTS_EVERY_MS);
  };
  const memoryChanged = () => {
    if (!memoryPane.hidden) memory.refresh();
  };
  const spendChanged = () => {
    // A run's spend or answers may have changed what an open record shows.
    if (tab === "activity" && sub !== "week" && sub !== "projects")
      parts[sub].refresh();
  };
  const open = (id: string) => {
    if (id === "flock-profiles") { profilesShown = true; show("flock"); return; }
    if (id === "flock") profilesShown = false;
    if (id === "memory") {
      if (zecaOn) { void selectRecipient(null).then(() => setMemory(true)); }
      return;
    }
    if (id.startsWith("connection:")) {
      const parts = id.split(":");
      if (parts.length === 3) {
        try {
          show("connections");
          connections.select(decodeURIComponent(parts[1]!), decodeURIComponent(parts[2]!));
        } catch { /* An invalid navigation link changes no connection. */ }
      }
      return;
    }
    const workPage = DICTATION_WORK_PAGES.find(p => id === `dictation-${p}`);
    if (workPage) { dictationArea.select(workPage); show("dictation"); return; }
    if ((TABS as string[]).includes(id)) return show(id as Tab);
    if ((SUBS as string[]).includes(id)) {
      sub = id as Sub;
      show("activity");
    }
  };
  // Ctrl+K: the palette, over the tab (it never answers a card: a waiting session leads to its card).
  const palette = createPalette(() => (paletteHost.hidden = true));
  const paletteHost = el("div", { class: "palette-host" }, palette.element);
  paletteHost.hidden = true;
  paletteHost.addEventListener("click", (e) => {
    if (e.target === paletteHost) paletteHost.hidden = true;
  });
  root.append(paletteHost);
  function openPalette(): void {
    paletteHost.hidden = false;
    palette.open(
      paletteItems({
        sessions: last?.sessions ?? [],
        focus: backend.flock.focus,
        review: backend.flock.review,
        command: zecaOn
          ? (text) => {
              void askCompanion(text);
            }
          : null,
        tab: id => { if (id === "dictation") chooseTab("dictation"); else open(id); },
      }),
    );
  }
  window.addEventListener("keydown", (e) => {
    if (!(e.ctrlKey || e.metaKey) || e.altKey || e.key.toLowerCase() !== "k") return;
    e.preventDefault();
    openPalette();
  });
  // Alt+1 to 5 pick a tab by the key's place, whatever the layout (a hidden Chat is skipped); Esc
  // leaves Full for the size before it. Neither while typing, composing or speaking.
  window.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && !e.isComposing && !e.defaultPrevented && tab === "chat" && chat.cancelVoice()) {
      e.preventDefault();
      return;
    }
    if (e.key === "Escape" && !e.isComposing && !e.defaultPrevented && paletteHost.hidden && tab === "chat" && !wide.matches && chatPane.classList.contains("recent-open")) {
      e.preventDefault();
      setRecent(false, true);
      return;
    }
    const typing = (e.target as Element | null)?.closest?.("input, textarea, select, [contenteditable]");
    if (e.isComposing || e.defaultPrevented || typing) return;
    // Esc closes an open palette first, wherever the focus went.
    if (e.key === "Escape" && !paletteHost.hidden) {
      e.preventDefault();
      paletteHost.hidden = true;
      return;
    }
    if (e.key === "Escape" && tab === "chat" && !memoryPane.hidden && matchMedia("(max-width: 600px)").matches) {
      e.preventDefault();
      setMemory(false, true);
      return;
    }
    if (e.key === "Escape" && full) {
      e.preventDefault();
      backend.size("room", currentPlace());
      return;
    }
    const pick = tabByKey(e, zecaOn || !!activeProfile);
    if (pick) {
      e.preventDefault();
      chooseTab(pick);
    }
  });
  const setFull = (on: boolean) => {
    full = on;
    root.classList.toggle("full", on);
    head.setFull(on);
    flock.setFull(on);
  };
  setFull(startFull);
  return {
    flushState: async () => {
      chat.cancelVoice();
      await chat.flushDraft();
      await connections.flush();
      await flock.flush();
      await Promise.all([parts.projects.flush?.(), parts.history.flush?.(), parts.audit.flush?.()]);
      rememberPlace();
      await placeSaved;
    },
    chat: companionChat,
    view,
    relabel,
    memoryChanged,
    dictationChanged: (erased = false) => {
      if (erased) dictationArea.invalidate();
      if (tab === "dictation") dictationArea.refresh();
      if (tab === "activity" && (sub === "dictation-statistics" || sub === "week")) parts[sub].refresh();
    },
    meetingsErased: dictationArea.meetingsErased,
    spendChanged,
    setZeca,
    open,
    current: currentPlace,
    setFull,
    setSound: head.setSound,
    setMedia: head.setMedia,
    setSpecies: (id) => {
      setZecaSpecies(id);
      foot.setSpecies(id);
      paintPortrait();
    },
    usageChanged: () => {
      flock.usageChanged();
      if (tab === "activity" && sub === "cost") parts.cost.refresh();
    },
    setKeys: (keys) => {
      chatKeys = keys;
      companionChat.setKeys(keys);
      for (const panel of profilePanels.values()) panel.setKeys(keys);
      flock.setKeys(keys);
    },
    shortcut: (id) => {
      if (id === "talk-cancel") {
        chat.cancelVoice();
        return;
      }
      if (id !== "allow" && id !== "deny") return;
      const allow = id === "allow";
      if (tab === "chat") chat.answerWaiting(allow);
      else if (tab === "flock" && !profilesShown) flock.shortcut(allow);
    },
    profileStateChanged: (id, state) => {
      const profile = recipientProfiles.get(id);
      if (profile && state.version >= (profile.state?.version ?? 0)) {
        profile.state = state;
        if (activeProfile?.id === id) activeProfile = profile;
        profilePanels.get(id)?.setProfileState(state);
        profiles.updateState(id, state); fillRecipients();
      }
    },
    receiveSpecialistChat: (profile, delta) => profilePanels.get(profile)?.receive(delta),
    syncSpecialistChat: (profile, sync) => profilePanels.get(profile)?.sync(sync),
  };
}
