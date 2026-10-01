# Chatting with Zeca

Open the **Chat** tab, click Zeca on the wire, or choose **Chat…** from the tray icon. Pick **Claude** or
**Codex** at the top of the panel (or **API**, see below); **New** starts a fresh conversation.

## Where it works

A conversation works in the folder of the session in front when it starts (shown at the top of the
panel, for example *in vultures-ai*), and stays there until you press **New**. With no session, it
works in an empty folder of its own.

## What it may do

- It reads files in that folder freely.
- Every command and every edit shows up in the conversation as a card with **Deny** and **Allow**, and
  nothing runs until you click. A card nobody answers is a no.
- Drop a file on the island to ask about it: Zeca picks it up and swallows it, and the next message
  carries it. Dropped files are copied into the app's inbox (up to 20 MB each) and deleted after a week.

## Your subscription, your login

The chat runs the `claude` or `codex` command you already logged into, so it uses your own
subscription and Vultures AI never reads your credentials. Both stream the reply as it is written:
Claude through `claude -p`, Codex through one long-lived `codex app-server`.

Chat turns ignore your hooks, settings and MCP servers, so a chat never shows up on the island as an
agent session, and no saved permission rule lets a command skip the card.

## Without a CLI: an API key

If you don't use Claude Code or Codex, you can chat with an Anthropic API key instead. Paste it in
**Settings → Chat**; an **API** choice then appears at the top of the chat panel.

- The key is kept in your system keyring, never in a file, and the app never shows it again. **Remove**
  deletes it from the keyring, and the next message stops working at once.
- Usage is billed to your API account. The chat uses Claude Opus 5.5.
- This chat only talks: it has no tools, so it can't run commands, edit files or look around your
  project. It reads what you type and the files you drop: images and PDFs as they are, text files
  inline (up to 512 KB).
- If Claude declines a request on safety grounds, the API retries it on another Claude model within the
  same call (Anthropic's server-side fallback). If that model declines too, the bubble says so.
