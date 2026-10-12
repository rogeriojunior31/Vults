// The Nest: the Panel and Full sizes (ADR 0021), a window built when opened and gone when closed
// (ADR 0008, D6). It opens with Zeca off too, without its Chat. Its Chat is the island's
// conversation in full Markdown: the same chat, so a message sent in either shows in both. Cards
// show here, but only the island answers them.
import { Bridge, type OtherAgent } from "../../bridge";
import { setLang } from "../../i18n";
import { createNest } from "./render";

/** The flock's folders now, for a new task's roost. */
let folders: { cwd: string; project: string }[] = [];
// The ACP agents installed here, read once: the Tasks tab draws again when they come.
let otherAgents: OtherAgent[] = [];

const nest = createNest(
  document.getElementById("nest")!,
  {
    chat: {
      send: (text, files, folder) => Bridge.chatSend(text, files, folder),
      // Never called: the Nest's cards have no Allow or Deny.
      decide: () => {},
      stop: () => void Bridge.chatStop(),
      reset: (provider) => Bridge.chatReset(provider),
      keyboard: () => {},
      models: (provider) => Bridge.chatModels(provider),
      pick: (folder) => Bridge.chatPick(folder),
      setPick: (folder, provider, pick) =>
        Bridge.setChatPick(folder, provider, pick),
      conversations: () => Bridge.chatConversations(),
      open: (id) => Bridge.chatOpen(id),
      rename: (id, title) => Bridge.chatRename(id, title),
      remove: (id) => Bridge.chatDelete(id),
      memory: (text, folder) => Bridge.memoryCommand(text, folder),
      task: (text, folder) => Bridge.taskCommand(text, folder),
      handoff: (text, folder) => Bridge.continueCommand(text, folder),
    },
    toIsland: () => void Bridge.chatToIsland(),
    roosts: {
      load: () => Bridge.roosts(),
      preview: (key) => Bridge.roostSharePreview(key),
      apply: (key, file, fingerprint) =>
        Bridge.roostShareApply(key, file, fingerprint),
      open: (item) => void Bridge.boardOpen("github", item),
    },
    memory: {
      load: () => Bridge.memoryView(),
      decide: (id, accept) => Bridge.memoryDecide(id, accept),
      undo: (id) => Bridge.memoryUndo(id),
    },
    tasks: {
      load: (closed) => Bridge.tasksList(closed),
      create: (goal, criteria, folder) =>
        Bridge.taskCreate(goal, criteria, folder),
      edit: (id, edit) => Bridge.taskEdit(id, edit),
      remove: (id) => Bridge.taskDelete(id),
      openIssue: (id) => Bridge.taskOpenIssue(id),
      visit: (id) => Bridge.taskVisitIssue(id),
      folders: () => folders,
      start: (id, engine) => Bridge.taskStart(id, engine),
      startFresh: (id, engine) => Bridge.taskStartFresh(id, engine),
      stop: (id) => Bridge.taskStop(id),
      review: (id) => Bridge.taskReview(id),
      removeWorktree: (id) => Bridge.taskRemoveWorktree(id),
      today: () => Bridge.spendToday(),
      agents: () => otherAgents,
    },
    budget: {
      today: () => Bridge.spendToday(),
      setBudget: (cents) => Bridge.budgetSet(cents),
    },
    records: {
      history: () => Bridge.nestHistory(),
      usage: () => Bridge.nestUsage(),
      traces: () => Bridge.nestTraces(),
      audit: () => Bridge.nestAudit(),
    },
    week: {
      activity: (monday) => Bridge.activity(monday),
      github: () => Bridge.githubCalendar(),
      saveImage: (name, png) => Bridge.saveRecapImage(name, png),
      setHistory: (on) => Bridge.setHistory(on),
      clear: () => Bridge.clearHistory(),
      connectors: () => void Bridge.openSettings("connectors"),
    },
    flock: {
      focus: (s) => void Bridge.sessionFocus({ agent: s.agent, id: s.id }),
      jump: (s) => void Bridge.sessionJump(s.agent, s.id),
      // The card is answered on the island only (D1): put the session in front, then open it.
      review: (s) => {
        void Bridge.sessionFocus({ agent: s.agent, id: s.id }).finally(
          () => void Bridge.openIsland(),
        );
      },
      diff: (s, step) => Bridge.stepDiff(s.agent, s.id, step),
      usage: () => Bridge.usage(),
    },
    connections: {
      open: (connector, item) => void Bridge.boardOpen(connector, item),
      status: () => Bridge.connectorsStatus(),
      refresh: () => Bridge.connectorsRefresh(),
      settings: () => void Bridge.openSettings("connectors"),
    },
    settings: () => void Bridge.openSettings(),
    setZeca: (on) => void Bridge.setZeca(on),
    island: () => void Bridge.openIsland(),
    sound: (on) => void Bridge.setSounds(on),
    look: () => void Bridge.openSettings("flock"),
    size: (size, tab) => void Bridge.setSize(size, tab),
  },
  false,
);
Bridge.onNestFull((full) => nest.setFull(full));
void Bridge.nestIsFull().then((full) => nest.setFull(full), () => {});
// `open_nest` names the tab in the address when it builds the window, and by an event after.
let asked = location.hash.slice(1);
nest.open(asked);
Bridge.onNestTab((tab) => {
  asked = tab;
  nest.open(tab);
});
Bridge.onMemory(() => nest.memoryChanged());
Bridge.onUsage(() => nest.usageChanged());
Bridge.onTasks(() => nest.tasksChanged());
void Bridge.taskAgents().then(
  (a) => {
    otherAgents = a;
    nest.tasksChanged();
  },
  () => {},
);

Bridge.onChat((d) => nest.chat.receive(d));
Bridge.onChatSync((s) => nest.chat.sync(s));
// The sessions' folders, for a new conversation (the view comes now, then with each change).
Bridge.onView((v) => {
  folders = v.sessions.flatMap((s) =>
    s.cwd ? [{ cwd: s.cwd, project: s.project }] : [],
  );
  nest.chat.setFolders(folders, null);
  nest.view(v);
});
void Bridge.appSettings().then((s) => {
  setLang(s.lang);
  nest.setZeca(s.zeca);
  nest.setSound(s.sounds);
  nest.setSpecies(s.zecaSpecies);
  nest.chat.setNative(s.nativeChat);
  // Asked for the Chat before Zeca was known to be on.
  // Not if the user went elsewhere meanwhile.
  if (s.zeca && asked === "chat" && nest.current() === "flock") nest.open("chat");
});
void Bridge.apiKeyStatus().then((api) => nest.chat.setApi(api));
Bridge.onSettings((s) => {
  if (s.lang !== undefined) {
    setLang(s.lang);
    nest.relabel();
  }
  if (s.zeca !== undefined) nest.setZeca(s.zeca);
  if (s.sounds !== undefined) nest.setSound(s.sounds);
  if (s.zecaSpecies !== undefined) nest.setSpecies(s.zecaSpecies);
  if (s.api !== undefined) nest.chat.setApi(s.api);
  if (s.nativeChat !== undefined) nest.chat.setNative(s.nativeChat);
});
// Opens on the conversation the chat is in.
void Bridge.chatCurrent().then(
  (c) => c && nest.chat.load(c),
  () => {},
);
