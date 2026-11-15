// Check Settings' GitHub *Check access* (GH1r) without a browser server: one sentence per cause,
// the stale identity warning, the optional repository and one check at a time.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build } from "vite";

const nodes = node => [node, ...node.children.flatMap(nodes)];
class Element {
  children = []; listeners = {}; attributes = {}; dataset = {}; style = { setProperty() {} };
  className = ""; value = ""; disabled = false; placeholder = ""; maxLength = 0;
  constructor(tag) { this.tagName = tag.toUpperCase(); }
  classList = {
    contains: name => this.className.split(" ").includes(name),
    add: name => { if (!this.classList.contains(name)) this.className += ` ${name}`; },
    remove: name => { this.className = this.className.split(" ").filter(v => v !== name).join(" "); },
    toggle: (name, on = !this.classList.contains(name)) => this.classList[on ? "add" : "remove"](name),
  };
  set textContent(text) { this.children = []; this.text = text; }
  get textContent() { return (this.text ?? "") + this.children.map(n => n.textContent).join(""); }
  append(...children) { for (const n of children) { n.parentElement = this; this.children.push(n); } }
  replaceChildren(...children) { this.text = ""; this.children = []; this.append(...children); }
  matches(selector) { return selector.split(",").some(s => this.tagName === s.trim().toUpperCase()); }
  querySelectorAll(selector) { return nodes(this).slice(1).filter(n => n.matches(selector)); }
  querySelector(selector) { return this.querySelectorAll(selector)[0] ?? null; }
  addEventListener(event, fn) { this.listeners[event] = fn; }
  setAttribute(name, value) { this.attributes[name] = String(value); }
  getAttribute(name) { return this.attributes[name]; }
  hasAttribute(name) { return name in this.attributes; }
  removeAttribute(name) { delete this.attributes[name]; }
  click() { if (!this.disabled) this.listeners.click?.({ target: this }); }
  focus() {}
}
globalThis.document = {
  createElement: tag => new Element(tag),
  createElementNS: (_ns, tag) => new Element(tag),
  createTextNode: text => { const n = new Element("#text"); n.text = text; return n; },
};
globalThis.window = globalThis;
globalThis.localStorage = { getItem: () => null, setItem() {} };
Object.defineProperty(globalThis, "navigator", { value: { language: "en-US", languages: ["en-US"] }, configurable: true });

const root = fileURLToPath(new URL("../ui/", import.meta.url));
const bundles = await build({ configFile: false, root, logLevel: "silent", build: {
  write: false, minify: false, lib: { entry: `${root}src/surfaces/settings/github-check.ts`, formats: ["es"] },
} });
const chunk = bundles[0].output.find(part => part.type === "chunk");
const { accountSentence, repositorySentence, createGithubCheck } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);

const states = ["ready", "noCli", "signedOut", "rejected", "rateLimited", "sso", "forbidden", "incomplete", "failed"];
const said = states.map(account => accountSentence({ account, login: "person", host: "github.com", repository: null }));
assert.equal(new Set(said).size, states.length, "each cause of a failed login reads differently");
assert.equal(said[0], "Signed in as person on github.com.");
assert.match(said[2], /gh auth login/);
assert.match(said[3], /expired or was revoked/);
const repo = (access, over = {}) => repositorySentence({ name: "o/r", access, push: null, archived: false, private: false, ...over });
assert.equal(repo("ready", { push: true }), "You can read and push to o/r.");
assert.equal(repo("ready", { push: false }), "You can read o/r, but not push to it.");
assert.match(repo("ready", { push: true, archived: true }), /archived/);
assert.match(repo("notFound"), /private and this login cannot see it/);
assert.match(repo("sso"), /single sign-on for o\/r/);

const flush = () => new Promise(resolve => setTimeout(resolve, 0));
const asked = [];
let answer;
const check = createGithubCheck({ check: async r => { asked.push(r); return answer; } }, () => "person (github.com)");
const input = check.element.querySelector("input");
const button = check.element.querySelectorAll("button").find(b => b.textContent === "Check access");
const result = () => check.element.children.find(n => n.className.includes("github-check-result"));
assert.equal(input.getAttribute("aria-label"), "Repository to check (optional)");
assert.equal(result().getAttribute("role"), "status");

answer = { account: "rejected", login: null, host: null, repository: null };
button.click();
await flush();
assert.deepEqual(asked, [null], "an empty field checks the account only");
assert.match(result().textContent, /expired or was revoked/);
assert.match(result().textContent, /The last successful account, person \(github\.com\), may no longer work\./);

answer = { account: "ready", login: "person", host: "github.com", repository: { name: "o/r", access: "ready", push: false, archived: false, private: true } };
input.value = "  o/r  ";
button.click();
assert.equal(button.disabled, true, "one check at a time");
button.click();
await flush();
assert.deepEqual(asked, [null, "o/r"], "trimmed, and a second click while busy asks nothing");
assert.match(result().textContent, /Signed in as person/);
assert.match(result().textContent, /You can read o\/r, but not push to it\./);
assert.ok(!result().textContent.includes("may no longer work"), "a working login is not called stale");
assert.equal(button.disabled, false);

console.log("check-github-access: ok");
