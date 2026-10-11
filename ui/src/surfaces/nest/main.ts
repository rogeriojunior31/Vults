// The Nest: Zeca's own window, built when opened and gone when closed (ADR 0008, D6). Its Chat tab
// is the island's conversation in full Markdown: the same chat, so a message sent in either shows in
// both. Cards show here, but only the island answers them. Its Memory tab is where the user says
// yes to what Zeca would remember (M6); its Roosts tab groups the flock by repository (M3).
import { Bridge } from "../../bridge";
import { setLang } from "../../i18n";
import { createNest } from "./render";

/** The flock's folders now, for a new task's roost. */
let folders: { cwd: string; project: string }[] = [];

const { chat, flockMoved, relabel, memoryChanged, tasksChanged } = createNest(
  document.getElementById("nest")!,
  {
    send: (text, files, folder) => Bridge.chatSend(text, files, folder),
    // Never called: the Nest's cards have no Allow or Deny.
    decide: () => {},
    stop: () => void Bridge.chatStop(),
    reset: (provider) => Bridge.chatReset(provider),
    keyboard: () => {},
    models: (provider) => Bridge.chatModels(provider),
    pick: (folder) => Bridge.chatPick(folder),
    setPick: (folder, provider, pick) => Bridge.setChatPick(folder, provider, pick),
    conversations: () => Bridge.chatConversations(),
    open: (id) => Bridge.chatOpen(id),
    rename: (id, title) => Bridge.chatRename(id, title),
    remove: (id) => Bridge.chatDelete(id),
    memory: (text, folder) => Bridge.memoryCommand(text, folder),
    task: (text, folder) => Bridge.taskCommand(text, folder),
  },
  () => void Bridge.chatToIsland(),
  {
    load: () => Bridge.roosts(),
    preview: (key) => Bridge.roostSharePreview(key),
    apply: (key, file, fingerprint) => Bridge.roostShareApply(key, file, fingerprint),
  },
  {
    load: () => Bridge.memoryView(),
    decide: (id, accept) => Bridge.memoryDecide(id, accept),
    undo: (id) => Bridge.memoryUndo(id),
  },
  (fresh) => Bridge.agentsDoctor(fresh),
  {
    load: (closed) => Bridge.tasksList(closed),
    create: (goal, criteria, folder) => Bridge.taskCreate(goal, criteria, folder),
    edit: (id, edit) => Bridge.taskEdit(id, edit),
    remove: (id) => Bridge.taskDelete(id),
    openIssue: (id) => Bridge.taskOpenIssue(id),
    visit: (id) => Bridge.taskVisitIssue(id),
    folders: () => folders,
    start: (id, engine) => Bridge.taskStart(id, engine),
    startFresh: (id, engine) => Bridge.taskStartFresh(id, engine),
    stop: (id) => Bridge.taskStop(id),
    removeWorktree: (id) => Bridge.taskRemoveWorktree(id),
  },
);
Bridge.onMemory(() => memoryChanged());
Bridge.onTasks(() => tasksChanged());

Bridge.onChat((d) => chat.receive(d));
Bridge.onChatSync((s) => chat.sync(s));
// The sessions' folders, for a new conversation (the view comes now, then with each change).
Bridge.onView((v) => {
  folders = v.sessions.flatMap((s) => (s.cwd ? [{ cwd: s.cwd, project: s.project }] : []));
  chat.setFolders(folders, null);
  flockMoved();
});
void Bridge.appSettings().then((s) => {
  setLang(s.lang);
  relabel();
  chat.setNative(s.nativeChat);
});
void Bridge.apiKeyStatus().then((api) => chat.setApi(api));
Bridge.onSettings((s) => {
  if (s.lang !== undefined) {
    setLang(s.lang);
    relabel();
  }
  if (s.api !== undefined) chat.setApi(s.api);
  if (s.nativeChat !== undefined) chat.setNative(s.nativeChat);
});
// Opens on the conversation the chat is in.
void Bridge.chatCurrent().then((c) => c && chat.load(c), () => {});
