import { searchText } from "./search";
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
  const heading = (title: () => string, about: () => string, glyph: "file" | "tag") => el("header", { class: "dictation-heading" },
    el("div", {}, el("h2", { class: "dictation-title" }, icon(glyph, 18), label(el("span"), title)), label(el("p"), about)),
    label(el("button", { onclick: settings }), () => t("Dictation settings")));
  dictionary.append(heading(() => t("Dictionary"), () => t("Names, spelling and replacements for independent dictation."), "file"));
  snippets.append(heading(() => t("Snippets"), () => t("Say a short phrase to insert its saved text unchanged."), "tag"));
  const status = [el("p", { class: "dictation-note" }), el("p", { class: "dictation-note" })];
  const confirmations = [el("div", { class: "dictation-confirmation" }), el("div", { class: "dictation-confirmation" })];
  const searches = [0, 1].map(() => { const input = el("input", { class: "dictation-search" }); input.type = "search"; return input; });
  const refreshes = [0, 1].map(() => label(el("button", { onclick: () => void read() }), () => t("Refresh")));
  for (const [index, page] of [dictionary, snippets].entries()) {
    status[index].setAttribute("role", "status");
    status[index].tabIndex = -1;
    page.append(el("div", { class: "dictation-toolbar" }, searches[index], refreshes[index]), status[index], confirmations[index]);
  }
  const field = (text: () => string, multiline = false) => {
    const control = multiline ? el("textarea") : el("input");
    const element = el("label", { class: multiline ? "words-text-field" : "" }, label(el("span"), text), control);
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
  const replacements = el("tbody");
  const snippetList = el("tbody");
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
  const form = (...children: HTMLElement[]) => {
    const node = el("form", { class: "words-form" }, ...children);
    node.addEventListener("submit", event => event.preventDefault());
    return node;
  };
  const wordCount = el("span", { class: "words-count" });
  const replacementCount = el("span", { class: "words-count" });
  const snippetCount = el("span", { class: "words-count" });
  const pairsTable = (body: HTMLTableSectionElement, output: () => string, name: () => string) => {
    const heading = (text: () => string) => { const cell = label(el("th"), text); cell.scope = "col"; return cell; };
    const table = el("table", { class: "words-pairs" }, el("thead", {}, el("tr", {}, heading(() => t("When I say")), heading(output), heading(() => t("Delete")))), body);
    const region = el("div", { class: "words-table-scroll" }, table); region.tabIndex = 0; region.setAttribute("role", "region");
    table.setAttribute("aria-label", name()); region.setAttribute("aria-label", name());
    return region;
  };
  dictionary.append(
    el("section", { class: "words-section" }, el("h3", {}, label(el("span"), () => t("Words")), wordCount),
      form(word.element, save(() => t("Add word"), [[word.control, true]], () => ({ action: "addWord", word: word.control.value.trim() }))), wordList),
    el("section", { class: "words-section" }, el("h3", {}, label(el("span"), () => t("Replacements")), replacementCount),
      label(el("p", { class: "dictation-note" }), () => t("Saving the same source updates its replacement without changing its order.")),
      form(from.element, to.element,
        save(() => t("Save replacement"), [[from.control, true], [to.control, false]], () => ({ action: "setReplacement", from: from.control.value.trim(), to: to.control.value }))), pairsTable(replacements, () => t("Write"), () => t("Replacements"))));
  const explanation = el("details", {}, label(el("summary"), () => t("How it is applied")),
    label(el("p", { class: "dictation-note" }), () => t("Whole words, any case; spaces and hyphens match alike. Snippets stay literal. Replacements run in saved order, then dictionary spelling and terminal spoken code. Nothing is learned automatically.")),
    label(el("p", { class: "dictation-note" }), () => t("Short phrases allow up to 1024 UTF-8 bytes; replacement and snippet text allow up to 64 KiB. Line breaks and spaces in saved text are preserved.")));
  const tryButton = label(el("button", { onclick: () => void trySentence() }), () => t("Test sentence"));
  dictionary.append(explanation, el("details", { class: "words-try" }, label(el("summary"), () => t("Try a sentence")), sentence.element,
    el("label", { class: "words-terminal" }, terminal, label(el("span"), () => t("Terminal context"))), tryButton,
    label(el("p", { class: "dictation-note" }), () => t("Uses saved local rules and current spoken-code/cancellation choices. No microphone, model, clipboard or history is used.")), result));
  snippets.append(el("section", { class: "words-section" },
    el("h3", {}, label(el("span"), () => t("Snippets")), snippetCount), form(trigger.element, body.element,
    save(() => t("Add snippet"), [[trigger.control, true], [body.control, false]], () => ({ action: "addSnippet", trigger: trigger.control.value.trim(), text: body.control.value }))), pairsTable(snippetList, () => t("Paste"), () => t("Snippets"))));
  let data: DictationWords | null = null;
  let loading = false, busy = false, failed = false, actionFailed = false, saved = false;
  let readId = 0, trialId = 0, trying = false;
  let confirmation: { page: number; name: string; edit: DictationWordsChange; key: string } | null = null;
  let preview: DictationTry | null = null, tryFailed = false;
  const clearPreview = () => { trialId++; trying = false; preview = null; tryFailed = false; draw(); };
  sentence.control.addEventListener("input", clearPreview);
  terminal.addEventListener("change", clearPreview);
  for (const input of [word.control, from.control, to.control, trigger.control, body.control]) {
    input.addEventListener("input", () => input.setCustomValidity(""));
  }
  const actionKey = (edit: DictationWordsChange) => edit.action === "deleteWord" ? `word-${edit.word}` : "id" in edit ? `${edit.action}-${edit.id}` : "";
  const focusEntry = (page: number, key: string) => {
    const root = page === 0 ? dictionary : snippets;
    ([...root.querySelectorAll<HTMLElement>("[data-word-action]")].find(node => node.dataset.wordAction === key) ?? searches[page]).focus({ preventScroll: true });
  };
  const cancelRemoval = () => {
    if (!confirmation) return;
    const { page, key } = confirmation; confirmation = null; draw(); focusEntry(page, key);
  };
  const remove = (name: string, edit: DictationWordsChange, page: number, compact = false) => {
    const button = el("button", { text: t("Delete…"), onclick: () => {
      confirmation = { page, name, edit, key: actionKey(edit) }; draw(); confirmations[page].querySelector("button")?.focus();
    } });
    button.setAttribute("aria-label", t("Delete {entry}", { entry: name }));
    button.dataset.wordAction = actionKey(edit);
    if (compact) button.replaceChildren(icon("close", 14));
    button.disabled = busy || loading; return button;
  };
  const draw = () => {
    const scrolls = [dictionary.scrollTop, snippets.scrollTop];
    const active = document.activeElement as HTMLElement | null;
    const focusedAction = active?.dataset.wordAction;
    const focusedPage = active && snippets.contains(active) ? 1 : 0;
    for (const [node, text] of labels) node.textContent = text();
    for (const [index, page] of [dictionary, snippets].entries()) {
      page.setAttribute("aria-busy", String(loading || busy));
      searches[index].placeholder = index === 0 ? t("Search words and replacements") : t("Search snippets");
      searches[index].setAttribute("aria-label", searches[index].placeholder);
      confirmations[index].setAttribute("role", "region");
      confirmations[index].setAttribute("aria-label", t("Confirm entry deletion"));
      refreshes[index].disabled = busy || loading;
      status[index].setAttribute("role", actionFailed || failed ? "alert" : "status");
      status[index].textContent = actionFailed ? t("Could not save this entry. Check for duplicates or invalid text, then try again.")
        : failed ? t("Could not load Your words. The last loaded entries are kept; use Refresh to try again.")
        : loading ? t("Reading Your words…") : busy ? t("Saving…") : saved ? t("Saved") : "";
      confirmations[index].replaceChildren();
    }
    if (confirmation) {
      const { page, name, edit } = confirmation;
      const cancel = el("button", { text: t("Cancel"), onclick: cancelRemoval });
      const accept = el("button", { class: "dictation-danger", text: t("Delete"), onclick: () => void act(edit, () => { confirmation = null; }) });
      cancel.disabled = accept.disabled = busy;
      cancel.dataset.wordAction = "cancel-confirm"; accept.dataset.wordAction = "delete-confirm";
      confirmations[page].append(el("p", { text: t("Delete {entry}?", { entry: name }) }), cancel, accept);
    }
    for (const input of [word.control, from.control, to.control, trigger.control, body.control]) input.disabled = busy || loading || !data;
    for (const button of forms) button.disabled = busy || loading || !data;
    const queries = searches.map(input => searchText(input.value.trim()));
    const matches = (page: number, ...text: string[]) => searchText(text.join("\n")).includes(queries[page]);
    const words = (data?.words ?? []).filter(word => matches(0, word));
    const pairs = (data?.replacements ?? []).filter(pair => matches(0, pair.from, pair.to));
    const clips = (data?.snippets ?? []).filter(snippet => matches(1, snippet.trigger, snippet.text));
    wordCount.textContent = String(words.length); replacementCount.textContent = String(pairs.length); snippetCount.textContent = String(clips.length);
    const tables = [dictionary.querySelector<HTMLTableElement>("table"), snippets.querySelector<HTMLTableElement>("table")];
    for (const [index, table] of tables.entries()) { const name = index === 0 ? t("Replacements") : t("Snippets"); table?.setAttribute("aria-label", name); table?.parentElement?.setAttribute("aria-label", name); }
    wordList.replaceChildren(...words.map(w => el("span", { class: "words-chip" }, el("bdi", { text: w }), remove(w, { action: "deleteWord", word: w }, 0, true))));
    const pair = (first: string, text: string, edit: DictationWordsChange, page: number) => el("tr", {},
      el("td", { class: "words-source" }, el("strong", { text: first })), el("td", { class: "words-value" }, el("p", { text })), el("td", { class: "words-row-actions" }, remove(first, edit, page)));
    replacements.replaceChildren(...pairs.map(r => pair(r.from, r.to, { action: "deleteReplacement", id: r.id }, 0)));
    snippetList.replaceChildren(...clips.map(s => pair(s.trigger, s.text, { action: "deleteSnippet", id: s.id }, 1)));
    for (const [list, empty, page] of [[wordList, !words.length, 0], [replacements, !pairs.length, 0], [snippetList, !clips.length, 1]] as const) {
      if (data && empty) {
        const filtered = searches[page].value.trim().length > 0;
        const content = list === wordList ? list : el("td");
        if (list !== wordList) { (content as HTMLTableCellElement).colSpan = 3; list.append(el("tr", {}, content)); }
        content.append(el("p", { class: "dictation-note", text: filtered ? t("No entries match your search.") : t("No entries yet.") }));
        if (filtered) content.append(el("button", { text: t("Clear search"), onclick: () => { searches[page].value = ""; draw(); searches[page].focus(); } }));
      }
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
    if (focusedAction) focusEntry(focusedPage, focusedAction);
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
    finally {
      busy = false; if (saved) await read(); draw();
      const focused = document.activeElement as HTMLElement | null;
      const ownsFocus = !focused || focused === document.body || focused === active || confirmations.some(node => node.contains(focused));
      if (ownsFocus && !document.hidden && page.isConnected && !page.closest("[hidden]")) {
        if (confirmation) confirmations[confirmation.page].querySelector("button")?.focus({ preventScroll: true });
        else if (restore.isConnected) restore.focus({ preventScroll: true });
      }
    }
  };
  const trySentence = async () => {
    if (busy || loading || trying || !data) return;
    const mine = ++trialId; trying = true; tryFailed = false; preview = null; draw();
    try { const next = await backend.trySentence(sentence.control.value, terminal.checked); if (mine === trialId) preview = next; }
    catch { if (mine === trialId) tryFailed = true; }
    finally { if (mine === trialId) { trying = false; draw(); } }
  };
  for (const node of confirmations) node.addEventListener("keydown", e => {
    if (e.key === "Escape" && !busy && confirmation) { e.preventDefault(); e.stopPropagation(); cancelRemoval(); }
  });
  for (const search of searches) {
    search.addEventListener("input", draw);
    search.addEventListener("keydown", event => {
      if (event.isComposing || event.keyCode === 229) return;
      if (event.key === "Escape" && search.value) { event.preventDefault(); event.stopPropagation(); search.value = ""; draw(); }
    });
  }
  draw();
  return { dictionary, snippets, refresh: () => { if (!data || failed) void read(); }, relabel: draw };
}
