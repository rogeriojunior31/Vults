# Zeca's memory

Zeca can remember you, your projects and what your agents did. It is off until you turn it on,
is kept on this computer, and only you write it.

## Turning it on

Turn it on in **Settings → Chat → Memory → Zeca's memory**. The app then keeps a journal in the
local `vults.sqlite` for 90 days: each agent turn's time, steps and changes, each card's answer with
its command or path, and the lines of Zeca's chat, secrets redacted before anything is written.
The section's other switches appear once this one is on.

Off, nothing of the memory reaches a model (no facts, no recall, no journal): the Nest's **Memory**
tab shows only *Zeca's memory is off…*, **Roosts** shows no memory facts, and `/remember` is
refused. Turning it off stops the writing and keeps what was written.

What Zeca remembers is a git repository of small Markdown files in `~/.local/share/vults/memory/`,
each change a commit. **Settings → Activity → Clear history** removes the journal, its day
summaries and recall data, the memory proposals, the sleep runs, the conversations, the trace, the
audit log and the flock snapshot file, but not that folder: to erase his facts, delete it.

## What joins a conversation

When a conversation starts, his memory joins it after his instructions: who you are, what he
learned, how to speak to you, and the memory of the project it works in. It stays fixed for the
whole conversation; a fact added meanwhile joins the next one. Turning the memory off doesn't take
it from a conversation already running; reopened later from the list, it starts without his facts.

Before each message, the app searches the journal and the days' summaries for what your words are
about, on this computer, and up to three passages join the message as the `<memories>` block,
context and never instructions. Today's chat is left out, and so is anything a conversation said
after it read content from outside. The passages go to the chat's provider with your message. The
API chat can also ask for more with its `recall` tool, and suggest a fact with `memory_propose`,
which only writes a proposal.

## Writing his memory

- **`/remember <fact>`** in the chat, with the memory on, saves a fact to what Zeca learned; **`/remember project:
  <fact>`** to the project the conversation is in. **`/forget <words>`** removes the one fact that
  holds them (with several, it lists them).
- **What Zeca proposes** waits in the Nest's **Memory** tab, with where it goes (a project by its
  name and folder) and the change as a diff: **Remember** saves it, **Turn down** drops it.
- The Memory tab also lists what he remembers, file by file with how full each is, and the history:
  who wrote each change (you, or Zeca with your yes) and **Undo**, which takes it back in a new
  change (refused when a later change touched the same file; never offered on one Vults did not
  write).

A fact never gets in twice: `/remember` and every proposal are refused when another fact already
says it, case, spaces and the final stop aside, and the same fact decided in the last month is not
proposed again. A proposal passes the memory's scan (no hidden characters, no words that address a
model, no key shapes).

## What he does on his own

Each is a switch under **Settings → Chat → Memory**, off until you turn it on. Haiku is Claude
Code's smallest model, through your `claude` and subscription, with no tools and none of your
settings; each call goes through the network gate and the local trace
([Safety](../safety.md#every-request-vults-makes)).

| Feature | Switch | What it reads | Model | Requests a day |
|---|---|---|---|---|
| A summary of each past day (2,000 bytes at most) | **A summary of each day** | One day of the journal, only days since you turned it on | Haiku | 8 |
| Up to three facts to remember, when you start or open another conversation | **Suggest what to remember** | The conversation you left, redacted, with at least 2 of your messages and no outside content | Haiku | 20 |
| Merges, and a debrief shown as *While you were away* | **Let Zeca sleep on it** | What he remembers about you and across projects, and the last 7 days' summaries | Haiku | 4 |
| Recall by meaning, not only by words | **Recall by meaning** | Each summary, and each message you send | `nomic-embed-text` in an [Ollama](https://ollama.com) on `127.0.0.1` | none leave the computer |

The summary runs in an hourly pass: each past day without a summary, up to three a pass (it catches
up after days away), three tries per day while the app runs. The embeddings run in that pass, for
each summary without one, and before each chat message (`ollama pull nomic-embed-text` first).

The sleep comes once a day, from two in the morning and while nothing is working. Even without its
switch, it proposes removing a fact that repeats another in its section, with no model. With it, it
also proposes merges and writes the debrief, in your language, marked as his reading of the
summaries. Every change waits for your yes.

## Project memory

The Nest's **Roosts** tab has a card for each repository a session works in: its folders and
branches, its sessions, your pull requests and checks (from the GitHub card), and what Zeca
remembers of the project. It reads only this computer. A conversation that starts in a repository
brings that memory along.

### Sharing AGENTS.md with Claude

When a repository has an `AGENTS.md` (the project memory Codex reads) that its `CLAUDE.md` does not
import, its card offers **Share it with Claude…**. It shows the change (`@AGENTS.md` added at the
end of `CLAUDE.md`, `@../AGENTS.md` at the end of `.claude/CLAUDE.md`, or a new `CLAUDE.md`) and
writes it only when you click **Apply**, over the very file you saw, after a dated backup in
`~/.local/share/vults/backups/`. Then Claude Code and Codex read the same project memory.

## Through Claude Code

The chat hands Claude Code Zeca's own MCP server, `vults-zeca`, with `--mcp-config` (your Claude
config is never touched): `flock_list` (your sessions now), `recall` and `journal_day` (his
journal), `roost_memory` (the project's memory) and `memory_propose`; only `flock_list` with the
memory off. Claude Code asks with a card before each call your rules don't allow, and a proposal still waits in the Memory
tab. These calls do not mark the conversation as having read outside content. The server is this
app run again (`vults --zeca-mcp`), and sends nothing anywhere. Codex reads MCP servers only from
its own config, so it does not get them.
