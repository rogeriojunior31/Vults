// GH1r: *Check access* for the GitHub connector. One click, one account read and, when a
// repository is named, one repository read; the result names why access fails, never what gh said.
import type { GithubDiagnosis } from "../../bridge";
import { el } from "../../dom";
import { locale, t } from "../../i18n";
import { button, row } from "./pieces";

type Native = { check(repository: string | null): Promise<GithubDiagnosis> };

/** Why the login does not work, as one sentence each. */
export function accountSentence(d: GithubDiagnosis): string {
  switch (d.account) {
    case "ready": return t("Signed in as {login} on {host}.", { login: d.login ?? "?", host: d.host ?? "github.com" });
    case "noCli": return t("The GitHub CLI (gh) is not installed. Install it, then run gh auth login in a terminal.");
    case "signedOut": return t("gh is not signed in. Run gh auth login in a terminal.");
    case "rejected": return t("Your GitHub login expired or was revoked. Run gh auth login again.");
    case "rateLimited": return t("GitHub's rate limit is spent for now. Try again in a few minutes.");
    case "sso": return t("An organization requires single sign-on for this login. Authorize it on GitHub, then check again.");
    case "forbidden": return t("GitHub refused this login's account read.");
    case "incomplete": return t("GitHub answered without the details this check needs. Try again.");
    default: return t("GitHub could not be reached. Check the connection and try again.");
  }
}

/** What this login may do in the named repository, or why it cannot read it. */
export function repositorySentence(r: NonNullable<GithubDiagnosis["repository"]>): string {
  const name = r.name;
  switch (r.access) {
    case "ready":
      if (r.archived) return t("{name} is archived: you can read it, and nobody can push to it.", { name });
      return r.push ? t("You can read and push to {name}.", { name }) : t("You can read {name}, but not push to it.", { name });
    case "notFound": return t("{name} was not found, or this login can't see it.", { name });
    case "sso": return t("An organization requires single sign-on for {name}. Authorize this login for it on GitHub.", { name });
    case "forbidden": return t("This login may not read {name}.", { name });
    case "rateLimited": return t("GitHub's rate limit is spent for now. Try again in a few minutes.");
    case "incomplete": return t("GitHub answered without the details this check needs. Try again.");
    default: return t("{name} could not be checked. Try again.", { name });
  }
}

/** `lastAccount` is the connector's last successful identity: on a failed check it may be stale.
 *  Built once and kept across Settings' redraws; `relabel` follows a change of language. */
export function createGithubCheck(native: Native, lastAccount: () => string | null) {
  const repo = el("input", { class: "field" });
  repo.placeholder = "owner/name";
  repo.maxLength = 200;
  repo.autocomplete = "off";
  repo.spellcheck = false;
  const result = el("div", { class: "github-check-result" });
  result.setAttribute("role", "status");
  let busy = false;
  let shown: GithubDiagnosis | "invalid" | "failed" | null = null;
  const paint = () => {
    if (shown === null) return result.replaceChildren();
    if (shown === "invalid" || shown === "failed") {
      return result.replaceChildren(el("p", { class: "note error", text: shown === "invalid" ? t("Enter a repository as owner/name.") : t("GitHub was checked many times today, or the check failed. Try again later.") }));
    }
    const d = shown;
    const lines = [el("p", { class: d.account === "ready" ? "note" : "note error", text: accountSentence(d) })];
    const last = lastAccount();
    if (d.account !== "ready" && last) lines.push(el("p", { class: "note warn", text: t("The last successful account, {account}, may no longer work.", { account: last }) }));
    if (d.repository) lines.push(el("p", { class: d.repository.access === "ready" ? "note" : "note error", text: repositorySentence(d.repository) }));
    result.replaceChildren(...lines);
  };
  const run = async () => {
    if (busy) return;
    busy = true;
    check.disabled = true;
    check.textContent = t("Checking…");
    shown = null;
    paint();
    try {
      shown = await native.check(repo.value.trim() || null);
    } catch (error) {
      shown = String(error).includes("owner/name") ? "invalid" : "failed";
    } finally {
      busy = false;
      check.disabled = false;
      check.textContent = t("Check access");
      paint();
    }
  };
  const check = button(t("Check access"), () => void run());
  repo.addEventListener("keydown", (e) => { if (e.key === "Enter") void run(); });
  const element = el("div", { class: "github-check" });
  let language = "";
  const relabel = () => {
    if (language === locale()) return;
    language = locale();
    repo.setAttribute("aria-label", t("Repository to check (optional)"));
    if (!busy) check.textContent = t("Check access");
    element.replaceChildren(
      row(
        t("Check access"),
        t("Asks gh which account it uses and what it may do in a repository you name. At most two reads, on your click, even with the connector off."),
        el("div", { class: "actions" }, repo, check),
      ),
      result,
    );
    paint();
  };
  relabel();
  return { element, relabel };
}
