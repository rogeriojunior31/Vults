// Exercise the production status owner with a delayed snapshot, without a desktop or browser.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";

const root = fileURLToPath(new URL("../ui/", import.meta.url));
let resolveSnapshot;
const listeners = [];
globalThis.dictationTestBridge = {
  onDictation(callback) { listeners.push(callback); },
  dictationStatus: () => new Promise(resolve => { resolveSnapshot = resolve; }),
};
const bundles = await build({ configFile: false, root, logLevel: "silent", plugins: [{
  name: "dictation-test-bridge",
  enforce: "pre",
  transform(code, id) {
    if (id === `${root}src/island/dictation.ts`) return `${code}\nexport { setLang } from "../i18n"; export { Sound } from "../sound";`;
  },
  resolveId(id) { if (id === "../bridge") return "\0dictation-bridge"; },
  load(id) { if (id === "\0dictation-bridge") return "export const Bridge = globalThis.dictationTestBridge"; },
}], build: { write: false, minify: false, lib: { entry: `${root}src/island/dictation.ts`, formats: ["es"] } } });
const chunk = bundles[0].output.find(part => part.type === "chunk");
const { dictationStatus, setLang, Sound } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
class Element {
  children = [];
  dataset = {};
  style = {};
  attributes = {};
  className = "";
  classList = { toggle() {} };
  hidden = true;
  measurements = 0;
  get offsetHeight() { this.measurements++; return this.className === "dictation-status" ? 20 : 48; }
  getBoundingClientRect() { return { width: 340, top: 620, bottom: 640 }; }
  setAttribute(name, value) { this.attributes[name] = value; }
  append(...children) { this.children.push(...children); }
  replaceChildren(...children) { this.children = children; }
  set textContent(text) { this.text = text; this.children = []; }
  get textContent() { return (this.text ?? "") + this.children.map(child => child.textContent).join(""); }
}
const nodes = [];
const bodyClasses = new Set();
globalThis.document = {
  documentElement: { lang: "en" },
  createElement: () => new Element(),
  createElementNS: () => new Element(),
  body: { style: { setProperty(name, value) { this[name] = value; } }, classList: {
    contains: name => bodyClasses.has(name), add: name => bodyClasses.add(name), remove: name => bodyClasses.delete(name),
  }, append(node) { nodes.push(node); } },
};
globalThis.window = { innerWidth: 720, innerHeight: 640, addEventListener() {} };
const timers = new Map();
const delays = new Map();
let nextTimer = 0;
const nativeTimeout = globalThis.setTimeout, nativeClear = globalThis.clearTimeout;
globalThis.setTimeout = (callback, delay) => { const id = ++nextTimer; timers.set(id, callback); delays.set(id, delay); return id; };
globalThis.clearTimeout = id => { timers.delete(id); delays.delete(id); };
const nativeNow = Date.now;
try {
  const view = dictationStatus();
  const line = nodes[0];
  const status = (sequence, phase) => ({ sequence, generation: 1, phase, error: null });
  listeners[0](status(2, "working"));
  assert.equal(line.textContent, "Dictation: transcribing locally…");
  resolveSnapshot(status(1, "listening"));
  await Promise.resolve();
  assert.equal(line.textContent, "Dictation: transcribing locally…", "A delayed snapshot cannot rewind live progress");
  view.visible(false);
  assert.equal(line.hidden, true, "The island yields status ownership to an open Nest");
  listeners[0](status(3, "copied"));
  assert.equal(line.hidden, true);
  assert.equal(line.textContent, "Copied. Use your app's Paste command.");
  view.visible(true);
  assert.equal(line.hidden, false);
  view.relocated(true);
  view.visible(false); view.visible(true);
  view.context({ sessions: [], approval: null });
  assert.equal(line.hidden, true, "The desktop capsule owns feedback across Nest/focus changes");
  view.relocated(false);
  assert.equal(line.hidden, false, "Unavailable overlay retains inline feedback");
  timers.values().next().value();
  view.visible(false); view.visible(true);
  assert.equal(line.hidden, true, "Expired feedback cannot reappear when the Nest closes");
  listeners[0](status(2, "locked"));
  assert.equal(line.hidden, true, "Stale progress cannot resurrect an ended recording");
  listeners[0]({ ...status(4, "copied"), app: "<img src=x>", terminal: true });
  assert.equal(line.textContent, "Copied from dictation in <img src=x>. Use Ctrl+Shift+V to paste.", "App names remain inert text");
  listeners[0]({ ...status(5, "failed"), error: "<script>unsafe()</script>" });
  assert.equal(line.textContent, "Dictation failed: <script>unsafe()</script>", "Diagnostics remain text");
  listeners[0]({ ...status(6, "pasted"), app: "KWrite" });
  assert.equal(line.textContent, "Pasted in KWrite. Review it in your editor.");
  assert.equal(timers.size, 1, "Pasted has a terminal expiry like Copied");
  timers.values().next().value();
  assert.equal(line.hidden, true);
  setLang("pt-BR");
  const failure = "The Wayland clipboard did not confirm the text.";
  listeners[0]({ ...status(7, "failed"), error: failure });
  assert.ok(line.textContent.length > 0 && !line.textContent.includes(failure), "Known clipboard errors are localized with the surrounding status");
  setLang("en");
  listeners[0]({ ...status(8, "session"), session_name: "<img src=x>" });
  assert.equal(line.textContent, "In session <img src=x>. Review it in your terminal.");
  timers.values().next().value();
  assert.equal(line.hidden, true, "Session delivery expires");
  listeners[0](status(9, "held"));
  assert.equal(line.textContent, "Copied. This session is waiting; review its card or terminal.");
  assert.equal(timers.size, 1, "Held feedback expires without hiding the card");
  for (const language of ["pt-BR", "es", "zh"]) {
    setLang(language); view.relabel();
    assert.ok(line.textContent.length > 0 && !line.textContent.startsWith("Copied."));
  }
  setLang("en");
  const phases = ["listening", "locked", "finishing", "working", "retrying", "pasted", "session", "held", "copied", "empty", "cancelled", "failed"];
  let mounted;
  const island = dictationStatus(false, node => { mounted = node; });
  assert.equal(nodes.filter(node => node.className === "dictation-status").length, 1, "An island status uses its reserved row instead of a body overlay");
  let sequence = 0;
  const clear = { sessions: [], approval: null, locked: false };
  for (const phase of phases) {
    island.update({ ...status(++sequence, phase), app: "Editor", session_name: "api", error: "No microphone" });
    assert.ok(mounted && !mounted.hidden, `${phase} has visible feedback`);
    assert.equal(mounted.dataset.phase, phase);
    assert.equal(mounted.attributes["aria-atomic"], "true", "Each transition reads as one sentence");
    assert.ok(mounted.children[0].children[0].children[0].attributes.d, "The phase uses a production icon");
    const english = mounted.textContent;
    for (const language of ["pt-BR", "es", "zh"]) {
      setLang(language); island.relabel();
      assert.ok(mounted.textContent.length && mounted.textContent !== english, `${phase} is translated in ${language}`);
    }
    setLang("en");
    const timerCount = timers.size;
    for (const priority of [{ approval: {} }, { sessions: [{ card: true }] }, { sessions: [{ waiting: true }] }, { inbox: [{ source: { kind: "chat", count: 1 } }] }, { locked: true }]) {
      island.context({ ...clear, ...priority });
      assert.equal(mounted, null, `${phase} yields to a new card or a locked desktop`);
      island.context(clear);
      assert.ok(mounted && !mounted.hidden);
    }
    assert.equal(timers.size, timerCount, "Card priority and relabeling never extend feedback expiry");
  }
  island.context({ ...clear, approval: {} });
  const expire = [...timers.values()].at(-1);
  expire();
  island.context(clear);
  assert.equal(mounted, null, "Feedback that expires behind a card never returns");
  island.update(status(sequence - 1, "listening"));
  assert.equal(mounted, null, "An old event cannot reclaim the row after expiry");
  island.update({ ...status(++sequence, "listening"), partial: "<img src=x>", bands: [0, 0.2, 0.4, 0.6, 0.8, 1, NaN, -1, 2] });
  assert.equal(mounted.textContent, "Listening: <img src=x>", "Live preview remains inert text");
  const liveLine = mounted;
  const bands = mounted.children[1];
  assert.equal(bands.hidden, false);
  assert.equal(bands.attributes["aria-hidden"], "true", "Meter updates do not flood screen readers");
  assert.deepEqual(bands.children.map(bar => bar.style.transform), [0.08, 0.2, 0.4, 0.6, 0.8, 1, 0.08, 0.08, 1].map(n => `scaleY(${n})`));
  const icon = mounted.children[0].children[0];
  island.update({ ...status(++sequence, "listening"), partial: "<img src=x>", bands: Array(9).fill(0.3) });
  assert.equal(mounted, liveLine);
  assert.equal(mounted.children[0].children[0], icon, "Meter frames preserve the phase icon");
  liveLine.hidden = true;
  island.update({ ...status(++sequence, "listening"), partial: "more words", bands: Array(9).fill(0.8) });
  assert.equal(liveLine.hidden, true, "Meter frames cannot undo immediate renderer card priority");
  assert.equal(bands.hidden, true);
  island.context({ ...clear, approval: {} });
  island.context(clear);
  assert.equal(liveLine.hidden, false);
  island.update({ ...status(++sequence, "locked"), partial: "hands free", bands: Array(9).fill(0.7) });
  assert.equal(mounted.textContent, "Hands-free: hands free");
  for (const language of ["pt-BR", "es", "zh"]) {
    setLang(language); island.relabel();
    assert.ok(!mounted.textContent.startsWith("Hands-free:"));
  }
  setLang("en");
  island.update({ ...status(++sequence, "working"), partial: null, bands: Array(9).fill(0) });
  assert.equal(bands.hidden, true, "Transcription stops live metering");
  assert.equal(mounted.textContent, "Dictation: transcribing locally…");
  island.update({ ...status(sequence - 1, "locked"), partial: "late words", bands: Array(9).fill(1) });
  assert.equal(mounted.textContent, "Dictation: transcribing locally…", "A late preview cannot rewind transcription");
  let now = 100_000;
  Date.now = () => now;
  let captionLine;
  const captions = dictationStatus(false, node => { captionLine = node; });
  const caption = nodes.at(-1);
  captions.captions(true, true);
  const rect = { width: 340, top: 0, bottom: 62 };
  captions.layout(rect, true);
  captions.update({ ...status(1, "listening"), partial: "short" });
  const captionWords = captionLine.children[2];
  captionWords.clientWidth = 270; captionWords.scrollWidth = 100;
  captions.layout(rect, true);
  assert.equal(caption.hidden, true, "Words that fit need no repeated caption");
  captionWords.scrollWidth = 800;
  captions.layout(rect, true);
  assert.equal(caption.hidden, false);
  assert.equal(caption.textContent, "Listening: short");
  assert.equal(caption.style.top, "68px");
  const measures = caption.measurements;
  captions.update({ ...status(2, "listening"), partial: "short", bands: Array(9).fill(0.2) });
  assert.equal(caption.measurements, measures, "Meter-only frames never measure the caption again");
  captions.layout({ ...rect, bottom: 634 }, true);
  assert.equal(caption.hidden, true, "No free space means no caption");
  captions.layout(rect, true);
  assert.equal(caption.hidden, false);
  for (const priority of [{ approval: {} }, { sessions: [{ waiting: true }] }, { locked: true }]) {
    captions.context({ ...clear, ...priority });
    assert.equal(caption.hidden, true);
    assert.equal(caption.textContent, "", "Blocked captions clear their DOM text");
    captions.context(clear);
    assert.equal(caption.hidden, false);
  }
  captions.context(clear, true);
  assert.equal(caption.hidden, true, "A Chat card arriving before the next core view wins immediately");
  captions.context(clear, false);
  assert.equal(caption.hidden, false);
  captions.layout(rect, false);
  captions.update({ ...status(3, "listening"), partial: "new words" });
  assert.equal(caption.hidden, true, "Meter and preview updates cannot undo renderer priority");
  captions.layout(rect, true);
  captions.captions(false);
  assert.equal(caption.hidden, true);
  assert.equal(captions.captions(true, true), false, "A delayed initial settings read cannot undo a newer choice");
  captions.captions(true);
  captions.update({ ...status(4, "copied"), text: "<script>inert final text</script>", caption_until: now + 2000 });
  assert.equal(caption.hidden, false);
  assert.equal(caption.textContent, "<script>inert final text</script>");
  assert.equal(caption.children.length, 0, "Final text is never parsed as markup");
  const expiry = [...delays].findLast(([, delay]) => delay === 2000)?.[0];
  assert.ok(expiry, "Caption expiry has its own two-second timer");
  now += 2000;
  captions.layout(rect, true);
  assert.equal(caption.hidden, true, "The deadline holds even before a suspended page's timer runs");
  timers.get(expiry)();
  captions.captions(false); captions.captions(true);
  captions.visible(false); captions.visible(true);
  captions.context({ ...clear, approval: {} }); captions.context(clear);
  assert.equal(caption.hidden, true, "Settings, ownership and cards never renew completion captions");
  captions.update({ ...status(3, "listening"), partial: "stale" });
  assert.equal(caption.hidden, true);
  captions.update({ ...status(5, "copied"), text: "old snapshot", caption_until: now - 1 });
  assert.equal(caption.hidden, true, "A new surface receiving an old snapshot cannot get a fresh two seconds");
  captions.update({ ...status(6, "cancelled"), text: null, caption_until: now + 2000 });
  captionWords.scrollWidth = 100;
  captions.layout(rect, true);
  assert.equal(caption.hidden, true, "Cancellation never retains the previous result");
  const nestFeedback = dictationStatus(true);
  nestFeedback.captions(true);
  nestFeedback.update({ ...status(1, "copied"), text: "confirmed result", caption_until: now + 500 });
  assert.ok([...delays.values()].includes(500), "A surface handoff gets only the remaining caption lifetime");
  assert.equal(nodes.at(-1).hidden, false);
  assert.equal(document.body.style["--dictation-caption-space"], "76px", "Nest and Settings reserve space for the caption and feedback line");
  assert.ok(bodyClasses.has("dictation-caption-open"), "The caption moves page content instead of covering controls");
  nestFeedback.captions(false);
  assert.equal(bodyClasses.has("dictation-caption-open"), false, "Hiding feedback returns its space");
  const totals = dictationStatus(false, node => { mounted = node; });
  let totalSequence = 0;
  for (const phase of ["copied", "pasted", "session", "held"]) {
    totals.update({ ...status(++totalSequence, phase), words: 1, app: "Editor", session_name: "api" });
    const metric = mounted.children[3];
    assert.equal(metric.hidden, false);
    assert.equal(metric.textContent, "1 word");
    totals.update({ ...status(++totalSequence, phase), words: 1234, app: "Editor", session_name: "api" });
    assert.equal(metric.textContent, "1,234 words");
    for (const [language, unit] of [["pt-BR", "palavras"], ["es", "palabras"], ["zh", "个字词"]]) {
      setLang(language); totals.relabel();
      assert.ok(metric.textContent.endsWith(unit), "Word totals use complete localized labels");
    }
    setLang("en");
    for (const count of [undefined, null, -1, NaN, Infinity, 1.5, Number.MAX_SAFE_INTEGER + 1]) {
      totals.update({ ...status(++totalSequence, phase), words: count });
      assert.equal(metric.hidden, true, "Unknown or invalid counts never appear as a measured zero");
      assert.equal(metric.textContent, "");
    }
    totals.update({ ...status(++totalSequence, phase), words: 0 });
    assert.equal(metric.textContent, "0 words", "An actual zero remains distinct from missing data");
  }
  for (const phase of ["listening", "locked", "working", "retrying", "empty", "cancelled", "failed"]) {
    totals.update({ ...status(++totalSequence, phase), words: 99 });
    assert.equal(mounted.children[3].hidden, true, "Only confirmed delivery states show final counts");
  }
  let played = 0;
  const gains = [];
  const nativeAudio = globalThis.AudioContext;
  globalThis.AudioContext = class {
    currentTime = 0;
    destination = {};
    resume() { return Promise.resolve(); }
    suspend() { return Promise.resolve(); }
    createOscillator() { return { frequency: { setValueAtTime() {} }, connect() { return { connect() {} }; }, start() { played++; }, stop() {} }; }
    createGain() { return { gain: { setValueAtTime() {}, linearRampToValueAtTime(value) { gains.push(value); } } }; }
  };
  window.setTimeout = globalThis.setTimeout;
  window.clearTimeout = globalThis.clearTimeout;
  const nativePerformance = globalThis.performance;
  let audioTime = 1000;
  globalThis.performance = { now: () => audioTime += 300 };
  try {
    const audioOwner = dictationStatus(false, undefined, () => Sound.play("peek"));
    const event = listeners.at(-1);
    const frame = (sequence, generation, phase = "listening") => ({ ...status(sequence, phase), generation });
    audioOwner.startSound(true, true);
    resolveSnapshot(frame(1, 1));
    await Promise.resolve();
    assert.equal(played, 0, "Mounting during a recording is silent");
    event(frame(2, 1));
    assert.equal(played, 0, "Meter frames after a snapshot cannot replay its start");
    event(frame(3, 2));
    assert.equal(played, 1, "A new live recording gets one cue");
    assert.ok(gains.includes(0.05), "The cue uses the shared default volume");
    event(frame(4, 2)); event(frame(5, 2, "locked"));
    audioOwner.visible(false); audioOwner.visible(true); audioOwner.relabel();
    audioOwner.startSound(false); audioOwner.startSound(true);
    event(frame(6, 2)); event(frame(2, 1));
    assert.equal(played, 1, "Meter, lock, ownership, settings and stale events never replay audio");
    audioOwner.startSound(false);
    assert.equal(audioOwner.startSound(true, true), false, "A delayed settings snapshot preserves the newer sound choice");
    event(frame(7, 3));
    audioOwner.startSound(true); event(frame(8, 3));
    assert.equal(played, 1, "Turning sound on never replays a suppressed start");
    Sound.setHushed(true); event(frame(9, 4));
    Sound.setHushed(false); event(frame(10, 4));
    Sound.setEnabled(false); event(frame(11, 5)); Sound.setEnabled(true);
    Sound.setVolume(0); event(frame(12, 6)); Sound.setVolume(25);
    assert.equal(played, 1, "Do Not Disturb, mute and zero volume suppress the dictation cue");
    event(frame(13, 7));
    assert.equal(played, 2);
    assert.ok(gains.includes(0.025), "Dictation follows the current shared volume");
    const passive = dictationStatus(true);
    passive.startSound(true);
    listeners.at(-1)(frame(1, 7));
    assert.equal(played, 2, "Nest and Settings have no audio owner");
    event(frame(14, 8, "failed")); event(frame(15, 9, "cancelled"));
    assert.equal(played, 2, "Failures and cancellations are silent");
    bodyClasses.add("dictation-overlay"); bodyClasses.add("dock-bottom");
  const floating = dictationStatus();
  const floatingLine = nodes.at(-2), floatingCaption = nodes.at(-1);
  floatingLine.children[2].clientWidth = 1;
  floatingLine.children[2].scrollWidth = 180;
  floating.captions(true);
  floating.update(status(1, "listening"));
  assert.equal(floatingCaption.hidden, true, "A compact mic does not repeat the listening label in a caption");
  floating.update({ ...status(2, "listening"), partial: "Actual heard words" });
  assert.equal(floatingCaption.textContent, "Actual heard words");
  assert.equal(floatingCaption.hidden, false);
  floating.update(status(3, "locked"));
  assert.equal(floatingCaption.hidden, true, "Hands-free without a preview stays compact");
  floating.update({ ...status(4, "copied"), text: "Confirmed result", caption_until: Date.now() + 2000 });
  assert.equal(floatingCaption.textContent, "Confirmed result");
  floating.context({ locked: true, approval: null, sessions: [] });
  assert.equal(floatingLine.hidden, true);
  assert.equal(floatingCaption.hidden, true);
  bodyClasses.delete("dictation-overlay"); bodyClasses.delete("dock-bottom");
  console.log("check-dictation-status: floating capsule has no filler caption; heard/final text, lock priority and inline fallback pass");
} finally {
    globalThis.AudioContext = nativeAudio;
    globalThis.performance = nativePerformance;
  }

  setLang("en");
  const commands = dictationStatus(true);
  const commandLine = nodes.at(-2), commandCaption = nodes.at(-1);
  let commandSequence = 1;
  commands.captions(true);
  for (const phase of ["listening", "commanded", "answered", "asked", "refused", "unknown"]) {
    commands.update({ ...status(commandSequence++, phase), command: true, result: "<img> inert result", text: "what was heard", caption_until: Date.now() + 2000 });
    assert.equal(commandLine.dataset.phase, phase);
    assert.equal(commandLine.hidden, false);
    if (phase !== "listening") {
      assert.ok(commandLine.textContent.includes("<img> inert result"));
      assert.equal(commandCaption.textContent, "what was heard", "The caption carries the transcript, not an instruction or result");
    }
    for (const language of ["pt-BR", "es", "zh"]) { setLang(language); commands.relabel(); assert.ok(commandLine.textContent); }
    setLang("en");
  }
  commands.update({ ...status(commandSequence++, "asked"), command: true, result: "Which one? api or web", expires_at: Date.now() + 500 });
  assert.ok([...delays.values()].includes(500), "A handoff or unknown reply keeps only the remaining question lifetime");
  commands.visible(false); commands.visible(true);
  commands.update({ ...status(commandSequence++, "asked"), command: true, result: "expired question", expires_at: Date.now() - 1 });
  assert.equal(commandLine.hidden, true, "An expired command question cannot reappear on another surface");
  commands.update({ ...status(commandSequence++, "answered"), command: true, result: "Safe local answer" });
  commands.context({ locked: false, approval: {}, sessions: [] });
  assert.equal(commandLine.hidden, true, "Voice answers preserve card priority");
  commands.context({ locked: false, approval: null, sessions: [] });
  commands.visible(false);
  assert.equal(commandLine.hidden, true, "The inactive surface yields command feedback ownership");
  console.log("check-dictation-status: command capture/results/refusal/unknown, inert text, heard caption, four languages, absolute question expiry and card/surface priority");
} finally {
  Date.now = nativeNow;
  globalThis.setTimeout = nativeTimeout;
  globalThis.clearTimeout = nativeClear;
}
console.log("check-dictation-status: ok (snapshot order, surface handoff, expiry, stale events, text safety, conditional captions, original deadline, settings race, meter layout budget, single-owner start sound, shared audio guards)");
