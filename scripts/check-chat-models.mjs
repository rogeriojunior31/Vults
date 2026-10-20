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
  style = { setProperty(name, value) { this[name] = value; } };
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
  querySelector(selector) { return this.querySelectorAll(selector)[0] ?? null; }
  append(...nodes) { for (const node of nodes) node.parentElement = this; this.children.push(...nodes); }
  prepend(...nodes) { for (const node of nodes) node.parentElement = this; this.children.unshift(...nodes); }
  replaceChildren(...nodes) { this.text = ""; this.children = []; this.append(...nodes); }
  replaceWith(node) { this.parentElement.children.splice(this.parentElement.children.indexOf(this), 1, node); node.parentElement = this.parentElement; }
  addEventListener(event, fn) { this.listeners[event] = fn; }
  setAttribute(name, value) { this.attributes[name] = value; }
  getAttribute(name) { return this.attributes[name]; }
  toggleAttribute() {}
  focus() { document.activeElement = this; }
  blur() { if (document.activeElement === this) { document.activeElement = null; this.listeners.blur?.({ target: this }); } }
  scrollTo() {}
  getBoundingClientRect() { return { height: 200 }; }
  getContext() { return { clearRect() {}, fillRect() {} }; }
  toDataURL() { return "data:image/png;base64,"; }
}
const documentEvents = {};
globalThis.document = { body: new Element("body"), dispatchEvent() {}, addEventListener: (event, fn) => (documentEvents[event] ??= []).push(fn), createElement: tag => new Element(tag), createElementNS: (_ns, tag) => new Element(tag),
  createDocumentFragment: () => new Element("fragment"),
  createTextNode: text => { const node = new Element("text"); node.textContent = text; return node; } };
globalThis.window = globalThis;
globalThis.addEventListener = () => {};
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
  input.parentElement.children[0].listeners.click();
  assert.deepEqual(picked, ["Attach a file"]);
  chat.attach(["/inbox/123-project.plan.md", "/work/.gitignore", "/work/README", "/work/design.review.png"]);
  const files = nodes(composer).find(node => node.className === "chips");
  assert.equal(input.parentElement.className, "composer-row");
  assert.equal(files.parentElement, composer, "File cards belong to the same draft surface");
  assert.equal(files.attributes["aria-label"], "Attachments");
  assert.equal(tools.parentElement, input.parentElement, "Microphone and Send share the draft row");
  assert.equal(input.parentElement.children[0].attributes["aria-label"], "Attach a file", "The left attachment action retains its explicit accessible name");
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
  const greetings = () => nodes(chat.element).filter(node => node.className === "chat-greeting" && !node.parentElement.hidden);
  const engine = nodes(chat.element).find(node => node.className === "model-box engine-box");
  assert.equal(engine.parentElement.className, surface === "island" ? "chat-foot" : "chat-head", "Only Quick moves its executor beside the draft; Room keeps its existing header");
  assert.equal(greetings().length, 1, "The recipient is introduced once");
  assert.equal(greetings()[0].parentElement.parentElement.className, surface === "island" ? "chat-head" : "hint", "Quick introduces the empty conversation beside its companion; Room keeps its empty state");
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

{
  const chat = new ChatPanel({ models: async () => [], pick: async () => ({}) }, () => {});
  await tick();
  chat.element.isConnected = true;
  chat.element.clientHeight = 300;
  chat.element.querySelector(".chat-foot").offsetHeight = 40;
  const log = chat.element.querySelector(".log");
  let contentHeight = 80;
  Object.defineProperty(log, "scrollHeight", { get: () => log.style.height === "auto" ? contentHeight : Math.max(contentHeight, 200) });
  chat.fitContents();
  assert.equal(chat.element.style["--chat-content-height"], "0px", "An empty Quick never reserves a transcript viewport");
  chat.load({ ...conversation, lines: [{ who: "zeca", text: "Short reply" }] });
  log.scrollTop = 11; log.listeners.wheel();
  chat.fitContents();
  assert.equal(chat.element.style["--chat-content-height"], "80px", "Short content is measured independently of its previous viewport height");
  assert.equal(log.scrollTop, 11, "Sizing respects a manual reading position");
  assert.equal(chat.element.style["--chat-menu-height"], "244px", "The menu is limited by the current surface, not the maximum overlay size");
  const trigger = nodes(chat.element).find(node => node.classList.contains("model"));
  trigger.click(); await tick(); chat.fitContents();
  assert.equal(chat.element.style["--chat-content-height"], "80px", "Opening and loading model choices does not enlarge the chat");
  trigger.click(); chat.fitContents();
  assert.equal(chat.element.style["--chat-content-height"], "80px", "Closing the selector does not shrink the transcript");
  contentHeight = 4000; chat.fitContents();
  assert.equal(chat.element.style["--chat-content-height"], "4000px", "Long replies retain their intrinsic size for the bounded flex viewport");
  chat.load({ ...conversation, started: false, lines: [] }); chat.fitContents();
  assert.equal(chat.element.style["--chat-content-height"], "0px", "A new empty conversation releases the previous transcript height");
  const room = new ChatPanel({ models: async () => [], pick: async () => ({}) }, () => {}, { window: "nest" });
  room.element.isConnected = true; room.fitContents();
  assert.equal(room.element.style["--chat-content-height"], undefined, "Quick sizing never changes Room");
}
console.log("check-chat-models: Quick grows with content, clears empty height, bounds selectors without resize, preserves manual reading and leaves Room unchanged");

for (const surface of ["island", "nest"]) {
  const chat = new ChatPanel({ models: async () => [], pick: async () => ({}), keyboard() {} }, () => {}, { window: surface });
  await tick();
  chat.load({ ...conversation, busy: true, lines: [{ who: "zeca", text: "A reply already streaming" }] });
  assert.equal(chat.hasWaitingApproval(), false);
  assert.equal(chat.clip(performance.now()), "think", "Thinking continues through streamed words, not only before the first one");
  chat.receive({ kind: "permission", id: "expression-only", tool: "Bash", target: "git status", description: null, full: null, added: 0, removed: 0, always: false, outside: null });
  assert.equal(chat.hasWaitingApproval(), true, "A new Chat card suppresses dictation before the next core view");
  chat.showVoice("listening");
  assert.equal(chat.clip(performance.now()), "question", "A pending decision keeps the question pose");
  chat.receive({ kind: "stopped" });
  assert.equal(chat.hasWaitingApproval(), false, "An expired Chat card releases feedback priority");
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
const voiceEvents = {}, voiceCalls = [], voiceDrafts = [], voiceViewActions = [];
let voiceSaveFails = false, pendingVoiceStop = false, resolveVoiceStop;
const voiceRoot = new Element("main");
document.getElementById = () => voiceRoot;
document.documentElement = new Element("html");
globalThis.location = { hash: "#chat" };
globalThis.voiceTestCreateNest = (_root, backend) => {
  const chat = new ChatPanel(backend.chat, () => {}, { window: "nest" });
  const noop = () => {};
  return globalThis.voiceTestNest = { chat, current: () => "chat", open: tab => voiceViewActions.push(tab), flushState: () => chat.flushDraft(), setFull: noop,
    setZeca: on => chat.setEnabled(on), setSound: noop, setSpecies: noop, setKeys: noop, setMedia: noop,
    memoryChanged: noop, usageChanged: noop, spendChanged: noop, relabel: noop, view: noop, shortcut: noop };
};
globalThis.voiceTestBridge = new Proxy({}, { get: (_target, name) => {
  if (name === "textQuestionStatus" || name === "textQuestionDepart") return async () => ({ seq: 0, surface: "", phase: "", offer: null });
  if (name.startsWith("on")) return cb => { voiceEvents[name] = cb; return Promise.resolve(); };
  return async (...args) => {
    switch (name) {
      case "voiceStatus": return { ready: true };
      case "voiceStart": voiceCalls.push(["start", ...args]); return;
      case "voiceStop": voiceCalls.push(["stop"]); return pendingVoiceStop ? new Promise(resolve => { resolveVoiceStop = resolve; }) : "Spoken words";
      case "voiceCancel": voiceCalls.push(["cancel"]); return;
      case "chatSend": assert.fail("Dictation never sends a message");
      case "chatDraftSet": voiceDrafts.push(args[0]); if (voiceSaveFails) throw new Error("Draft save unavailable."); return;
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
voiceEvents.onDictation({ sequence: 1, generation: 1, phase: "listening", error: null });
const roomDictation = nodes(document.body).find(node => node.classList.contains("dictation-status"));
assert.equal(roomDictation.hidden, false);
voiceEvents.onChat({ kind: "permission", id: "dictation-priority", tool: "Bash", target: "git status", description: null, full: null, added: 0, removed: 0, always: false, outside: null });
assert.equal(roomDictation.hidden, true, "The real Nest entry yields dictation feedback to a new Chat card before a core view");
voiceEvents.onChat({ kind: "stopped" });
assert.equal(roomDictation.hidden, false);
console.log("check-chat-models: real Room entry wires voice status/settings, microphone, preview, silence, transcription draft and cancellation");
pendingVoiceStop = true;
roomMic.listeners.click(); await tick();
voiceEvents.onVoiceSilence(); await tick();
assert.equal(roomChat.cancelVoice(), true, "Cancel also discards an in-flight Chat transcription");
const externalFocus = new Element("input"); externalFocus.focus();
const keptVoiceDraft = roomInput.value;
roomMic.listeners.click(); await tick();
resolveVoiceStop("Cancelled late words"); await tick();
assert.equal(roomInput.value, keptVoiceDraft, "An old transcript cannot fill a cancelled or newer draft");
assert.equal(document.activeElement, externalFocus, "An old transcript cannot pull focus from a dictation destination");
assert.ok(roomChat.element.textContent.includes("Listening…"), "The new recording survives the old result");
roomChat.cancelVoice(); pendingVoiceStop = false;
console.log("check-chat-models: cancelling Chat transcription rejects late text, focus and state changes across a new recording");


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
  const permission = nodes(chat.element).find(n => n.classList.contains("ask"));
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

// The plan badge updates only the model control, never the draft, transcript or open choices.
const actualDateNow = Date.now, originalPlanTimeout = window.setTimeout, originalPlanClear = window.clearTimeout;
let planNow = 2_000_000, timerId = 0;
const planTimers = new Map();
const windowOf = (agent, used_percent, resets_at = null, minutes = 300) => ({ agent, used_percent, resets_at, minutes });
try {
  Date.now = () => planNow;
  for (const surface of ['island', 'nest']) {
    let modelReads = 0, effects = 0;
    const chat = new ChatPanel({ models: async () => { modelReads++; return []; }, pick: async () => ({}), keyboard() {},
      send: async () => { effects++; }, decide: () => { effects++; }, reset: async () => { effects++; return 'claude'; }, setPick: async () => { effects++; },
    }, () => {}, { window: surface });
    await tick();
    chat.load({ ...conversation, busy: false, provider: 'claude', lines: [{ who: 'zeca', text: 'Read this reply first.' }] });
    chat.draft('Keep this draft'); chat.attach(['/inbox/next.txt']);
    const trigger = nodes(chat.element).find(n => n.classList.contains('model'));
    const badge = nodes(trigger).find(n => n.classList.contains('model-usage'));
    const input = nodes(chat.element).find(n => n.tag === 'textarea');
    const transcript = nodes(chat.element).find(n => n.className === 'log');
    const contents = [...transcript.children]; transcript.scrollTop = 96;
    trigger.click(); const menu = chat.element.querySelector('.model-menu');
    const choices = [...menu.querySelectorAll('button')]; choices[0].focus();
    const focused = document.activeElement, reads = modelReads;
    chat.setUsage([windowOf('claude', 99, 1999), windowOf('claude', 69, 2010), windowOf('claude', 88, 2020, 10080), windowOf('codex', 97, 2030)]);
    assert.equal(badge.hidden, false); assert.equal(badge.textContent, '88%');
    assert.equal(badge.classList.contains('warning'), true);
    assert.ok(trigger.attributes['aria-label'].includes('88% used · Warning'));
    assert.ok(trigger.title.includes('5 hours: 69% used') && trigger.title.includes('A week: 88% used'));
    assert.equal(document.activeElement, focused); assert.deepEqual(menu.querySelectorAll('button'), choices);
    assert.deepEqual(transcript.children, contents); assert.equal(transcript.scrollTop, 96);
    assert.equal(input.value, 'Keep this draft'); assert.ok(chat.element.textContent.includes('next.txt'));
    for (const [percent, level] of [[0, 'normal'], [69, 'normal'], [70, 'warning'], [89, 'warning'], [90, 'critical'], [100, 'critical']]) {
      chat.setUsage([windowOf('claude', percent)]);
      assert.equal(badge.textContent, `${percent}%`); assert.equal(badge.classList.contains(level), true);
      assert.equal(badge.querySelector('.usage-ring').style['--used'], `${percent}%`);
    }
    chat.setUsage([windowOf('claude', 100, 2000), windowOf('codex', 97)]);
    assert.equal(badge.hidden, true, 'Expired or another provider’s windows are not current usage');
    chat.load({ ...conversation, provider: 'codex', busy: false, lines: [] });
    assert.equal(badge.hidden, false); assert.equal(badge.textContent, '97%');
    chat.load({ ...conversation, provider: 'api', busy: false, lines: [] });
    chat.setUsage([windowOf('claude', 93), windowOf('codex', 97)]);
    assert.equal(badge.hidden, true, 'The API’s Claude bird never borrows Claude Code’s subscription');
    chat.load({ ...conversation, provider: 'claude', busy: false, lines: [] });
    chat.setUsage([]); assert.equal(badge.hidden, true);
    chat.setUsage([windowOf('claude', NaN), windowOf('claude', -1), windowOf('claude', 101)]); assert.equal(badge.hidden, true);
    window.setTimeout = (fn, ms) => { const id = ++timerId; planTimers.set(id, { fn, ms }); return id; };
    window.clearTimeout = id => planTimers.delete(id);
    chat.toggle(true); chat.setUsage([windowOf('claude', 91, 2001), windowOf('claude', 40, 2002)]);
    const next = [...planTimers.values()].find(t => t.ms === 1000); assert.ok(next, 'One deadline wakes the badge at reset');
    planNow = 2_001_000; next.fn(); assert.equal(badge.textContent, '40%');
    const last = [...planTimers.values()].find(t => t.ms === 1000); assert.ok(last);
    planNow = 2_002_000; last.fn(); assert.equal(badge.hidden, true, 'A window can expire without any new hook');
    planNow = 2_000_000; chat.setUsage([windowOf('claude', 80, 2010)]); assert.ok(chat.usageTimer);
    chat.toggle(false); assert.equal(chat.usageTimer, undefined, 'A closed Chat has no reset timer');
    chat.toggle(true); chat.setEnabled(false); assert.equal(chat.usageTimer, undefined);
    assert.equal(modelReads, reads, 'Plan updates and expiry never ask for models'); assert.equal(effects, 0);
    window.setTimeout = originalPlanTimeout; window.clearTimeout = originalPlanClear; planTimers.clear();
  }
} finally {
  Date.now = actualDateNow; window.setTimeout = originalPlanTimeout; window.clearTimeout = originalPlanClear;
}
console.log('check-chat-models: provider-scoped plan ring, every threshold, zero/unknown/invalid/API/reset cases, deadline lifecycle, stable menu/draft/files/reading and no actions');

// The real Nest entry forwards live windows to its ChatPanel too.
voiceEvents.onUsage([{ agent: 'claude', minutes: 300, used_percent: 76, resets_at: null }]);
assert.equal(nodes(roomChat.element).find(n => n.classList.contains('model-usage')).textContent, '76%');

// The actual Bridge subscribes before reading; a delayed snapshot never overwrites an event.
const usageIpc = { callback: null, subscribed: null, read: null, calls: [] };
globalThis.planUsageIpc = usageIpc;
const bridgeBundles = await build({ configFile: false, root, logLevel: 'silent',
  plugins: [{ name: 'usage-ipc', enforce: 'pre', resolveId(id) { if (id.startsWith('@tauri-apps/api/')) return '\0usage-'+id.split('/').at(-1); },
    load(id) {
      if (id === '\0usage-core') return 'export const invoke = (...args) => {planUsageIpc.calls.push(args);return new Promise(resolve=>planUsageIpc.read=resolve)};';
      if (id === '\0usage-event') return 'export const emitTo=()=>{};export const listen=(event,cb)=>{planUsageIpc.callback=cb;return new Promise(resolve=>planUsageIpc.subscribed=resolve)};';
      if (id === '\0usage-window') return 'export const getCurrentWindow=()=>({});';
    } }], build: { write: false, minify: false, lib: { entry: `${root}src/bridge.ts`, formats: ['es'] } },
});
const bridgeChunk = bridgeBundles[0].output.find(p => p.type === 'chunk');
const { Bridge: usageBridge } = await import(`data:text/javascript;base64,${Buffer.from(bridgeChunk.code).toString('base64')}`);
const seenUsage = [];
usageBridge.onUsage(windows => seenUsage.push(windows));
assert.deepEqual(usageIpc.calls, [], 'No snapshot is read before the listener is ready');
usageIpc.subscribed(() => {}); await tick(); assert.deepEqual(usageIpc.calls, [['usage']]);
const liveUsage = [windowOf('codex', 90)]; usageIpc.callback({ payload: liveUsage });
usageIpc.read([windowOf('claude', 20)]); await tick(); assert.deepEqual(seenUsage, [liveUsage]);
usageIpc.calls.length = 0; const seededUsage = [];
usageBridge.onUsage(windows => seededUsage.push(windows));usageIpc.subscribed(() => {});await tick();
usageIpc.read([]); await tick(); assert.deepEqual(seededUsage, [[]], 'An empty cached snapshot is explicit absence, not zero usage');
console.log('check-chat-models: actual Nest usage event binding and Bridge listener-before-cache, late read rejection and empty seed');

// References use stable identities and selecting an option only changes the editable draft.
for (const surface of ['island','nest']) {
  const sent=[], drafts=[];
  const chat=new ChatPanel({models:async()=>[],pick:async()=>({}),keyboard(){},saveDraft:async d=>drafts.push(d),send:async(...args)=>sent.push(args)},()=>{},{window:surface});
  await tick(); chat.toggle(true);
  const input=chat.element.querySelector('textarea'), menu=chat.element.querySelector('.mention-menu');
  const sessions=[{agent:'claude',id:'shared',project:'same name',cwd:'/one/same name'},{agent:'codex',id:'shared',project:'same name',cwd:'/two/same name'},{agent:'codex',id:'next',project:'same name',cwd:'/one/same name'}];
  chat.setSessions(sessions);
  chat.element.isConnected=true;
  const type=(value,caret=value.length)=>{input.value=value;input.setSelectionRange(caret,caret);input.listeners.input();};
  const key=key=>input.listeners.keydown({key,preventDefault(){},stopPropagation(){}});
  type('Email user@example.com');assert.equal(menu.classList.contains('on'),false);
  type('@same name');assert.equal(menu.children.length,5,'Project searches include spaces');
  input.listeners.keydown({key:'Enter',shiftKey:true,preventDefault(){assert.fail('Shift+Enter stays native even with suggestions')}});
  assert.equal(menu.classList.contains('on'),false);assert.equal(sent.length,0);
  type('Review @same then explain',12);assert.equal(menu.classList.contains('on'),true);
  assert.equal(menu.children.length,5);assert.ok(menu.textContent.includes('/one/same name')&&menu.textContent.includes('/two/same name'));
  assert.equal(input.attributes['aria-expanded'],'true');
  key('ArrowDown'); const selected=input.attributes['aria-activedescendant'];
  chat.setSessions([...sessions]);assert.equal(input.attributes['aria-activedescendant'],selected,'A live view keeps the active identity');
  key('Enter');await tick();assert.equal(sent.length,0);assert.ok(input.value.endsWith(' then explain'));
  const project=drafts.at(-1).mentions[0];assert.deepEqual(project.target,{kind:'project',cwd:'/two/same name'});
  assert.ok(input.value.includes(project.token));assert.equal(menu.classList.contains('on'),false);
  const copy=new ChatPanel({models:async()=>[],pick:async()=>({}),keyboard(){},saveDraft:async d=>drafts.push(d)},()=>{},{window:surface});await tick();
  copy.restoreDraft(drafts.at(-1));await copy.flushDraft();assert.deepEqual(drafts.at(-1).mentions,[project]);
  type(input.value+' @claude');
  const mouseChoice=menu.children[0];
  mouseChoice.listeners.pointerdown({preventDefault(){}});
  for(const listener of documentEvents.pointerdown)listener({target:mouseChoice});
  assert.equal(menu.classList.contains('on'),true,'A real pointer sequence keeps the row until click');
  mouseChoice.click();await tick();assert.equal(drafts.at(-1).mentions.length,2);assert.equal(sent.length,0);
  type(input.value+' @');for(const listener of documentEvents.pointerdown)listener({target:document.body});assert.equal(menu.classList.contains('on'),false);
  type(input.value+' @codex');key('ArrowDown');key('Tab');await tick();
  assert.equal(sent.length,0);assert.deepEqual(drafts.at(-1).mentions[2].target,{kind:'session',agent:'codex',id:'next'});
  const references=[...drafts.at(-1).mentions], text=input.value;
  type(text+' @');key('Escape');assert.equal(menu.classList.contains('on'),false);assert.ok(input.value.endsWith(' @'));
  type(text+' @');input.listeners.compositionstart();assert.equal(menu.classList.contains('on'),false);
  input.listeners.keydown({key:'Enter',isComposing:true,preventDefault(){assert.fail('IME is native')}});assert.equal(sent.length,0);
  type(text);key('Enter');await tick();await tick();assert.equal(sent.length,1);assert.deepEqual(sent[0][3],references);assert.equal(sent[0][2],null,'References do not change the execution folder');
  chat.receive({kind:'done'});type('');await tick();assert.equal(drafts.at(-1).mentions,undefined);
  type('@codex');assert.equal(menu.classList.contains('on'),true);chat.toggle(false);assert.equal(menu.classList.contains('on'),false);
  chat.toggle(true);type('@');chat.setEnabled(false);assert.equal(menu.classList.contains('on'),false);
}
console.log('check-chat-models: both surfaces reference exact sessions/projects, caret insertion, keyboard/IME/dismissal, live identity, draft transfer and explicit send without folder changes');
voiceEvents.onView({sessions:[{agent:'claude',id:'entry',project:'wired',cwd:'/wired'}]});
roomInput.value='@wired';roomInput.setSelectionRange(6,6);roomInput.listeners.input();
assert.ok(roomChat.element.querySelector('.mention-menu').textContent.includes('wired'),'The actual Nest entry supplies live sessions');

for(const surface of ['island','nest']) {
  const writes=[];
  const chat=new ChatPanel({models:async()=>[],pick:async()=>({}),keyboard(){},saveDraft:async d=>writes.push(d),send:async()=>{throw "A referenced session or project is no longer in the flock. Remove its reference and choose again.";}},()=>{},{window:surface});
  await tick();
  const ref={token:'@"Session: missing"',target:{kind:'session',agent:'codex',id:'gone'}};
  chat.restoreDraft({text:'Review '+ref.token,files:['/inbox/review.txt'],folder:null,provider:'claude',mentions:[ref]});
  const input=chat.element.querySelector('textarea');input.listeners.keydown({key:'Enter',preventDefault(){}});await tick();await tick();
  assert.equal(input.value,'Review '+ref.token);assert.deepEqual(writes.at(-1).mentions,[ref]);assert.deepEqual(writes.at(-1).files,['/inbox/review.txt']);
  assert.equal(chat.started,false,'A failed reference never pins a new conversation');
}
console.log('check-chat-models: rejected references retain editable text, files, identities and the unstarted conversation in both surfaces');

// Live core offers update their own controls, keeping the transcript and editable source intact.
for (const surface of ['island','nest']) {
  const drafts=[], actions=[];
  const chat=new ChatPanel({models:async()=>[],pick:async()=>({}),keyboard(){},saveDraft:async d=>drafts.push(d),openFlock:()=>actions.push('flock'),send:async()=>actions.push('send')},()=>{},{window:surface});
  await tick();
  const input=chat.element.querySelector('textarea');
  const sessions=[{agent:'codex',id:'same',project:'site',cwd:'/one/site'},{agent:'claude',id:'same',project:'site',cwd:'/two/site'},{agent:'codex',id:'next',project:'site',cwd:'/three/site'}];
  const offers={greeting:'Good morning',caption:'site is waiting for your answer.',suggestions:[{id:'waiting',text:'What is site waiting for?',session:{agent:'codex',id:'next'}}],followups:['explain','next','flock']};
  chat.setSessions(sessions);chat.setOffers(offers);
  chat.restoreDraft({text:'Keep my notes',files:['/inbox/notes.txt'],folder:'/execution',provider:'claude'});
  const suggestions=chat.element.querySelectorAll('.suggestion');
  assert.equal(suggestions.length,surface==='nest'?1:0);
  if(surface==='nest') {
    suggestions[0].focus();chat.setOffers({...offers,greeting:'Good afternoon'});
    assert.equal(document.activeElement,suggestions[0],'A greeting refresh retains the offered button and focus');
    suggestions[0].click();await chat.flushDraft();
    assert.ok(input.value.startsWith('Keep my notes\n\nWhat is site waiting for?'));
    assert.deepEqual(drafts.at(-1).mentions[0].target,{kind:'session',agent:'codex',id:'next'});
    assert.ok(input.value.includes('next'),'Duplicate session names keep the exact ID in their token');
    assert.deepEqual(drafts.at(-1).files,['/inbox/notes.txt']);assert.equal(drafts.at(-1).folder,'/execution');
    const before=input.value;chat.setSessions([]);suggestions[0].click();assert.equal(input.value,before,'An expired exact session is not inserted');chat.setSessions(sessions);
    suggestions[0].focus();chat.setOffers({...offers,suggestions:[]});assert.equal(document.activeElement,input,'A removed offer returns focus to the editor');
    suggestions[0].click();assert.equal(input.value,before,'Detached offers are inert');
  }
  chat.load({...conversation,lines:[{who:'you',text:'Question',files:[]},{who:'zeca',text:'Older reply'},{who:'you',text:'Next question',files:[]},{who:'zeca',text:'Latest reply'}]});
  const log=chat.element.querySelector('.log'), replies=chat.element.querySelectorAll('.zeca'), reply=replies.at(-1);
  const group=reply.querySelector('.reply-chips');assert.ok(group);assert.equal(replies[0].querySelector('.reply-chips'),null);
  assert.equal(group.children.length,3);assert.equal(group.attributes['aria-label'],'Continue the conversation');
  log.scrollTop=27;input.focus();input.setSelectionRange(2,2);const text=input.value;
  chat.setOffers({...offers,greeting:'Good evening',followups:['next']});
  assert.equal(chat.element.querySelectorAll('.zeca').at(-1),reply);assert.equal(log.scrollTop,27);assert.equal(document.activeElement,input);assert.equal(input.selectionStart,2);assert.equal(input.value,text);
  group.children[0].click();await chat.flushDraft();assert.ok(input.value.endsWith('What should I check next?'));assert.deepEqual(actions,[]);
  chat.setOffers(offers);group.children[0].click();await chat.flushDraft();assert.ok(input.value.endsWith('Explain your last reply more simply.'));
  group.children[2].click();await tick();assert.deepEqual(actions,['flock']);
  const stale=group.children[0];chat.load({...conversation,lines:[{who:'error',text:'Failed reply'}]});
  assert.equal(chat.element.querySelector('.reply-chips'),null);const unchanged=input.value;stale.click();assert.equal(input.value,unchanged);
  chat.load({...conversation,busy:true,partial:'Still writing',lines:[]});assert.equal(chat.element.querySelector('.reply-chips'),null);stale.click();assert.equal(input.value,unchanged);
  chat.load({...conversation,lines:[{who:'stopped',text:''}]});assert.equal(chat.element.querySelector('.reply-chips'),null);
  chat.load({...conversation,lines:[{who:'zeca',text:'Finished'}]});chat.setEnabled(false);stale.click();assert.equal(input.value,unchanged);
  assert.deepEqual(actions,['flock'],'Suggestions never send, execute a tool or answer a card');
}
console.log('check-chat-models: core greetings/offers, exact draft references, stale/disabled guards, latest settled reply, stable reading/caret/focus and explicit Flock navigation in both surfaces');

// The real Nest entry routes this new action through the same save/error guard as size changes.
roomChat.setEnabled(true);
roomChat.load({...conversation,lines:[{who:'zeca',text:'Ready to review'}]});
voiceEvents.onView({sessions:[{agent:'claude',id:'entry',project:'wired',cwd:'/wired'}],chat:{greeting:'Good morning',caption:'Current flock',suggestions:[],followups:['flock']}});
assert.equal(roomChat.element.querySelectorAll('.reply-chip').length,1,'The actual Nest entry supplies core offers');
roomInput.value='Keep this draft';roomInput.listeners.input();await tick();
const navigationCount=voiceViewActions.length;
voiceSaveFails=true;roomChat.element.querySelector('.reply-chip').click();await tick();await tick();
assert.equal(voiceViewActions.length,navigationCount,'A failed save keeps the same view');
assert.equal(voiceRoot.querySelector('.nest-close-error').hidden,false);
assert.ok(voiceRoot.textContent.includes('Could not change the view'));
assert.equal(roomInput.value,'Keep this draft');
voiceSaveFails=false;roomChat.element.querySelector('.reply-chip').click();await tick();await tick();
assert.equal(voiceViewActions.at(-1),'flock');assert.equal(voiceRoot.querySelector('.nest-close-error').hidden,true);
assert.equal(voiceDrafts.at(-1).text,'Keep this draft');
console.log('check-chat-models: actual Nest core offers, saved Flock navigation, visible save failure, retained draft and explicit retry');

// Drive real conversation events against a deterministic clock; reactions never perform actions.
{
  const realPerformance=globalThis.performance, timeout=window.setTimeout, clear=window.clearTimeout;
  let now=10_000, next=0;const timers=new Map();
  globalThis.performance={now:()=>now};
  window.setTimeout=(run,ms)=>{const id=++next;timers.set(id,{run,ms});return id;};
  window.clearTimeout=id=>timers.delete(id);
  document.hidden=false;
  for(const surface of ['island','nest']) {
    const actions=[];let changes=0;
    const chat=new ChatPanel({models:async()=>[],pick:async()=>({}),keyboard(){},send:async()=>actions.push('send'),decide:()=>actions.push('decide')},()=>changes++,{window:surface});await tick();chat.toggle(true);
    const input=chat.element.querySelector('textarea');
    input.value='Unsent question';input.listeners.input({isComposing:true});
    assert.equal(chat.clip(now),'listen','Typing and IME composition are live listening cues');
    const before=changes;input.value+='x';input.listeners.input();assert.equal(changes,before,'Continued typing extends the cue without repainting the surface');
    now+=1500;const expiry=timers.get(chat.reactionTimer);assert.ok(expiry);expiry.run();assert.equal(chat.clip(now),'idle');assert.equal(changes,before+1);
    input.listeners.input();input.value='';input.listeners.input();assert.equal(chat.clip(now),'idle');
    chat.restoreDraft({text:'Saved draft',files:[],folder:null,provider:'claude'});assert.equal(chat.clip(now),'idle','Restoring text does not impersonate typing');
    chat.receive({kind:'toolstate',active:true});chat.load({...conversation,busy:true,tools:false,lines:[]},false,true);assert.equal(chat.clip(now),'search','A live tool event outranks a delayed initial snapshot');
    chat.load({...conversation,busy:true,lines:[],partial:'Earlier words',tools:true});assert.equal(chat.clip(now),'search','A recreated surface retains current tool activity');
    const reply=chat.element.querySelector('.zeca'), log=chat.element.querySelector('.log');log.scrollTop=31;input.focus();input.setSelectionRange(2,2);
    chat.receive({kind:'toolstate',active:false});assert.equal(chat.clip(now),'think');
    assert.equal(chat.element.querySelector('.zeca'),reply);assert.equal(log.scrollTop,31);assert.equal(input.selectionStart,2);assert.equal(document.activeElement,input);
    chat.receive({kind:'text',text:'New reply'});assert.equal(chat.clip(now),'speak');
    now+=1600;assert.equal(chat.clip(now),'think','A quiet stream returns to thinking without assuming it finished');
    chat.receive({kind:'toolstate',active:true});chat.receive({kind:'text',text:'Text beside a running call'});assert.equal(chat.clip(now),'search');
    chat.receive({kind:'permission',id:'unanswered',tool:'Bash',target:'git status',description:null,full:null,added:0,removed:0,always:false,outside:null});
    chat.receive({kind:'tool',tool:'read',outside:false});chat.showVoice('listening');chat.attach(['/inbox/note.txt']);
    assert.equal(chat.clip(now),'question','Every pending card outranks later notes, voice and attachment reactions');
    chat.showVoice('off');chat.receive({kind:'stopped'});now+=2000;assert.equal(chat.clip(now),'idle');
    chat.load({...conversation,busy:true,lines:[]});chat.receive({kind:'text',text:'A completed reply'});chat.receive({kind:'done'});assert.equal(chat.clip(now),'speak');
    now+=1600;timers.get(chat.reactionTimer).run();assert.equal(chat.clip(now),'idle','Completion is a brief visual reply, never an endless celebration');
    chat.load({...conversation,lines:[{who:'zeca',text:'Saved reply'}]});assert.equal(chat.clip(now),'idle');
    input.listeners.input();chat.toggle(false);assert.equal(chat.reactionTimer,undefined);chat.toggle(true);assert.equal(chat.clip(now),'idle');
    input.listeners.input();document.hidden=true;for(const fn of documentEvents.visibilitychange)fn();assert.equal(chat.reactionTimer,undefined);document.hidden=false;assert.equal(chat.clip(now),'idle');
    chat.load({...conversation,busy:true,lines:[]});chat.receive({kind:'text',text:'Partial'});chat.receive({kind:'error',message:'Controlled error'});assert.equal(chat.clip(now),'idle');
    chat.load({...conversation,busy:true,lines:[]});chat.receive({kind:'text',text:'More'});chat.setEnabled(false);assert.equal(chat.clip(now),'idle');assert.equal(chat.reactionTimer,undefined);
    assert.deepEqual(actions,[],'Pose changes cannot send a message, run a tool or answer a permission');
  }
  globalThis.performance=realPerformance;window.setTimeout=timeout;window.clearTimeout=clear;
}
console.log('check-chat-models: live typing/IME/tool/stream/completion poses, concurrent-tool priority, pending-card priority, static restoration, bounded expiry, hidden/off/close/error cleanup, no actions');

// Activity is display-only; exactly the selected pending request owns each decision.
for (const surface of ["island", "nest"]) {
  const decisions = [], timers = [];
  const oldTimeout = window.setTimeout;
  window.setTimeout = fn => { timers.push(fn); return timers.length; };
  const chat = new ChatPanel({ models: async () => [], pick: async () => ({}), keyboard() {},
    decide: (id, answer) => decisions.push([id, answer]), openSettings() {},
  }, () => {}, { window: surface });
  await tick(); chat.toggle(true); chat.element.isConnected = true; chat.element.offsetParent = {};
  chat.load({ ...conversation, busy: true, lines: [{ who: "you", text: "Inspect this project", files: [] }] });
  const flush = () => { for (const fn of timers.splice(0)) fn(); };
  chat.receive({ kind: "activity", event: { type: "reasoning", text: "Provider summary <script>inert</script>" } }); flush();
  const activity = nodes(chat.element).find(n => n.classList.contains("chat-activity"));
  assert.ok(activity); assert.ok(!activity.open, "Thinking is available without opening itself over the reply");
  activity.open = true;
  chat.receive({ kind: "activity", event: { type: "start", id: "read", tool: "Read", detail: "/work/file.rs" } }); flush();
  const tool = nodes(activity).find(n => n.classList.contains("activity-tool")); tool.open = true;
  chat.receive({ kind: "text", text: "The readable answer" });
  chat.receive({ kind: "activity", event: { type: "reasoning", text: " More summary." } }); flush();
  assert.equal(nodes(chat.element).find(n => n.classList.contains("chat-activity")), activity);
  assert.equal(activity.open, true); assert.equal(tool.open, true);
  assert.equal(nodes(activity).find(n => n.classList.contains("activity-tool")), tool, "Streaming preserves the inspected tool and its disclosure");
  assert.ok(activity.textContent.includes("Provider summary <script>inert</script> More summary."));
  assert.ok(activity.textContent.includes("Running"));
  chat.receive({ kind: "activity", event: { type: "end", id: "read", ok: false } }); flush();
  assert.ok(tool.textContent.includes("Failed"));
  const request = (id, target) => ({ kind: "permission", id, tool: "Bash", target, full: target, description: null, added: 0, removed: 0, always: false, outside: null });
  chat.receive(request("first", "git status"));
  const first = nodes(chat.element).find(n => n.classList.contains("ask"));
  const staleAllow = nodes(first).find(n => n.tag === "button" && n.textContent === "Allow");
  chat.receive(request("second", "git diff"));
  assert.equal(nodes(chat.element).filter(n => n.classList.contains("ask")).length, 1);
  assert.ok(chat.element.textContent.includes("Review approval 1 of 2"));
  nodes(chat.element).find(n => n.attributes["aria-label"] === "Next approval").click();
  staleAllow.click();
  assert.deepEqual(decisions, [], "A stale button cannot answer a request now hidden behind the selected one");
  assert.ok(chat.element.textContent.includes("Review approval 2 of 2"));
  assert.equal(chat.answerWaiting(true), true);
  assert.deepEqual(decisions, [["second", "allow"]]);
  const receipt = nodes(chat.element).find(n => n.classList.contains("approval-receipt"));
  assert.ok(receipt && !receipt.open, "Answered commands become closed receipts, with details retained");
  assert.equal(chat.answerWaiting(false), true);
  assert.deepEqual(decisions, [["second", "allow"], ["first", "deny"]]);
  chat.receive(request("cancelled", "must never run"));
  chat.receive({ kind: "done" });
  assert.equal(chat.answerWaiting(true), false, "Even an unexpected successful end expires its pending cards");
  assert.ok(chat.element.textContent.includes("Cancelled"));
  const saved = { reasoning: "Saved public summary", tools: [{ id: "saved", tool: "Read", detail: "file.rs", ok: null }], decisions: [{ tool: "Bash", target: "git status", answer: "allow", by: "human" }], truncated: false };
  chat.load({ ...conversation, lines: [{ who: "activity", text: JSON.stringify(saved), files: [] }, { who: "zeca", text: "Saved answer", files: [] }] });
  assert.ok(chat.element.textContent.includes("Saved public summary"));
  assert.ok(chat.element.textContent.includes("No result reported"), "Missing completion is never presented as success");
  assert.equal(nodes(chat.element).filter(n => n.classList.contains("ask")).length, 0);
  assert.equal(decisions.length, 2, "Restored activity cannot replay any decision");
  chat.load({ ...conversation, busy: true, activity: saved, waiting: [{ id: "restored", tool: "Bash", target: "cut…", full: "whole command with exact scope", description: "Describe scope", added: 4, removed: 2, always: false, outside: "Web" }], lines: [] });
  assert.ok(chat.element.textContent.includes("whole command with exact scope"));
  assert.ok(chat.element.textContent.includes("content from outside"));
  chat.receive({ kind: "stopped" });
  assert.equal(chat.answerWaiting(true), false);
  assert.equal(decisions.length, 2);
  window.setTimeout = oldTimeout;
}
console.log("check-chat-models: provider-only reasoning, stable tool disclosures/status, single selected approval, stale buttons/shortcuts, compact receipts, cancellation and display-only restored activity in both surfaces");
