import type { DictationTry, DictationWords, DictationWordsChange } from "../../../bridge";
import { el } from "../../../dom";
import { t } from "../../../i18n";
import { icon } from "../../../island/icons";

export interface WordsBackend {
  words(): Promise<DictationWords>;
  change(edit: DictationWordsChange): Promise<void>;
  trySentence(text: string, terminal: boolean): Promise<DictationTry>;
}

export function createYourWords(backend: WordsBackend, settings: () => void) {
  const dictionary = el("section", { class: "dictation-history your-words" });
  const snippets = el("section", { class: "dictation-history your-words" });
  const labels: [HTMLElement, () => string][] = [];
  const label = <T extends HTMLElement>(node: T, text: () => string): T => { labels.push([node, text]); return node; };
  const heading = (title: () => string, about: () => string) => el("header", { class: "dictation-heading" },
    el("div", {}, label(el("h2"), title), label(el("p"), about)),
    label(el("button", { onclick: settings }), () => t("Dictation settings")));
  dictionary.append(heading(() => t("Dictionary"), () => t("Names, spelling and replacements for independent dictation.")));
  snippets.append(heading(() => t("Snippets"), () => t("Say a short phrase to insert its saved text unchanged.")));
  const status = [el("p", { class: "dictation-note" }), el("p", { class: "dictation-note" })];
  const confirmations = [el("div", { class: "dictation-confirmation" }), el("div", { class: "dictation-confirmation" })];
  const refreshes = [0, 1].map(() => label(el("button", { onclick: () => void read() }), () => t("Refresh")));
  for (const [index, page] of [dictionary, snippets].entries()) {
    status[index].setAttribute("role", "status");
    status[index].tabIndex = -1;
    page.append(el("div", { class: "dictation-toolbar" }, refreshes[index]), status[index], confirmations[index]);
  }
  const field = (text: () => string, multiline = false) => {
    const control = multiline ? el("textarea") : el("input");
    const element = el("label", {}, label(el("span"), text), control);
    return { element, control };
  };
  const word = field(() => t("New word"));
  const from = field(() => t("When I say"));
  const to = field(() => t("Write"), true);
  const trigger = field(() => t("When I say"));
  const body = field(() => t("Paste"), true);
  const sentence = field(() => t("Try a sentence"), true);
  sentence.control.maxLength = 262144;
  const terminal = el("input"); terminal.type = "checkbox";
  const wordList = el("div", { class: "words-chips" });
  const replacements = el("div", { class: "words-entries" });
  const snippetList = el("div", { class: "words-entries" });
  const result = el("div", { class: "words-result" }); result.setAttribute("aria-live", "polite");
  const forms: HTMLButtonElement[] = [];
  const valid = (control: HTMLInputElement | HTMLTextAreaElement, source = false) => {
    const value = source ? control.value.trim() : control.value;
    const invalid = !value.trim() || value.includes("\0") || (source && /\p{Cc}/u.test(value))
      || new TextEncoder().encode(value).length > (source ? 1024 : 65536);
    control.setCustomValidity(invalid ? t("Enter nonempty text within the entry limit. Short phrases cannot contain control characters.") : "");
    return control.reportValidity();
  };
  const save = (text: () => string, fields: [HTMLInputElement | HTMLTextAreaElement, boolean][], edit: () => DictationWordsChange) => {
    const button = label(el("button", { onclick: () => {
      if (busy || loading || !data || !fields.every(([input, source]) => valid(input, source))) return;
      void act(edit(), () => fields.forEach(([input]) => { input.value = ""; }));
    } }), text);
    forms.push(button); return button;
  };
  dictionary.append(label(el("h3"), () => t("Words")), wordList,
    el("div", { class: "words-form" }, word.element, save(() => t("Add word"), [[word.control, true]], () => ({ action: "addWord", word: word.control.value.trim() }))),
    label(el("h3"), () => t("Replacements")), replacements,
    el("div", { class: "words-form" }, from.element, to.element,
      save(() => t("Save replacement"), [[from.control, true], [to.control, false]], () => ({ action: "setReplacement", from: from.control.value.trim(), to: to.control.value }))),
    label(el("p", { class: "dictation-note" }), () => t("Saving the same source updates its replacement without changing its order.")));
  const explanation = el("details", {}, label(el("summary"), () => t("How it is applied")),
    label(el("p", { class: "dictation-note" }), () => t("Whole words, any case; spaces and hyphens match alike. Snippets stay literal. Replacements run in saved order, then dictionary spelling and terminal spoken code. Nothing is learned automatically.")),
    label(el("p", { class: "dictation-note" }), () => t("Short phrases allow up to 1024 UTF-8 bytes; replacement and snippet text allow up to 64 KiB. Line breaks and spaces in saved text are preserved.")));
  const tryButton = label(el("button", { onclick: () => void trySentence() }), () => t("Test sentence"));
  dictionary.append(explanation, el("section", { class: "words-try" }, sentence.element,
    el("label", { class: "words-terminal" }, terminal, label(el("span"), () => t("Terminal context"))), tryButton,
    label(el("p", { class: "dictation-note" }), () => t("Uses saved local rules and current spoken-code/cancellation choices. No microphone, model, clipboard or history is used.")), result));
  snippets.append(snippetList, el("div", { class: "words-form" }, trigger.element, body.element,
    save(() => t("Add snippet"), [[trigger.control, true], [body.control, false]], () => ({ action: "addSnippet", trigger: trigger.control.value.trim(), text: body.control.value }))));
  let data: DictationWords | null = null;
  let loading = false, busy = false, failed = false, actionFailed = false, saved = false;
  let readId = 0, trialId = 0, trying = false;
  let confirmation: { page: number; name: string; edit: DictationWordsChange } | null = null;
  let preview: DictationTry | null = null, tryFailed = false;
  const clearPreview = () => { trialId++; trying = false; preview = null; tryFailed = false; draw(); };
  sentence.control.addEventListener("input", clearPreview);
  terminal.addEventListener("change", clearPreview);
  for (const input of [word.control, from.control, to.control, trigger.control, body.control]) {
    input.addEventListener("input", () => input.setCustomValidity(""));
  }
  const remove = (name: string, edit: DictationWordsChange, page: number, compact = false) => {
    const button = el("button", { text: t("Delete…"), onclick: () => {
      confirmation = { page, name, edit }; draw(); confirmations[page].querySelector("button")?.focus();
    } });
    button.setAttribute("aria-label", t("Delete {entry}", { entry: name }));
    if (compact) button.replaceChildren(icon("close", 14));
    button.disabled = busy || loading; return button;
  };
  const draw = () => {
    const scrolls = [dictionary.scrollTop, snippets.scrollTop];
    for (const [node, text] of labels) node.textContent = text();
    for (const [index, page] of [dictionary, snippets].entries()) {
      page.setAttribute("aria-busy", String(loading || busy));
      refreshes[index].disabled = busy || loading;
      status[index].textContent = actionFailed ? t("Could not save this entry. Check for duplicates or invalid text, then try again.")
        : failed ? t("Could not load Your words. The last loaded entries are kept; use Refresh to try again.")
        : loading ? t("Reading Your words…") : busy ? t("Saving…") : saved ? t("Saved") : "";
      confirmations[index].replaceChildren();
    }
    if (confirmation) {
      const { page, name, edit } = confirmation;
      const cancel = el("button", { text: t("Cancel"), onclick: () => { confirmation = null; draw(); refreshes[page].focus(); } });
      const accept = el("button", { class: "dictation-danger", text: t("Delete"), onclick: () => void act(edit, () => { confirmation = null; }) });
      cancel.disabled = accept.disabled = busy;
      confirmations[page].append(el("p", { text: t("Delete {entry}?", { entry: name }) }), cancel, accept);
    }
    for (const input of [word.control, from.control, to.control, trigger.control, body.control]) input.disabled = busy || loading || !data;
    for (const button of forms) button.disabled = busy || loading || !data;
    wordList.replaceChildren(...(data?.words ?? []).map(w => el("span", { class: "words-chip" }, el("bdi", { text: w }), remove(w, { action: "deleteWord", word: w }, 0, true))));
    const pair = (first: string, text: string, edit: DictationWordsChange, page: number) => el("article", {},
      el("div", {}, el("strong", { text: first }), el("p", { text })), remove(first, edit, page));
    replacements.replaceChildren(...(data?.replacements ?? []).map(r => pair(r.from, r.to, { action: "deleteReplacement", id: r.id }, 0)));
    snippetList.replaceChildren(...(data?.snippets ?? []).map(s => pair(s.trigger, s.text, { action: "deleteSnippet", id: s.id }, 1)));
    for (const [list, empty] of [[wordList, !data?.words.length], [replacements, !data?.replacements.length], [snippetList, !data?.snippets.length]] as const) {
      if (data && empty) list.append(el("p", { class: "dictation-note", text: t("No entries yet.") }));
    }
    tryButton.disabled = busy || loading || trying || !data;
    result.replaceChildren();
    if (trying) result.append(el("p", { text: t("Testing sentence…") }));
    else if (tryFailed) result.append(el("p", { text: t("Could not test this sentence. Try again.") }));
    else if (preview?.cancelled) result.append(el("p", { text: t("This sentence cancels dictation before your rules run.") }));
    else if (preview) {
      const output = el("p", { class: "dictation-text" }); output.setAttribute("dir", "auto");
      for (const s of preview.segments) {
        const node = el(s.kind ? "mark" : "span", { text: s.text });
        if (s.kind) node.title = t("Applied rule: {rule}", { rule: s.rule ?? "" });
        output.append(node);
      }
      result.append(el("p", { class: "dictation-note", text: t("Becomes, before any clean-up:") }), output);
    }
    dictionary.scrollTop = scrolls[0]; snippets.scrollTop = scrolls[1];
  };
  const read = async () => {
    if (loading || busy) return;
    const mine = ++readId; loading = true; draw();
    try { const next = await backend.words(); if (mine === readId) { data = next; failed = false; clearPreview(); } }
    catch { if (mine === readId) failed = true; }
    finally { if (mine === readId) { loading = false; draw(); } }
  };
  const act = async (edit: DictationWordsChange, success: () => void) => {
    if (busy || loading) return;
    const page = snippets.contains(document.activeElement) ? snippets : dictionary;
    const active = document.activeElement as HTMLElement | null;
    const restore = active && page.contains(active) && !confirmations.some(node => node.contains(active)) ? active : refreshes[page === snippets ? 1 : 0];
    busy = true; saved = false; actionFailed = false; clearPreview();
    try { await backend.change(edit); success(); saved = true; }
    catch { actionFailed = true; }
    finally { busy = false; if (saved) await read(); draw(); if (page.isConnected && !page.closest("[hidden]") && restore.isConnected) restore.focus({ preventScroll: true }); }
  };
  const trySentence = async () => {
    if (busy || loading || trying || !data) return;
    const mine = ++trialId; trying = true; tryFailed = false; preview = null; draw();
    try { const next = await backend.trySentence(sentence.control.value, terminal.checked); if (mine === trialId) preview = next; }
    catch { if (mine === trialId) tryFailed = true; }
    finally { if (mine === trialId) { trying = false; draw(); } }
  };
  for (const node of confirmations) node.addEventListener("keydown", e => {
    if (e.key === "Escape" && !busy && confirmation) { const page = confirmation.page; confirmation = null; draw(); refreshes[page].focus(); }
  });
  draw();
  return { dictionary, snippets, refresh: () => { if (!data || failed) void read(); }, relabel: draw };
}
