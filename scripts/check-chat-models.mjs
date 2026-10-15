// Check the production Chat panel's failed model reads without a browser server.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";

class Element {
  children = [];
  listeners = {};
  attributes = {};
  dataset = {};
  className = "";
  style = {};
  value = "";
  classList = {
    contains: name => this.className.split(" ").includes(name),
    add: name => { if (!this.classList.contains(name)) this.className += ` ${name}`; },
    remove: name => { this.className = this.className.split(" ").filter(value => value !== name).join(" "); },
    toggle: (name, on = !this.classList.contains(name)) => this.classList[on ? "add" : "remove"](name),
  };
  constructor(tag) { this.tag = tag; this.tagName = tag.toUpperCase(); }
  set textContent(text) { this.text = text; this.children = []; }
  get textContent() { return (this.text ?? "") + this.children.map(node => node.textContent).join(""); }
  get childNodes() { return this.children; }
  contains(node) { return nodes(this).includes(node); }
  matches(selector) { return selector.split(",").some(value => value.trim().startsWith(".") ? this.classList.contains(value.trim().slice(1)) : this.tag === value.trim()); }
  querySelectorAll(selector) { return nodes(this).slice(1).filter(node => node.matches(selector)); }
  click() { if (!this.disabled) this.listeners.click?.({ target: this }); }
  setSelectionRange(start, end) { this.selectionStart = start; this.selectionEnd = end; }
  querySelector(tag) { return nodes(this).find(node => node.tag === tag) ?? null; }
  append(...nodes) { for (const node of nodes) node.parentElement = this; this.children.push(...nodes); }
  prepend(...nodes) { for (const node of nodes) node.parentElement = this; this.children.unshift(...nodes); }
  replaceChildren(...nodes) { this.text = ""; this.children = []; this.append(...nodes); }
  replaceWith(node) { this.parentElement.children.splice(this.parentElement.children.indexOf(this), 1, node); node.parentElement = this.parentElement; }
  addEventListener(event, fn) { this.listeners[event] = fn; }
  setAttribute(name, value) { this.attributes[name] = value; }
  toggleAttribute() {}
  focus() { document.activeElement = this; }
  scrollTo() {}
  getBoundingClientRect() { return { height: 200 }; }
  getContext() { return { clearRect() {}, fillRect() {} }; }
  toDataURL() { return "data:image/png;base64,"; }
}
const documentEvents = {};
globalThis.document = { body: new Element("body"), addEventListener: (event, fn) => (documentEvents[event] ??= []).push(fn), createElement: tag => new Element(tag), createElementNS: (_ns, tag) => new Element(tag),
  createDocumentFragment: () => new Element("fragment"),
  createTextNode: text => { const node = new Element("text"); node.textContent = text; return node; } };
globalThis.window = globalThis;
globalThis.setTimeout = globalThis.setInterval = () => 0;
globalThis.requestAnimationFrame = () => 0;
globalThis.getComputedStyle = () => ({ lineHeight: "20", borderTopWidth: "1", borderBottomWidth: "1" });
const root = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({ configFile: false, root, logLevel: "silent", build: {
  write: false, minify: false, lib: { entry: `${root}src/island/chat.ts`, formats: ["es"] },
} });
const chunk = bundles[0].output.find(part => part.type === "chunk");
const { ChatPanel } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
const nodes = element => [element, ...element.children.flatMap(nodes)];
const tick = () => new Promise(resolve => setImmediate(resolve));
let calls = 0, fail = true, sends = 0;
const panel = new ChatPanel({
  models: () => {
    calls++;
    assert.ok(calls <= 3, "A failed model read must not trigger an automatic retry loop");
    return fail ? Promise.reject(new Error("Models unavailable."))
      : Promise.resolve([{ id: "sonnet", label: "Sonnet", efforts: [] }]);
  },
  pick: async () => ({}),
  send: async () => { sends++; },
}, () => {});
await tick();
assert.equal(calls, 1);
const input = nodes(panel.element).find(node => node.tag === "textarea");
input.value = "Keep this draft";
input.listeners.input();
panel.relabel();
await tick();
assert.equal(calls, 1, "Typing and repainting must not repeat a failed read");
const picker = nodes(panel.element).find(node => node.className.split(" ").includes("model"));
assert.ok(picker, "The model picker must be accessible");
assert.equal(picker.attributes["aria-expanded"], "false");
assert.equal(nodes(picker).find(node => node.className === "engine-labels").textContent, "Claude CodeDefault model");
assert.equal(picker.attributes["aria-label"], "Claude Code, Default model", "The compact visual grouping retains the full accessible selection");
picker.listeners.click();
assert.equal(picker.attributes["aria-expanded"], "true");
await tick();
assert.equal(calls, 2, "Explicitly opening the picker retries once");
assert.ok(panel.element.textContent.includes("did not list its models"));
picker.listeners.click();
assert.equal(picker.attributes["aria-expanded"], "false");
fail = false;
picker.listeners.click();
await tick();
assert.equal(calls, 3);
assert.ok(panel.element.textContent.includes("Sonnet"));
assert.ok(!panel.element.textContent.includes("did not list its models"));
panel.relabel();
await tick();
assert.equal(calls, 3, "A successful read is reused");
assert.equal(input.value, "Keep this draft");
assert.equal(sends, 0);
console.log("check-chat-models: ok (failed read settles, repaint without retry, explicit retry, recovery, draft retained, no send)");

const kept = { text: "Saved before opening", files: ["/work/note.txt"], folder: "/work", provider: "codex" };
const conversation = { provider: "codex", label: "Codex", folder: "/work", started: true,
  model: "gpt", effort: "high", lines: [], busy: false, partial: null, waiting: [] };
let reads = 0, readFails = true, currentFails = false;
const writes = [];
const restoring = new ChatPanel({
  models: async () => [], pick: async () => ({}),
  current: async () => { if (currentFails) throw new Error("Conversation unavailable."); return conversation; },
  draft: async () => { reads++; if (readFails) throw new Error("Draft unavailable."); return kept; },
  saveDraft: async draft => { writes.push(draft); },
}, () => {}, { window: "nest" });
await tick();
await assert.rejects(restoring.flushDraft(), /Draft unavailable/);
assert.deepEqual(writes, [], "A failed initial read must never save an empty draft");
assert.equal(reads, 2, "A save retries the failed startup read once, with no automatic loop");
readFails = false; currentFails = true;
await assert.rejects(restoring.flushDraft(), /Conversation unavailable/);
assert.deepEqual(writes, [], "A failed conversation read must also block saving");
currentFails = false;
await restoring.flushDraft();
assert.deepEqual(writes, [kept], "Retry restores text, files and bound source before saving");
await restoring.flushDraft();
assert.equal(reads, 4, "Successful restoration is reused");
console.log("check-chat-models: initial draft/conversation failure, no empty write, explicit retry, source/attachments retained");

for (const surface of ["island", "nest"]) {
  const from = surface === "nest" ? "island" : "nest";
  let releaseCurrent;
  const raceWrites = [], actions = [];
  const racing = new ChatPanel({
    models: async () => [], pick: async () => ({}),
    current: () => new Promise(resolve => { releaseCurrent = resolve; }),
    draft: async () => kept,
    saveDraft: async draft => { raceWrites.push(draft); },
    decide: () => actions.push("decide"), send: async () => actions.push("send"),
  }, () => {}, { window: surface });
  const area = nodes(racing.element).find(node => node.tag === "textarea");
  area.value = "Typing ahead"; area.listeners.input();
  racing.sync({ kind: "sent", from, text: "Live turn", files: [], provider: "claude", folder: "/new",
    model: "sonnet", effort: "high" });
  racing.receive({ kind: "permission", id: "live-card", tool: "Bash", target: "git status",
    description: null, full: null, added: 0, removed: 0, always: false, outside: null });
  releaseCurrent(conversation);
  await racing.flushDraft();
  assert.equal(racing.isBusy(), true, "A stale initial conversation cannot end a live turn");
  assert.ok(racing.element.textContent.includes("git status"), "A live card cannot disappear behind a late initial read");
  assert.ok(racing.element.textContent.includes("Live turn"));
  assert.equal(area.value, "Typing ahead");
  assert.deepEqual(raceWrites.at(-1), { text: "Typing ahead", files: [], folder: "/new", provider: "claude" });
  assert.deepEqual(actions, [], "Restoration never sends or answers a card");

  for (const kind of ["opened", "reset", "done"]) {
    let release;
    const newer = new ChatPanel({ models: async () => [], pick: async () => ({}),
      current: () => new Promise(resolve => { release = resolve; }), draft: async () => kept,
      saveDraft: async () => {},
    }, () => {}, { window: surface });
    if (kind === "opened") newer.sync({ kind, from,
      conversation: { ...conversation, provider: "claude", folder: "/new", lines: [{ who: "you", text: "Newer conversation", files: [] }] } });
    else if (kind === "reset") newer.sync({ kind, from, provider: "claude" });
    else newer.receive({ kind });
    release({ ...conversation, busy: true, waiting: [{ id: "stale-card", tool: "Bash", target: "Stale command", outside: null }] });
    await newer.restore();
    assert.equal(newer.isBusy(), false, `A late read must not resurrect a turn after ${kind}`);
    assert.ok(!newer.element.textContent.includes("Stale command"));
    if (kind === "opened") assert.ok(newer.element.textContent.includes("Newer conversation"));
    if (kind !== "done") assert.equal(newer.agent(), "claude");
  }

  let releaseSource;
  const sameOwner = new ChatPanel({ models: async () => [], pick: async () => ({}),
    current: () => new Promise(resolve => { releaseSource = resolve; }), draft: async () => kept,
    saveDraft: async draft => { raceWrites.push(draft); },
  }, () => {}, { window: surface });
  sameOwner.receive({ kind: "permission", id: "current-card", tool: "Bash", target: "Current command",
    description: null, full: null, added: 0, removed: 0, always: false, outside: null });
  releaseSource({ ...conversation, busy: true, lines: [{ who: "you", text: "Earlier question", files: [] }] });
  await sameOwner.flushDraft();
  assert.equal(sameOwner.agent(), "codex", "A live event in the same conversation must still restore its executor");
  assert.equal(sameOwner.isBusy(), true);
  assert.ok(sameOwner.element.textContent.includes("Current command"));
  assert.ok(sameOwner.element.textContent.includes("Earlier question"), "The current owner's saved history stays available too");
  assert.deepEqual(raceWrites.at(-1), kept, "The current run retains its bound source and draft");
}
console.log("check-chat-models: late initial conversation retains live turn, pending card, newer draft/source in both surfaces");

for (const surface of ["island", "nest"]) {
  const decisions = [];
  const card = new ChatPanel({ models: async () => [], pick: async () => ({}), keyboard() {},
    decide: (id, answer) => decisions.push([id, answer]),
  }, () => {}, { window: surface });
  card.element.isConnected = true; card.element.offsetParent = {};
  card.toggle(true);
  card.receive({ kind: "permission", id: "visible-only", tool: "Bash", target: "git status",
    description: null, full: null, added: 0, removed: 0, always: false, outside: null });
  document.hidden = true;
  assert.equal(card.answerWaiting(true), false, "A delayed shortcut cannot answer in a minimized/hidden surface");
  document.hidden = false; card.element.offsetParent = null;
  assert.equal(card.answerWaiting(true), false, "A card in a hidden tab cannot take a shortcut");
  card.element.offsetParent = {}; card.element.isConnected = false;
  assert.equal(card.answerWaiting(true), false, "A destroyed surface cannot take a shortcut");
  card.element.isConnected = true; card.setEnabled(false);
  assert.equal(card.answerWaiting(true), false);
  assert.deepEqual(decisions, []);
  card.setEnabled(true); card.toggle(true);
  assert.equal(card.answerWaiting(false), true);
  assert.equal(card.answerWaiting(false), false);
  assert.deepEqual(decisions, [["visible-only", "deny"]], "One explicit decision answers the visible card once");
}
console.log("check-chat-models: hidden/minimized/detached/disabled card shortcuts rejected, visible decision once");

// Real ChatPanel events: composition must not send, and stream updates must respect reading.
const realNow = performance.now.bind(performance);
const inertTimeout = window.setTimeout;
let now = 1000;
performance.now = () => now;
try {
  for (const surface of ["island", "nest"]) {
    let sent = 0;
    const panel = new ChatPanel({ models: async () => [], pick: async () => ({}),
      send: async () => { sent++; },
    }, () => {}, { window: surface });
    await tick();
    const all = nodes(panel.element);
    const input = all.find(node => node.tag === "textarea");
    const log = all.find(node => node.className === "log");
    const jump = all.find(node => node.className === "jump-end");
    input.value = "Keep composing";
    for (const event of [{ isComposing: true }, { keyCode: 229 }, { shiftKey: true }]) {
      input.listeners.keydown({ key: "Enter", preventDefault() { assert.fail("Composition or Shift+Enter must stay native"); }, ...event });
      await tick();
      assert.equal(sent, 0);
      assert.equal(input.value, "Keep composing");
    }
    assert.equal(input.attributes["aria-label"], "Ask Zeca…");
    assert.equal(log.tabIndex, 0, "The transcript must support keyboard scrolling");
    input.listeners.keydown({ key: "Enter", preventDefault() {} });
    await tick();
    assert.equal(sent, 1, "Plain Enter still sends once");
    panel.receive({ kind: "done" });
    log.scrollHeight = 1000; log.clientHeight = 200; log.scrollTop = 800;
    panel.receive({ kind: "text", text: "A reply" });
    assert.equal(log.scrollTop, 1000, "New text follows the end while pinned");

    for (const [event, detail] of [["wheel", {}], ["touchmove", {}], ["pointerdown", {}], ["keydown", { key: "PageUp" }]]) {
      const queued = [];
      window.setTimeout = (fn, ms) => { queued.push({ fn, ms }); return queued.length; };
      log.scrollTop = 800; log.listeners.scroll();
      panel.receive({ kind: "text", text: " More" });
      const repaint = queued.find(timer => timer.ms === 80);
      assert.ok(repaint, "A stream repaint must be pending before manual input");
      log.listeners[event](detail);
      log.listeners.scroll(); // A near-bottom scroll event can arrive during the cooldown.
      now += 100;
      repaint.fn();
      assert.equal(log.scrollTop, 800, `${event}: a queued repaint cannot steal the viewport during the cooldown`);
      now += 51;
      panel.receive({ kind: "tool", tool: "Read" });
      assert.equal(log.scrollTop, 1000, `${event}: follow resumes at the end after the cooldown`);

      log.listeners[event](detail);
      log.scrollTop = 600; log.listeners.scroll();
      now += 1000;
      panel.receive({ kind: "tool", tool: "Read" });
      assert.equal(log.scrollTop, 600, "Reading older content stays unpinned after the cooldown");
      jump.listeners.click();
      assert.equal(log.scrollTop, 1000, "The explicit jump always returns to the latest content");
      panel.receive({ kind: "text", text: "Next reply" });
    }
    window.setTimeout = inertTimeout;
  }
} finally {
  performance.now = realNow;
  window.setTimeout = inertTimeout;
}
console.log("check-chat-models: both surfaces retain IME/Shift+Enter drafts, plain Enter, keyboard transcript, queued stream cooldown, manual reading and explicit jump");

for (const surface of ["island", "nest"]) {
  const scheduled = [];
  const timeout = window.setTimeout;
  window.setTimeout = (fn, ms) => { scheduled.push({ fn, ms }); return scheduled.length; };
  try {
    const chat = new ChatPanel({ models: async () => [], pick: async () => ({}), keyboard() {} }, () => {}, { window: surface });
    await tick();
    chat.receive({ kind: "text", text: "New reply" });
    const log = nodes(chat.element).find(node => node.classList.contains("log"));
    const bubble = log.children[0];
    assert.ok(bubble.classList.contains("message-enter"));
    chat.receive({ kind: "text", text: " keeps streaming" });
    scheduled.find(timer => timer.ms === 80).fn();
    assert.equal(log.children[0], bubble, "A stream update retains the animated bubble itself");
    assert.ok(bubble.textContent.includes("keeps streaming"));
    chat.relabel();
    assert.ok(!log.children[0].classList.contains("message-enter"), "A repaint never replays old message entrances");
    chat.attach(["/inbox/1-note.txt"]);
    const chips = nodes(chat.element).find(node => node.className === "chips");
    const file = chips.children[0];
    assert.ok(file.classList.contains("attachment-enter"));
    assert.equal(file.querySelector("button").attributes["aria-label"], "Remove note.txt");
    chat.receive({ kind: "tool", tool: "Read" });
    assert.equal(chips.children[0], file, "Stream repaint retains the attachment's node and animation");
    file.querySelector("button").listeners.click();
    assert.equal(chips.children.length, 0);
    chat.load({ ...conversation, lines: [{ who: "you", text: "Saved question", files: [] }, { who: "zeca", text: "Saved answer" }], busy: false });
    assert.ok(log.children.every(node => !node.classList.contains("message-enter")), "Restoring history never animates old messages as new");
  } finally { window.setTimeout = timeout; }
}
console.log("check-chat-models: live message entrance once, stable streaming bubble/attachment, named removal, saved history without replay in both surfaces");

for (const surface of ["island", "nest"]) {
  const picked = [], drafts = [];
  let sends = 0;
  const chat = new ChatPanel({ models: async () => [], pick: async () => ({}),
    keyboard() {},
    pickFiles: title => picked.push(title), saveDraft: async draft => { drafts.push(draft); },
    send: async () => { sends++; },
  }, () => {}, { window: surface });
  await tick();
  const all = nodes(chat.element);
  const input = all.find(node => node.tag === "textarea");
  const composer = all.find(node => node.className === "composer");
  const tools = all.find(node => node.className === "composer-tools");
  const suggestion = all.find(node => node.className === "suggestion");
  if (surface === "nest") {
    assert.ok(suggestion, "Room keeps its editable suggested prompts");
    suggestion.listeners.click();
  } else {
    assert.equal(suggestion, undefined, "Quick leaves suggested prompts to Room");
    chat.draft("Question about this project");
  }
  const draft = input.value;
  assert.ok(draft.includes("project"));
  assert.equal(document.activeElement, input);
  assert.equal(sends, 0, "A suggestion only prepares the editable draft");
  tools.children[0].listeners.click();
  assert.deepEqual(picked, ["Attach a file"]);
  chat.attach(["/inbox/123-project.plan.md", "/work/.gitignore", "/work/README", "/work/design.review.png"]);
  const files = nodes(composer).find(node => node.className === "chips");
  assert.equal(input.parentElement, composer);
  assert.equal(files.parentElement, composer, "File cards belong to the same draft surface");
  assert.equal(files.attributes["aria-label"], "Attachments");
  assert.equal(tools.parentElement, composer);
  assert.equal(tools.children[0].attributes["aria-label"], "Attach a file", "The icon-only file picker keeps its explicit accessible name");
  assert.deepEqual(files.children.map(file => file.querySelector("small").textContent), ["MD", "File", "File", "PNG"]);
  assert.ok(files.children[0].textContent.includes("project.plan.md"), "Inbox timestamps stay out of the visible name");
  files.children[1].querySelector("button").listeners.click();
  await chat.flushDraft();
  assert.equal(input.value, draft, "Removing one file keeps the text and other files");
  assert.deepEqual(drafts.at(-1).files, ["/inbox/123-project.plan.md", "/work/README", "/work/design.review.png"]);
  assert.equal(tools.children.at(-1).attributes["aria-label"], "Send (Enter)");
  assert.equal(sends, 0, "Choosing or removing files never sends the draft");
}
console.log("check-chat-models: Room-only editable suggestions, both file pickers, real file labels, individual removal and saved draft");

for (const surface of ["island", "nest"]) {
  const chat = new ChatPanel({ models: async () => [], pick: async () => ({}) }, () => {}, { window: surface });
  await tick();
  const greetings = () => nodes(chat.element).filter(node => node.className === "chat-greeting");
  const engine = nodes(chat.element).find(node => node.className === "model-box engine-box");
  assert.equal(engine.parentElement.className, surface === "island" ? "chat-foot" : "chat-head", "Only Quick moves its executor beside the draft; Room keeps its existing header");
  assert.equal(greetings().length, 1, "The recipient is introduced once");
  assert.equal(greetings()[0].parentElement.className, "hint", "The greeting belongs to the empty state, never the persistent header");
  assert.equal(chat.element.classList.contains("has-messages"), false, "An empty chat has no message-state marker");
  chat.load({ ...conversation, busy: true, lines: [] });
  assert.equal(chat.element.classList.contains("has-messages"), true, "Thinking already reserves the conversation space");
  chat.load({ ...conversation, provider: "api", started: false, busy: false, lines: [] });
  assert.equal(greetings().length, 1, "The API notice does not duplicate the recipient");
  assert.ok(chat.element.textContent.includes("can't run commands or open your project"), "The API capability notice stays visible");
  chat.load({ ...conversation, lines: [{ who: "zeca", text: "Saved reply" }] });
  assert.ok(!chat.element.textContent.includes("can't run commands or open your project"));
  assert.equal(greetings().length, 0, "Both surfaces leave the reading area to messages once the conversation starts");
  assert.ok(chat.element.textContent.includes("Saved reply"));
  const trigger = nodes(engine).find(node => node.classList.contains("model"));
  trigger.listeners.click();
  assert.equal(trigger.title === "", surface === "island", "Quick explains a pinned model inside its menu, without a hovering tooltip blocking choices");
  assert.ok(nodes(engine).some(node => node.textContent.includes("A conversation keeps its model")) === (surface === "island"));
  assert.equal(chat.element.classList.contains("has-messages"), true, "Restored messages get the full conversation height");
}
console.log("check-chat-models: both surfaces keep the greeting in the empty state only, API capability notice survives");

for (const surface of ["island", "nest"]) {
  const chat = new ChatPanel({ models: async () => [], pick: async () => ({}), keyboard() {} }, () => {}, { window: surface });
  await tick();
  chat.load({ ...conversation, busy: true, lines: [{ who: "zeca", text: "A reply already streaming" }] });
  assert.equal(chat.clip(performance.now()), "think", "Thinking continues through streamed words, not only before the first one");
  chat.receive({ kind: "permission", id: "expression-only", tool: "Bash", target: "git status", description: null, full: null, added: 0, removed: 0, always: false, outside: null });
  chat.showVoice("listening");
  assert.equal(chat.clip(performance.now()), "question", "A pending decision keeps the question pose");
  chat.receive({ kind: "stopped" });
  assert.equal(chat.clip(performance.now()), "listen");
  chat.showVoice("transcribing");
  assert.equal(chat.clip(performance.now()), "think");
  chat.showVoice("off");
  assert.equal(chat.clip(performance.now()), "idle");
  chat.attach(["/inbox/note.txt"]);
  assert.equal(chat.clip(0), "swallow");
  assert.equal(chat.clip(Infinity), "idle");
}
console.log("check-chat-models: both companions retain streaming thought, decision priority, voice, transcription and attachment expressions");

// Load Room's real entry point: a working ChatPanel alone cannot catch missing native wiring.
const voiceEvents = {}, voiceCalls = [], voiceDrafts = [];
const voiceRoot = new Element("main");
document.getElementById = () => voiceRoot;
document.documentElement = new Element("html");
globalThis.location = { hash: "#chat" };
globalThis.voiceTestCreateNest = (_root, backend) => {
  const chat = new ChatPanel(backend.chat, () => {}, { window: "nest" });
  const noop = () => {};
  return globalThis.voiceTestNest = { chat, current: () => "chat", open: noop, setFull: noop,
    setZeca: on => chat.setEnabled(on), setSound: noop, setSpecies: noop, setKeys: noop, setMedia: noop,
    memoryChanged: noop, usageChanged: noop, tasksChanged: noop, relabel: noop, view: noop, shortcut: noop };
};
globalThis.voiceTestBridge = new Proxy({}, { get: (_target, name) => {
  if (name.startsWith("on")) return cb => { voiceEvents[name] = cb; return Promise.resolve(); };
  return async (...args) => {
    switch (name) {
      case "voiceStatus": return { ready: true };
      case "voiceStart": voiceCalls.push(["start", ...args]); return;
      case "voiceStop": voiceCalls.push(["stop"]); return "Spoken words";
      case "voiceCancel": voiceCalls.push(["cancel"]); return;
      case "chatSend": assert.fail("Dictation never sends a message");
      case "chatDraftSet": voiceDrafts.push(args[0]); return;
      case "chatDraft": return { text: "Existing draft", files: [], folder: "/work", provider: "claude" };
      case "chatCurrent": return null;
      case "chatModels": case "taskAgents": return [];
      case "chatPick": case "shortcutKeys": return {};
      case "apiKeyStatus": return { ready: false, label: "API" };
      case "appSettings": return { lang: "en", zeca: true };
      default: return;
    }
  };
} });
const entryBundles = await build({ configFile: false, root, logLevel: "silent",
  plugins: [{ name: "room-voice-native-mock", enforce: "pre", resolveId(id, importer) {
    if (/\/bridge(?:\.ts)?$/.test(id)) return "\0voice-bridge";
    if (id === "./render" && importer?.endsWith("/nest/main.ts")) return "\0voice-nest";
  }, load(id) {
    if (id === "\0voice-bridge") return "export const Bridge = globalThis.voiceTestBridge;";
    if (id === "\0voice-nest") return "export const createNest = globalThis.voiceTestCreateNest;";
  } }], build: { write: false, minify: false, lib: { entry: `${root}src/surfaces/nest/main.ts`, formats: ["es"] } },
});
const entryChunk = entryBundles[0].output.find(part => part.type === "chunk");
await import(`data:text/javascript;base64,${Buffer.from(entryChunk.code).toString("base64")}`);
await tick();
const roomChat = globalThis.voiceTestNest.chat;
const roomMic = nodes(roomChat.element).find(node => node.classList.contains("mic"));
const roomInput = nodes(roomChat.element).find(node => node.tag === "textarea");
assert.equal(roomMic.hidden, false, "A configured microphone is visible in Room after its native status read");
roomMic.listeners.click(); await tick();
assert.deepEqual(voiceCalls, [["start", true]], "Room starts its real native tap-to-talk binding");
voiceEvents.onVoiceLevel(0.8);
voiceEvents.onVoicePartial("Words so far");
assert.ok(roomChat.element.textContent.includes("Words so far"));
assert.equal(roomInput.hidden, true);
voiceEvents.onVoiceSilence(); await tick();
assert.deepEqual(voiceCalls.at(-1), ["stop"], "Silence stops the Room recording");
assert.equal(roomInput.value, "Existing draft Spoken words");
assert.equal(roomInput.hidden, false);
await roomChat.flushDraft();
assert.equal(voiceDrafts.at(-1).text, roomInput.value, "Transcription remains a saved editable draft");
roomMic.listeners.click(); await tick();
voiceEvents.onSettings({ voice: false });
assert.equal(roomMic.hidden, true);
assert.deepEqual(voiceCalls.at(-1), ["cancel"], "Turning voice off cancels Room's recording");
voiceEvents.onSettings({ voice: true });
assert.equal(roomMic.hidden, false);
console.log("check-chat-models: real Room entry wires voice status/settings, microphone, preview, silence, transcription draft and cancellation");

// Rich replies keep links inert, expose local scrolling and copy failures without touching drafts.
let clipboardCalls = 0, failClipboard = true;
Object.defineProperty(globalThis, "navigator", { configurable: true, value: { clipboard: { writeText: async () => {
  clipboardCalls++; if (failClipboard) throw new Error("Clipboard unavailable");
} } } });
for (const surface of ["island", "nest"]) {
  const chat = new ChatPanel({ models: async () => [], pick: async () => ({}) }, () => {}, { window: surface });
  await tick();
  chat.draft("Keep my next question");
  const body = "# Result\n\n| File | Status |\n| --- | --- |\n| app.ts | ready |\n\n```ts\nconst value = 1;\n```\n\n[Link](https://example.test) <script>never execute</script>";
  chat.load({ ...conversation, busy: false, lines: [{ who: "zeca", text: body }] });
  const rendered = nodes(chat.element);
  assert.ok(rendered.some(n => n.tag === "h3")); assert.ok(rendered.some(n => n.tag === "table"));
  assert.ok(!rendered.some(n => n.tag === "a" || n.tag === "script"));
  const table = rendered.find(n => n.className === "table");
  assert.equal(table.tabIndex, 0); assert.equal(table.attributes["aria-label"], "Table");
  const code = rendered.find(n => n.tag === "pre");
  assert.equal(code.tabIndex, 0); assert.equal(code.attributes["aria-label"], "Code in ts");
  const copy = rendered.find(n => n.className === "copy");
  assert.equal(copy.attributes["aria-label"], "Copy code");
  failClipboard = true; const before = clipboardCalls;
  copy.listeners.click(); copy.listeners.click(); await tick();
  assert.equal(clipboardCalls, before + 1); assert.ok(copy.textContent.includes("Copy failed"));
  failClipboard = false; copy.listeners.click(); await tick(); assert.ok(copy.textContent.includes("Copied"));
  assert.equal(nodes(chat.element).find(n => n.tag === "textarea").value, "Keep my next question");
  chat.receive({ kind: "permission", id: "layout-scope", tool: "Bash", target: "rm /work/old", description: "Remove old output", full: "rm /work/old", added: 0, removed: 0, always: false, outside: "Web" });
  const permission = nodes(chat.element).find(n => n.className === "ask");
  assert.ok(permission.textContent.includes("Permission required"));
  assert.ok(permission.textContent.includes("Folder")); assert.ok(permission.textContent.includes("rm /work/old"));
  assert.ok(permission.textContent.includes("content from outside"));
  const deny = nodes(permission).find(n => n.tag === "button" && n.textContent === "Deny");
  assert.ok(deny.className.includes("primary"));
  assert.ok(!permission.textContent.includes("Always in this chat"));
}
console.log("check-chat-models: rich replies in both surfaces, inert links/HTML, named scrolling, copy failure/retry, visible permission scope and cautious primary action");

// Selectors navigate without choosing; asynchronous repaints retain the actual focused choice.
for (const surface of ["island", "nest"]) {
  let resolveModels, menuActions = 0;
  const menuChat = new ChatPanel({
    models: () => new Promise(resolve => { resolveModels = resolve; }), pick: async () => ({}),
    conversations: async () => [{ id: 42, title: "Saved question", folder: "/work/alpha", label: "Claude Code", current: false }],
    send: async () => { menuActions++; }, decide: () => { menuActions++; },
    setPick: async () => { menuActions++; }, setAccess: async () => { menuActions++; },
    reset: async () => { menuActions++; return "claude"; }, openSettings: () => { menuActions++; },
  }, () => {}, { window: surface });
  menuChat.element.isConnected = true;
  menuChat.setFolders([{ cwd: "/work/alpha", project: "alpha" }, { cwd: "/work/beta", project: "beta" }], "/work/alpha");
  menuChat.draft("Next question stays here"); await tick();
  const menuFor = suffix => nodes(menuChat.element).find(n => n.id?.endsWith(`-${suffix}`));
  const triggerFor = suffix => nodes(menuChat.element).find(n => n.attributes["aria-controls"] === menuFor(suffix).id);
  const pressed = key => ({ key, target: document.activeElement, preventDefault() { this.prevented = true; }, stopPropagation() { this.stopped = true; } });
  const modelTrigger = triggerFor("model"), folderTrigger = triggerFor("folder"), historyTrigger = triggerFor("history"), accessTrigger = triggerFor("access");
  const transcript = nodes(menuChat.element).find(n => n.className === "log");
  transcript.scrollTop = 96;
  const transcriptContent = [...transcript.children];
  modelTrigger.focus(); const down = pressed("ArrowDown"); modelTrigger.listeners.keydown(down); await tick();
  const engineChoice = document.activeElement;
  assert.equal(engineChoice.dataset.choice, "engine:claude"); assert.equal(down.prevented, true);
  resolveModels([{ id: "sonnet", label: "Sonnet", efforts: [] }]); await tick();
  assert.equal(document.activeElement.dataset.choice, "engine:claude");
  assert.notEqual(document.activeElement, engineChoice, "A replaced menu row regains focus after the model read");
  assert.deepEqual(transcript.children, transcriptContent, "Opening and loading the model selector never rebuilds the transcript");
  const modelMenu = menuFor("model"); modelMenu.listeners.keydown(pressed("End"));
  assert.equal(document.activeElement, modelMenu.querySelectorAll("button").at(-1));
  modelMenu.listeners.keydown(pressed("Home")); assert.equal(document.activeElement.dataset.choice, "engine:claude");
  accessTrigger.click(); assert.equal(modelTrigger.attributes["aria-expanded"], "false");
  assert.equal(accessTrigger.attributes["aria-expanded"], "true");
  assert.ok(menuFor("access").querySelectorAll("button").length >= 3);
  folderTrigger.click(); assert.equal(accessTrigger.attributes["aria-expanded"], "false");
  assert.equal(folderTrigger.attributes["aria-expanded"], "true");
  historyTrigger.click(); await tick(); assert.equal(folderTrigger.attributes["aria-expanded"], "false");
  assert.equal(historyTrigger.attributes["aria-expanded"], "true");
  let history = menuFor("history"); history.querySelectorAll("button").find(n => n.dataset.choice === "rename-button:42").click(); await tick();
  let rename = menuFor("history").querySelector("input"); rename.value = "Unfinished title"; rename.setSelectionRange(4, 8); rename.focus();
  menuChat.relabel(); await tick(); rename = menuFor("history").querySelector("input");
  assert.equal(rename.value, "Unfinished title"); assert.equal(rename.selectionStart, 4); assert.equal(rename.selectionEnd, 8);
  const arrowInText = pressed("ArrowDown"); menuFor("history").listeners.keydown(arrowInText);
  assert.equal(arrowInText.prevented, undefined, "Editing a title retains normal text-key behavior");
  const composingEscape = { ...pressed("Escape"), isComposing: true }; menuChat.element.listeners.keydown(composingEscape);
  assert.equal(historyTrigger.attributes["aria-expanded"], "true");
  const escape = pressed("Escape"); menuChat.element.listeners.keydown(escape);
  assert.equal(escape.stopped, true, "Escape dismisses the selector before reaching the surface close handler");
  assert.equal(document.activeElement, historyTrigger); assert.equal(historyTrigger.attributes["aria-expanded"], "false");
  folderTrigger.click();
  const folderMenu = menuFor("folder"); const beta = folderMenu.querySelectorAll("button").find(n => n.dataset.choice === "folder:/work/beta");
  assert.equal(beta.title, "/work/beta"); beta.focus(); beta.click(); assert.equal(document.activeElement, folderTrigger);
  modelTrigger.click(); for (const listener of documentEvents.pointerdown) listener({ target: document.body });
  assert.equal(modelTrigger.attributes["aria-expanded"], "false");
  assert.equal(nodes(menuChat.element).find(n => n.tag === "textarea").value, "Next question stays here");
  assert.equal(transcript.scrollTop, 96, "Opening, loading and dismissing selectors retains the reading position");
  assert.equal(menuActions, 0, "Browsing, focusing and dismissing selectors never sends, grants access or opens Settings");
}
console.log("check-chat-models: both selectors retain focus/rename caret across reads, one open panel, keyboard/IME/Escape/outside dismissal and no implicit actions");
