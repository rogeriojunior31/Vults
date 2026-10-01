// Every connector the settings window offers. Adding one: an entry here, plus the Rust side
// (docs/CONNECTORS.md). The id must match `Connector::id()`.

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
    about:
      "Your open pull requests (checks, approvals, changes requested), reviews requested from you, and checks on the default branch of your recently pushed repositories. Uses the GitHub CLI (gh) you are already logged into; Vultures AI never sees a token.",
  },
];
