// Every connector Settings offers. Adding one: an entry here, plus the Rust side
// (docs/contributing/connectors.md). The id must match `Connector::id()`.

import { t, tk } from "./i18n";

export interface ConnectorInfo {
  id: string;
  name: string;
  /** One sentence: what it watches and how it signs in. */
  about: string;
}

export const CONNECTORS: ConnectorInfo[] = [
  {
    id: "github",
    name: "GitHub",
    about: tk(
      "Your open pull requests (checks, approvals, changes requested), reviews requested from you, and checks on the default branch of your recently pushed repositories. Uses the GitHub CLI (gh) you are already logged into; Vults never sees a token.",
    ),
  },
  {
    id: "gitlab",
    name: "GitLab",
    about: tk("Your open merge requests, review requests, diff sizes, reviewers and individual pipeline jobs on GitLab.com, plus default-branch checks and releases from 10 recently active contributed projects. Uses your GitLab CLI (glab) login. Read only. Sign in with glab auth login --hostname gitlab.com."),
  },
  { id: "gitea", name: "Gitea", about: tk("Your open pull requests and review requests on your Gitea instance. Read only; checks, approvals and merges are not supported. Configure the HTTPS base URL and an access token, then turn it on.") },
];

const ERRORS = [
  tk("Gitea returned incomplete or invalid data. The previous snapshot is kept."),
  tk("Could not read Gitea. Check the connection and try again."),
  tk("Check the Gitea URL and token in Settings."),
  tk("Could not read the Gitea connection."),
  tk("Could not save the Gitea connection."),
  tk("The Gitea connection changed. Reload Settings and try again."),
  tk("Enter a valid HTTPS Gitea base URL without credentials, query or fragment."),
  tk("Enter a Gitea access token for this URL."),
  tk("Turn off Gitea before changing its connection."),
  tk("The GitLab CLI (glab) is not installed."),
  tk("Sign in to GitLab.com with glab auth login --hostname gitlab.com."),
  tk("GitLab did not respond in time."),
  tk("Could not read GitLab. Check glab and try again."),
  tk("GitLab returned incomplete or invalid data. The previous snapshot is kept."),
  tk("GitLab returned more than 50 items in a group. The previous snapshot is kept."),
];
export function connectorError(error: string): string {
  if (ERRORS.includes(error)) return t(error);
  const limited = /^rate limited, retrying in (\d+)s$/.exec(error);
  return limited ? t("Rate limited. Retrying in {seconds} seconds.", { seconds: limited[1] }) : error;
}
