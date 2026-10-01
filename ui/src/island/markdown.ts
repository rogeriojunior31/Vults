// Just enough Markdown for chat replies: fenced code (with a copy button), inline code, bold, and
// lists. Built as DOM nodes, never innerHTML, so a reply can't inject anything.
import { el } from "../dom";

/** `**bold**` and `` `code` `` inside one line. */
function inline(text: string): Node[] {
  const out: Node[] = [];
  const re = /(`[^`\n]+`|\*\*[^*\n]+\*\*)/g;
  let last = 0;
  for (const m of text.matchAll(re)) {
    if (m.index! > last) out.push(document.createTextNode(text.slice(last, m.index)));
    const token = m[0];
    if (token.startsWith("`")) out.push(el("code", { text: token.slice(1, -1) }));
    else out.push(el("strong", { text: token.slice(2, -2) }));
    last = m.index! + token.length;
  }
  if (last < text.length) out.push(document.createTextNode(text.slice(last)));
  return out;
}

function codeBlock(code: string): HTMLElement {
  const copy = el("button", {
    class: "copy",
    text: "Copy",
    onclick: () => {
      void navigator.clipboard?.writeText(code).then(() => {
        copy.textContent = "Copied";
        window.setTimeout(() => (copy.textContent = "Copy"), 1200);
      });
    },
  });
  return el("div", { class: "codeblock" }, copy, el("pre", { text: code }));
}

export function renderLite(text: string): DocumentFragment {
  const frag = document.createDocumentFragment();
  // Fences split the text; an unclosed fence (still streaming) is code to the end.
  const parts = text.split(/^```[^\n]*\n?/m);
  parts.forEach((part, i) => {
    if (i % 2 === 1) {
      frag.append(codeBlock(part.replace(/\n$/, "")));
      return;
    }
    let list: HTMLElement | null = null;
    let para: string[] = [];
    const flush = () => {
      if (para.length) frag.append(el("p", {}, ...inline(para.join("\n"))));
      para = [];
    };
    for (const line of part.split("\n")) {
      const item = /^\s*(?:[-*•]|\d+[.)])\s+(.*)$/.exec(line);
      if (item) {
        flush();
        list ??= el("ul", {});
        list.append(el("li", {}, ...inline(item[1])));
        continue;
      }
      if (list) {
        frag.append(list);
        list = null;
      }
      if (line.trim() === "") flush();
      else para.push(line);
    }
    flush();
    if (list) frag.append(list);
  });
  return frag;
}
