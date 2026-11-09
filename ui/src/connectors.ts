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
    about: tk("Your open merge requests, review requests and head-pipeline status on GitLab.com. Uses your GitLab CLI (glab) login. Read only; up to 50 items per group. Sign in with glab auth login --hostname gitlab.com."),
  },
];

const ERRORS = [
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
