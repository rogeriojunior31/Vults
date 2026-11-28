// Chat Markdown is built as DOM nodes, never HTML. Links are text and never followed.
import { el } from "../dom";
import { t } from "../i18n";
import { copyTextButton } from "../clipboard";

/** Languages whose comments start with `#`, with `--`, or with neither `//` nor `#` (data). */
const HASH = new Set(["py", "python", "sh", "bash", "zsh", "shell", "toml", "yaml", "yml", "rb", "ruby", "ps1", "powershell", "pwsh", "dockerfile", "make", "makefile", "r", "nix"]);
const DASH = new Set(["sql", "lua", "haskell", "hs"]);
const DATA = new Set(["json", "jsonc", "diff", "text", "txt", "plain", "markdown", "md", "csv", "log"]);
const KEYWORDS = new Set(("as async await break case catch class const continue crate def default defer del do elif else enum "
  + "except export extends false fn for from func function go if impl import in interface is let loop match mod module "
  + "mut new nil None not null or package pass pub raise return self Self static struct super switch then this throw "
  + "trait true True False try type typeof undefined unsafe use var void where while with yield echo fi done esac local "
  + "select insert update delete into values create table alter drop join on group order by limit").split(" "));
/** Past this many characters a block is drawn plain: the stream repaints it many times a second. */
const HIGHLIGHT_MAX = 20_000;

/** Code as colored spans (comments, strings, numbers, keywords), built as DOM nodes, never HTML. */
function highlighted(code: string, lang: string): Node[] {
  const l = lang.toLowerCase();
  if (!l || DATA.has(l) || code.length > HIGHLIGHT_MAX) {
    if (l === "diff") return code.split("\n").flatMap((line, i) => {
      const kind = line.startsWith("+") ? "add" : line.startsWith("-") ? "del" : line.startsWith("@@") ? "hunk" : "";
      const nodes: Node[] = i ? [document.createTextNode("\n")] : [];
      nodes.push(kind ? el("span", { class: `tok-${kind}`, text: line }) : document.createTextNode(line));
      return nodes;
    });
    return [document.createTextNode(code)];
  }
  const comment = HASH.has(l) ? /#[^\n]*/.source : DASH.has(l) ? /--[^\n]*/.source : /\/\/[^\n]*|\/\*[\s\S]*?(?:\*\/|$)/.source;
  const strings = /"(?:[^"\\\n]|\\.)*"?|'(?:[^'\\\n]|\\.)*'?|`(?:[^`\\]|\\.)*`?/.source;
  const numbers = /\b0x[\da-f]+\b|\b\d[\d_]*(?:\.\d+)?(?:e[+-]?\d+)?\b/.source;
  const re = new RegExp(`(${comment})|(${strings})|(${numbers})|([A-Za-z_]\\w*)`, "gi");
  const out: Node[] = [];
  let last = 0;
  for (const m of code.matchAll(re)) {
    const [token, isComment, isString, isNumber, word] = m;
    const kind = isComment ? "comment" : isString ? "string" : isNumber ? "number" : word && KEYWORDS.has(word) ? "keyword" : "";
    if (!kind) continue;
    if (m.index! > last) out.push(document.createTextNode(code.slice(last, m.index)));
    out.push(el("span", { class: `tok-${kind}`, text: token }));
    last = m.index! + token.length;
  }
  if (last < code.length) out.push(document.createTextNode(code.slice(last)));
  return out;
}

/** A fenced block: its language (the fence's first word) and Copy over the code. */
function codeBlock(code: string, info = ""): HTMLElement {
  const lang = info.trim().split(/\s+/)[0] ?? "";
  const copy = copyTextButton(code, "copy", t("Copy code"));
  const pre = el("pre", {}, ...highlighted(code, lang));
  pre.tabIndex = 0;
  pre.setAttribute("role", "region");
  pre.setAttribute("aria-label", lang ? t("Code in {language}", { language: lang }) : t("Code"));
  return el(
    "div",
    { class: "codeblock" },
    el("div", { class: "code-head" }, el("span", { class: "code-lang", text: lang }), copy),
    pre,
  );
}

/** `[label](url)`, `*italic*` and `_italic_` too, for full chat replies. A link is never
 *  followed: its label shows with the address beside it, to copy. */
function inlineFull(text: string): Node[] {
  const out: Node[] = [];
  const re = /(`[^`\n]+`|\*\*[^*\n]+\*\*|~~[^~\n]+~~|\[[^\[\]\n]+\]\([^)\s]+\)|https?:\/\/[^\s<>()]*[^\s<>().,;:!?'"]|(?<![\w*])\*[^*\n]+\*(?!\w)|(?<!\w)_[^_\n]+_(?!\w))/g;
  let last = 0;
  for (const m of text.matchAll(re)) {
    if (m.index! > last) out.push(document.createTextNode(text.slice(last, m.index)));
    const token = m[0];
    if (token.startsWith("`")) out.push(el("code", { text: token.slice(1, -1) }));
    else if (token.startsWith("**")) out.push(el("strong", { text: token.slice(2, -2) }));
    else if (token.startsWith("~~")) out.push(el("s", { text: token.slice(2, -2) }));
    // A bare address: shown as one, to copy; never followed.
    else if (token.startsWith("http")) out.push(el("span", { class: "link", text: token }));
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

/** The same readable reply structure in Quick and Room, including incomplete streamed fences. */
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
        const region = el("div", { class: "table" }, table);
        region.tabIndex = 0;
        region.setAttribute("role", "region");
        region.setAttribute("aria-label", t("Table"));
        frag.append(region);
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
          // A task list's box, and how deep the item sits (two spaces a level).
          const task = /^\[([ xX])\]\s+(.*)$/.exec(next[2]);
          const li = el("li", {}, ...(task ? [el("span", { class: `task${task[1] === " " ? "" : " done"}`, text: task[1] === " " ? "☐" : "☑" }), ...inlineFull(task[2])] : inlineFull(next[2])));
          const depth = Math.min(3, Math.floor((/^\s*/.exec(lines[n])![0].length) / 2));
          if (depth) li.dataset.depth = String(depth);
          if (task) li.classList.add("task-item");
          list.append(li);
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
