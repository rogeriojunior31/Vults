// Production command-check renderer: no capture, action dispatch or Chat backend exists here.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";
const all = node => [node, ...node.children.flatMap(all)];
class Element {
  children = []; listeners = {}; attributes = {}; dataset = {}; value = ""; disabled = false;
  constructor(tag) { this.tag = tag; }
  set textContent(value) { this.text = value; this.children = []; }
  get textContent() { return (this.text ?? "") + this.children.map(node => node.textContent).join(""); }
  append(...nodes) { this.children.push(...nodes); }
  replaceChildren(...nodes) { this.text = ""; this.children = nodes; }
  addEventListener(name, fn) { this.listeners[name] = fn; }
  setAttribute(name, value) { this.attributes[name] = value; }
  focus() { document.activeElement = this; }
  click() { if (!this.disabled) this.listeners.click?.(); }
}
globalThis.document = { createElement: tag => new Element(tag), documentElement: {} };
const root = fileURLToPath(new URL("../ui/", import.meta.url));
const bundle = await build({ configFile: false, root, logLevel: "silent", plugins: [{ name: "command-check-language", transform(code,id) {
  if (id.endsWith("/voice-command-check.ts")) return code+'\nexport { setLang } from "./i18n";';
} }], build: { write: false, minify: false, lib: { entry: `${root}src/voice-command-check.ts`, formats: ["es"] } } });
const { createCommandCheck, setLang } = await import(`data:text/javascript;base64,${Buffer.from(bundle[0].output.find(p => p.type === "chunk").code).toString("base64")}`);
const calls = [];
const view = createCommandCheck(text => new Promise((resolve,reject) => calls.push({ text, resolve, reject })));
const input = all(view.element).find(node => node.tag === "textarea");
const submit = all(view.element).find(node => node.tag === "button");
const text = () => view.element.textContent;
const tick = () => new Promise(resolve => setImmediate(resolve));
const edit = value => { input.value = value; input.listeners.input(); };
assert.equal(calls.length, 0); assert.equal(submit.disabled, true);
view.visible(true);
view.fill("open api");
assert.equal(input.value, "open api", "A catalog example becomes an editable draft");
assert.equal(view.element.open, true);
assert.equal(document.activeElement, input);
assert.equal(calls.length, 0, "Filling an example never checks or executes it");
edit("pause media"); submit.focus(); submit.click(); submit.click();
assert.equal(calls.length, 1); assert.equal(calls[0].text, "pause media");
calls[0].resolve({ status: "parsed", actions: [{ action: "pause", target: null, value: null, status: "recognized", candidates: [] }] }); await tick();
assert.ok(text().includes("Pause playback") && text().includes("Nothing was executed"));
assert.equal(document.activeElement, submit);
edit("open api"); assert.ok(!text().includes("Pause playback")); submit.click();
calls[1].resolve({ status: "parsed", actions: [{ action: "open", target: "api", value: null, status: "ambiguous", candidates: ["<script>inert target</script>", "other api"] }] }); await tick();
assert.ok(text().includes("need clarification") && text().includes("<script>inert target</script>"));
assert.ok(!all(view.element).some(node => node.tag === "script"));
submit.click(); edit("mute computer");
calls[2].resolve({ status: "unknown", actions: [] }); await tick();
assert.ok(!text().includes("No supported command"), "Editing invalidates an older result");
submit.click(); view.visible(false); view.visible(true);
calls[3].reject(Error("private backend detail")); await tick();
assert.ok(!text().includes("Could not check") && !text().includes("private backend detail"));
assert.equal(input.value, "mute computer", "Leaving retains the local draft but discards results");
submit.click(); calls[4].reject(Error("native detail")); await tick();
assert.ok(text().includes("Could not check this command") && !text().includes("native detail"));
submit.click(); calls[5].resolve({ status: "negated", actions: [] }); await tick();
assert.ok(text().includes("Negated requests stay local"));
submit.click(); view.invalidate();
assert.equal(input.value, "mute computer", "Setup changes retain the phrase draft");
assert.ok(text().includes("Command setup changed"));
calls[6].resolve({ status: "parsed", actions: [{ action: "pause", target: null, value: null, status: "recognized", candidates: [] }] }); await tick();
assert.ok(!text().includes("Pause playback") && text().includes("Command setup changed"), "A late result cannot restore targets after setup changes");
view.invalidate(); assert.ok(text().includes("Command setup changed"));
submit.click(); calls[7].resolve({ status: "negated", actions: [] }); await tick();
assert.ok(text().includes("Negated requests stay local") && !text().includes("Command setup changed"));
edit("\u4e2d".repeat(1400)); assert.equal(submit.disabled, true, "Enforce UTF-8 bytes, not just code units");
edit("pause media");
for (const lang of ["pt-BR", "es", "zh"]) {
  setLang(lang); view.relabel();
  assert.ok(!text().includes("Check a command") && !text().includes("Type a phrase"));
}
assert.equal(input.value, "pause media");
console.log("check-voice-command-check: explicit bounded query, inert targets, ambiguity, edit/departure races, retry, retained draft/focus and languages");

setLang("en");
const targetReads = [];
const targetView = createCommandCheck(phrase => new Promise(resolve => targetReads.push({ phrase, resolve })));
const targetInput = all(targetView.element).find(node => node.tag === "textarea");
const targetSubmit = all(targetView.element).find(node => node.tag === "button");
const targets = [{ id: "a", agent: "codex", project: "api", cwd: "/work/api" }, { id: "b", agent: "codex", project: "web", cwd: "/work/web" }];
const interpretation = { status: "parsed", actions: [{ action: "focus", target: "api", value: null, status: "recognized", candidates: [] }] };
const targetText = () => targetView.element.textContent;
targetView.visible(true); targetView.targets(targets);
targetInput.value = "open project api"; targetInput.listeners.input(); targetSubmit.click();
targetReads[0].resolve(interpretation); await tick();
assert.ok(targetText().includes("Focus session"));
targetView.targets(targets.toReversed().map(session => ({ ...session, status: "working", step: "new progress" })));
assert.ok(targetText().includes("Focus session"), "Progress and presentation order do not invalidate the target catalog");
targetView.targets([{ ...targets[0], cwd: "/work/replaced" }, targets[1]]);
assert.ok(!targetText().includes("Focus session") && targetText().includes("Command targets changed"));
assert.equal(targetInput.value, "open project api");
assert.equal(targetReads.length, 1, "Target updates never start another check automatically");
targetSubmit.focus(); targetSubmit.click(); targetView.targets([]);
targetReads[1].resolve(interpretation); await tick();
assert.ok(!targetText().includes("Focus session") && targetText().includes("Command targets changed"), "Departed targets invalidate pending replies");
assert.equal(document.activeElement, targetSubmit);
targetView.targets(targets); targetSubmit.click(); targetReads[2].resolve(interpretation); await tick();
targetView.targets([{ ...targets[0], project: "renamed" }, targets[1]]);
assert.ok(targetText().includes("Command targets changed"));
console.log("check-voice-command-check: live target identity/name/folder/removal changes discard stale results, retain draft/focus, ignore late replies and preserve progress-only results");

const cancellableQueries = [];
const cancellable = createCommandCheck(phrase => new Promise((resolve,reject) => cancellableQueries.push({ phrase, resolve, reject })));
const cancellationInput = all(cancellable.element).find(node => node.tag === "textarea");
const cancellationSubmit = all(cancellable.element).find(node => node.dataset.setting === "command-check-submit");
const cancellationAction = all(cancellable.element).find(node => node.dataset.setting === "command-check-cancel");
assert.equal(cancellationAction.hidden, true);
cancellable.visible(true); cancellationInput.value = "open project api"; cancellationInput.listeners.input();
cancellationSubmit.click(); cancellationAction.focus(); cancellationAction.click();
assert.equal(cancellationInput.value, "open project api");
assert.equal(cancellationAction.hidden, true);
assert.equal(cancellationSubmit.disabled, false);
assert.equal(document.activeElement, cancellationSubmit);
assert.ok(cancellable.element.textContent.includes("Check cancelled"));
assert.equal(cancellableQueries.length, 1);
cancellationSubmit.click(); cancellationAction.focus();
cancellableQueries[0].reject(Error("old private failure")); await tick();
assert.equal(cancellationSubmit.disabled, true, "A cancelled failure cannot end the replacement query");
assert.ok(!cancellable.element.textContent.includes("Could not check") && !cancellable.element.textContent.includes("old private failure"));
cancellableQueries[1].resolve(interpretation); await tick();
assert.equal(cancellationAction.hidden, true);
assert.equal(document.activeElement, cancellationSubmit);
assert.ok(cancellable.element.textContent.includes("Focus session"));
console.log("check-voice-command-check: explicit cancel retains draft, permits retry, ignores old failure and returns focus when Cancel disappears");

const validationCalls = [];
const validation = createCommandCheck(async phrase => { validationCalls.push(phrase); return interpretation; });
const validationInput = all(validation.element).find(node => node.tag === "textarea");
const validationSubmit = all(validation.element).find(node => node.dataset.setting === "command-check-submit");
validation.visible(true);
const controlPoints = [...Array.from({ length: 32 }, (_, index) => index), ...Array.from({ length: 33 }, (_, index) => index + 127)];
for (const point of controlPoints) {
  validationInput.value = `pause${String.fromCodePoint(point)} media`; validationInput.listeners.input();
  const allowed = [9, 10, 11, 12, 13, 133].includes(point);
  assert.equal(validationSubmit.disabled, !allowed, `Native control/whitespace boundary: U+${point.toString(16)}`);
  assert.equal(validationInput.attributes["aria-invalid"], String(!allowed));
  if (!allowed) {
    validationSubmit.click();
    assert.ok(validation.element.textContent.includes("Remove control characters"));
  }
}
assert.equal(validationCalls.length, 0);
validationInput.value = "\u4e2d".repeat(1400); validationInput.listeners.input();
assert.equal(validationSubmit.disabled, true);
assert.equal(validationInput.attributes["aria-invalid"], "true");
assert.ok(validation.element.textContent.includes("too long"));
validationInput.value = "pause\n\tmedia"; validationInput.listeners.input();
assert.equal(validationSubmit.disabled, false, "Permitted whitespace is preserved");
assert.equal(validationInput.attributes["aria-invalid"], "false");
assert.equal(validationInput.attributes["aria-describedby"], "command-check-feedback");
validationSubmit.click(); await tick();
assert.deepEqual(validationCalls, ["pause\n\tmedia"], "The renderer never silently rewrites the phrase");
console.log("check-voice-command-check: native-compatible control/byte limits, associated accessible error and exact permitted whitespace without invalid backend calls");
