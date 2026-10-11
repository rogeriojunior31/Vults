// Settings → Approvals → Policies (plan-zeca W2, ADR 0014): the user writes a policy, sees it as a
// diff against the one in place, and accepts it with a click; revoking one makes its cards ask
// again. The app records each click as a consent, and only those answer.
import {
  Bridge,
  type PolicyError,
  type PolicyPreview,
  type PolicyRow,
} from "../../bridge";
import { el } from "../../dom";
import { locale, t, tk } from "../../i18n";
import { button, diff } from "./pieces";

const ERRORS: Record<PolicyError["kind"], string> = {
  "unknown-word": tk("Line {line}: start it with allow, deny, ask, project or agents."),
  "unknown-what": tk("Line {line}: after allow, deny or ask comes read, read-external, write, external, command or tool."),
  destructive: tk("Line {line}: destructive actions always ask; a policy can't answer them."),
  missing: tk("Line {line}: something is missing after the first word."),
  metacharacters: tk("Line {line}: a command can't hold shell symbols, quotes or globs, only a last * for more words."),
  "not-absolute": tk("Line {line}: the project is a whole folder path, such as ~/Projects/site."),
  twice: tk("Line {line}: project and agents come once each."),
  "unknown-agent": tk("Line {line}: the agents are claude, codex, gemini, opencode and qwen."),
  empty: tk("Line {line}: a policy needs at least one allow, deny or ask line."),
  "never-allowed": tk("Line {line}: a shell is allowed by its commands, and an MCP tool never; deny or ask them instead."),
};

function template(): string {
  return [
    `# ${t("A policy: one rule per line. Lines that start with # are notes.")}`,
    "project ~/Projects/site",
    "allow read",
    "allow command git status",
    "allow command git diff *",
    "deny external",
    "",
  ].join("\n");
}

let policies: PolicyRow[] = [];
/** The text being written; null while the editor is closed. */
let draft: string | null = null;
/** The policy being edited (its consent), when the draft started from one. */
let editing: number | null = null;
/** The review of `draft` the Accept click would take. */
let review: { text: string; preview: PolicyPreview } | null = null;
let message: { text: string; error: boolean } | null = null;

export async function refreshPolicies(render: () => void): Promise<void> {
  try {
    policies = await Bridge.policiesList();
  } catch (e) {
    policies = [];
    message = { text: String(e), error: true };
  }
  render();
}

function editor(render: () => void): HTMLElement {
  const area = el("textarea", { class: "policy-editor" });
  area.spellcheck = false;
  area.value = draft ?? "";
  area.rows = 10;
  area.addEventListener("input", () => {
    draft = area.value;
    // A change after the review needs a new one: the click accepts only what was shown.
    if (review && review.text !== draft) {
      const at = area.selectionStart;
      review = null;
      render();
      // The page is drawn anew: the caret moves to the new editor.
      const fresh =
        document.querySelector<HTMLTextAreaElement>(".policy-editor");
      fresh?.focus();
      fresh?.setSelectionRange(at, at);
    }
  });
  const preview = review?.preview;
  const reviewed = review?.text;
  return el(
    "section",
    { class: "card policy-edit" },
    area,
    preview?.error
      ? el("p", {
          class: "note error",
          text: t(ERRORS[preview.error.kind], { line: preview.error.line }),
        })
      : null,
    preview && !preview.error && preview.diff ? diff(preview.diff) : null,
    preview && !preview.error && preview.revokes
      ? el("p", {
          class: "note warn",
          text: t("Accepting also revokes the policy you edited, for {project}.", {
            project: preview.revokes === "everywhere" ? t("Everywhere") : preview.revokes,
          }),
        })
      : null,
    preview && !preview.error && !preview.diff
      ? el("p", {
          class: "note",
          text: t("This is the policy in place already."),
        })
      : null,
    el(
      "div",
      { class: "actions" },
      button(t("Review"), () => {
        const text = draft ?? "";
        void Bridge.policyPreview(text, editing).then(
          (p) => {
            review = { text, preview: p };
            message = null;
            render();
          },
          (e) => {
            message = { text: String(e), error: true };
            render();
          },
        );
      }),
      preview && !preview.error && preview.diff && reviewed !== undefined
        ? button(
            t("Accept"),
            () => {
              void Bridge.policyAccept(reviewed, preview.diff, editing).then(
                () => {
                  draft = null;
                  editing = null;
                  review = null;
                  message = {
                    text: t("Policy accepted. It answers from now on."),
                    error: false,
                  };
                  void refreshPolicies(render);
                },
                (e) => {
                  message = { text: String(e), error: true };
                  render();
                },
              );
            },
            true,
          )
        : null,
      button(t("Cancel"), () => {
        draft = null;
        editing = null;
        review = null;
        render();
      }),
    ),
  );
}

function policyCard(p: PolicyRow, render: () => void): HTMLElement {
  const when = new Date(p.at * 1000).toLocaleDateString(locale());
  return el(
    "section",
    { class: "card rows" },
    el(
      "div",
      { class: "row" },
      el(
        "div",
        { class: "row-text" },
        el("div", { class: "row-title", text: p.project ?? t("Everywhere") }),
        el("div", {
          class: "row-about",
          text: t("Accepted on {date}", { date: when }),
        }),
      ),
      button(t("Edit"), () => {
        draft = p.text;
        editing = p.consent;
        review = null;
        render();
      }),
      button(t("Revoke"), () => {
        void Bridge.policyRevoke(p.consent).then(
          () => {
            message = {
              text: t("Policy revoked. Its cards ask again."),
              error: false,
            };
            void refreshPolicies(render);
          },
          (e) => {
            message = { text: String(e), error: true };
            render();
          },
        );
      }),
    ),
    el("pre", { class: "policy-text", text: p.text }),
  );
}

/** The section, under the Always rules. */
export function policiesSection(render: () => void): HTMLElement[] {
  return [
    el("h2", { text: t("Policies") }),
    el("p", {
      class: "lede",
      text: t(
        "A policy answers cards for you, in a folder or everywhere: what to allow, deny or always ask. You write it, see it as a diff and accept it with a click. A destructive action always asks, a deny beats an allow, an allow reaches only the session's folder, and every answer goes to the audit log.",
      ),
    }),
    message
      ? el("p", {
          class: `note ${message.error ? "error" : "ok"}`,
          text: message.text,
        })
      : null,
    ...policies.map((p) => policyCard(p, render)),
    !policies.length && draft === null
      ? el(
          "section",
          { class: "card" },
          el("p", {
            class: "note",
            text: t("No policy. Every card asks you."),
          }),
        )
      : null,
    draft !== null
      ? editor(render)
      : el(
          "div",
          { class: "actions" },
          button(t("New policy"), () => {
            draft = template();
            editing = null;
            review = null;
            render();
          }),
        ),
  ].filter((x): x is HTMLElement => x !== null);
}
