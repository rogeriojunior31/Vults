// Just enough Markdown for chat replies: fenced code (with a copy button), inline code, bold, and
// lists. Built as DOM nodes, never innerHTML, so a reply can't inject anything.
import { el } from "../dom";
import { t } from "../i18n";

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

/** A fenced block: its language (the fence's first word) and Copy over the code. */
function codeBlock(code: string, info = ""): HTMLElement {
  const lang = info.trim().split(/\s+/)[0] ?? "";
  const copy = el("button", {
    class: "copy",
    text: t("Copy"),
    onclick: () => {
      void navigator.clipboard?.writeText(code).then(() => {
        copy.textContent = t("Copied");
        window.setTimeout(() => (copy.textContent = t("Copy")), 1200);
      });
    },
  });
  return el(
    "div",
    { class: "codeblock" },
    el("div", { class: "code-head" }, el("span", { class: "code-lang", text: lang }), copy),
    el("pre", { text: code }),
  );
}

export function renderLite(text: string): DocumentFragment {
  const frag = document.createDocumentFragment();
  // Fences split the text; an unclosed fence (still streaming) is code to the end.
  // The capture keeps each fence's info: [text, info, code, text, …].
  const parts = text.split(/^```([^\n]*)\n?/m);
  parts.forEach((part, i) => {
    if (i % 3 === 1) return;
    if (i % 3 === 2) {
      frag.append(codeBlock(part.replace(/\n$/, ""), parts[i - 1]));
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

/** `[label](url)`, `*italic*` and `_italic_` too, for the Nest's full Markdown. A link is never
 *  followed: its label shows with the address beside it, to copy. */
function inlineFull(text: string): Node[] {
  const out: Node[] = [];
  const re = /(`[^`\n]+`|\*\*[^*\n]+\*\*|\[[^\[\]\n]+\]\([^)\s]+\)|(?<![\w*])\*[^*\n]+\*(?!\w)|(?<!\w)_[^_\n]+_(?!\w))/g;
  let last = 0;
  for (const m of text.matchAll(re)) {
    if (m.index! > last) out.push(document.createTextNode(text.slice(last, m.index)));
    const token = m[0];
    if (token.startsWith("`")) out.push(el("code", { text: token.slice(1, -1) }));
    else if (token.startsWith("**")) out.push(el("strong", { text: token.slice(2, -2) }));
    else if (token.startsWith("[")) {
      const link = /^\[([^\]]+)\]\(([^)]+)\)$/.exec(token)!;
      out.push(el("span", { class: "link", text: link[1] }), el("span", { class: "link-url", text: ` (${link[2]})` }));
    } else out.push(el("em", { text: token.slice(1, -1) }));
    last = m.index! + token.length;
  }
  if (last < text.length) out.push(document.createTextNode(text.slice(last)));
  return out;
}

/** The cells of a table row: `| a | b |`. */
function cells(line: string): string[] {
  return line.trim().replace(/^\|/, "").replace(/\|$/, "").split("|").map((c) => c.trim());
}

const TABLE_RULE = /^\s*\|?\s*:?-{3,}:?\s*(\|\s*:?-{3,}:?\s*)*\|?\s*$/;

/** Everything a reply may use in the Nest: headings, quotes, rules, tables, numbered and plain
 *  lists, emphasis and links (shown, never followed), on top of what `renderLite` reads. */
export function renderFull(text: string): DocumentFragment {
  const frag = document.createDocumentFragment();
  // The capture keeps each fence's info: [text, info, code, text, …].
  const parts = text.split(/^```([^\n]*)\n?/m);
  parts.forEach((part, i) => {
    if (i % 3 === 1) return;
    if (i % 3 === 2) {
      frag.append(codeBlock(part.replace(/\n$/, ""), parts[i - 1]));
      return;
    }
    const lines = part.split("\n");
    let para: string[] = [];
    const flush = () => {
      if (para.length) frag.append(el("p", {}, ...inlineFull(para.join("\n"))));
      para = [];
    };
    for (let n = 0; n < lines.length; n++) {
      const line = lines[n];
      const heading = /^(#{1,6})\s+(.*)$/.exec(line);
      if (heading) {
        flush();
        // Never larger than the chat's own titles: h3 and below.
        const level = Math.min(6, heading[1].length + 2);
        frag.append(el(`h${level}` as "h3", {}, ...inlineFull(heading[2])));
        continue;
      }
      if (/^\s*([-*_])(\s*\1){2,}\s*$/.test(line)) {
        flush();
        frag.append(el("hr"));
        continue;
      }
      if (line.includes("|") && TABLE_RULE.test(lines[n + 1] ?? "")) {
        flush();
        const table = el("table");
        table.append(el("thead", {}, el("tr", {}, ...cells(line).map((c) => el("th", {}, ...inlineFull(c))))));
        const body = el("tbody");
        n += 2;
        for (; n < lines.length && lines[n].includes("|"); n++)
          body.append(el("tr", {}, ...cells(lines[n]).map((c) => el("td", {}, ...inlineFull(c)))));
        n--;
        table.append(body);
        frag.append(el("div", { class: "table" }, table));
        continue;
      }
      if (/^\s*>/.test(line)) {
        flush();
        const quoted: string[] = [];
        for (; n < lines.length && /^\s*>/.test(lines[n]); n++) quoted.push(lines[n].replace(/^\s*>\s?/, ""));
        n--;
        frag.append(el("blockquote", {}, ...inlineFull(quoted.join("\n"))));
        continue;
      }
      const item = /^\s*([-*•]|\d+[.)])\s+(.*)$/.exec(line);
      if (item) {
        flush();
        const ordered = /\d/.test(item[1]);
        const list = el(ordered ? "ol" : "ul");
        for (; n < lines.length; n++) {
          const next = /^\s*([-*•]|\d+[.)])\s+(.*)$/.exec(lines[n]);
          if (!next || /\d/.test(next[1]) !== ordered) break;
          list.append(el("li", {}, ...inlineFull(next[2])));
        }
        n--;
        frag.append(list);
        continue;
      }
      if (line.trim() === "") flush();
      else para.push(line);
    }
    flush();
  });
  return frag;
}
