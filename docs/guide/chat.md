# Chatting with Zeca

Open the **Chat** tab, choose **Chat…** from the tray icon, or drop a file on the island. Zeca sits on
the left of the conversation: he thinks while the reply comes, tilts his head when he asks you
something, and swallows the files you give him. Pick **Claude** or **Codex** at the top (or the
API provider you set up, see below). **New** starts a fresh conversation, and so does switching to
another provider; the one before stays in the list (see [Conversations](#conversations)). **Esc**
closes the chat from anywhere in it; the conversation is still there when you come back.

The chat is Zeca's: with him turned off (**Settings → Flock → Zeca**) there is no chat, no microphone
and the talk shortcut does nothing, and files dropped on the island are not taken. See
[Without Zeca](island.md#without-zeca). On KDE Plasma the desktop keeps **Ctrl+Alt+V** for the app
meanwhile; to use the key elsewhere, clear *Hold to speak to the chat* in System Settings →
Shortcuts; the talk key then stays off until you give it a key there again.

## Where it works

Before the first message, the folder at the top (*in vults ▾*) says where the conversation
will work: the folder of the session in front, unless you pick another one of the sessions on the
wire, or an empty folder of its own. Once the conversation starts it stays there until **New**.

## Model and effort

Beside the folder, *Default model ▾* picks the model and how hard it thinks, for Claude and for
Codex. Claude offers Opus, Sonnet and Haiku (Claude Code's names for the latest of each); Codex
lists the models your own `codex` offers, asked the first time you open the picker with Codex.
The effort goes from **Low** to **Max**, with only the ones the chosen model takes. **Default**
leaves either to the CLI, as your own settings set it.

The choice is remembered per folder and per engine: a new conversation in that folder starts with
it. Like the folder, it is fixed once the conversation starts, so a reply never changes model
midway; **New** starts another. The API chat's model is chosen in **Settings → Chat**.

## The Nest

The arrow at the top of the island's chat opens the **Nest**, Zeca's own window. Its **Chat** tab is
the same conversation, wide, with replies in full Markdown (a link shows its address and is never
followed); a message, **New** or another engine in either one shows in both. Only the island answers
cards: **Answer on the island** brings its chat up. The **Tasks** tab holds your [tasks](#tasks).
The **Memory** and **Roosts** tabs show what Zeca
remembers and the flock by repository (see [Zeca's memory](memory.md)), and the **Agents** tab what
each agent can do with Vults and what this computer has of it (see
[Other agents](other-agents.md#what-each-agent-can-do)). The Nest is gone when you close it or turn
Zeca off.

## Tasks

`/task` in the chat keeps a task in one message, sent to no model: the goal on the first line, then
a line for each criterion that says it is done, starting with `-`:

```
/task Fix the flaky login test
- passes 20 runs in a row
- no sleep added
```

On one line, `done when:` starts the criteria, split by `;`. Its roost is the repository of the
conversation's folder. The Nest's **Tasks** tab lists them, the ones still to do first, with each
criterion to tick, who works on it (you, Zeca or an agent), where it stands (*To do*, *Doing*,
*Blocked*, *Done*, *Dropped*) and what it cost once Vults starts sessions for it. **New task** makes
one there too. With the GitHub connector on, a task in a GitHub repository gets **Open an issue**:
on your click, your `gh` opens an issue with its goal and criteria, and the task links to it. Any
task can link one you already have: `#12` (in its GitHub repository) or the issue's address, then
**Link**. Tasks stay in `vults.sqlite`, on this computer.

**Start with Claude Code** works on a task in a repository. Vults makes the task its own git
worktree, on a branch of its own (`vults/task-12`), under Vults' data folder, and starts Claude
Code there on the task's goal and criteria, told to commit on that branch and never push. It runs in
Claude Code's sandbox (bubblewrap): it can't start without it, every command still asks, and it
writes only in its worktree and the temp folders (a commit on its branch works; your repository's
hooks and config stay out of reach). Commands you excluded from the sandbox in your own Claude Code
settings still run outside it, each asking first. It
reads your own settings, so it shows in the flock like any session and its permissions are island
cards, answered by you, your rules or your policies. Two tasks run side by side without touching
each other's files. When its turn ends, **What it said last** under the task keeps the end of its
reply, and **Go on** continues the same session. **Start fresh** hands the task to a new session
instead, when the old one has grown long: the old session writes a context pack (Objective,
Constraints, Done, In progress, Decisions, Files, Failures, Next), Vults puts in what it knows for
certain (the goal, which criteria are ticked, the files changed on the branch), and the new session
starts from that pack alone. The pack stays under the task as **Handoff notes**; if the old session
writes none, nothing is handed off and it keeps its session. **Stop** asks its turn to stop, waits,
then ends it; only a session Vults started can be stopped, and a running task can't be deleted.
What it cost joins the task. **Remove worktree** removes the folder once
it's stopped, and git refuses while it holds uncommitted work; the branch and its commits stay.
Only Claude Code starts here for now: Codex asks for approvals on a channel no card shows yet.

## Conversations

Every conversation is saved on this computer as it goes, and the list button at the top of the chat
shows them, the latest used first, with the engine and folder of each. Click one to go on with it
where it stopped, even after a restart: it keeps its engine, folder, model and effort, and its CLI
resumes its own session (Claude Code's session, Codex's thread; the API chat replays what was
said). The pencil renames one; the cross deletes it after a second click, with what was said in it.

What is saved is what the chat shows: your messages and the names of the files you sent, Zeca's
replies, errors and stops; never a card's answer (the audit log keeps those) and never an
*Always in this chat* rule, so a conversation opened again asks as a new one would. The newest 200
are kept, in the same local `vults.sqlite` as the activity history; **Settings → Activity →
Clear history** removes them all. A conversation whose folder is gone can't be opened again.

<a id="memory"></a>

## What Zeca knows

Each message carries, before your words, a `<today>` block with the date and time on this computer
and a `<flock>` block with a line for each session the island shows (up to 12, then "and N more"):
its agent, its project, its state and the step it is on (secrets redacted), never the command a
card asks about. It goes to whoever runs the chat, the CLI's provider or the API's, with the
message.

With the memory on, what Zeca remembers and recalls joins as well: see [Zeca's memory](memory.md).

## Writing and reading

- **Enter** sends, **Shift+Enter** starts a new line. You can type the next message while a reply
  comes, and send it when the reply ends.
- While Zeca answers, the send button becomes **Stop**: it ends the turn there, and anything the turn
  was waiting on is a no.
- The reply streams in as it is written. Scroll up to read and it stays where you are, with a
  *↓ new text* button to jump back to the end.

## Speaking instead of typing

Turn it on once in **Settings → Chat → Voice**: download a model (*Base* is 60 MB and quick; *Small*
and *Large v3 Turbo* understand accents and names better). A mic appears next to the send button.

- Click the mic and speak. It stops by itself after about 1.5 s of quiet after your words, or when
  you click again; the words land in the input for you to read and fix, then **Enter** sends them.
  Silence before you start never stops it; if a pause does, click again to go on.
- Or hold **Ctrl+Alt+V** from anywhere: the chat opens and Zeca listens while you hold it (pauses
  never stop it); letting go turns it into text. The key can be changed in System Settings →
  Shortcuts.
- **Esc** while it listens throws the recording away. A recording stops by itself after a minute.
- **Language you speak** follows your system's; pick another, or *Detect it each time*. A fixed
  language is far more reliable on short phrases. *Base* is weak outside English: in Portuguese use
  *Small*, or *Large v3 Turbo* with a GPU.

Only your speech goes to whisper: a small speech detector (Silero VAD, under 1 MB) cuts the silence
around it, so whisper makes no words up there. It downloads by itself while voice is on and it is
missing; until then the mic only stops on a click. With a GPU (Vulkan: AMD, Intel or NVIDIA) a
sentence takes a fraction of a second and a dimmed preview of your words shows as you speak; on the
processor, pick *Base*.

Everything happens on this computer: whisper.cpp transcribes the audio in memory, and the audio is
never saved or sent anywhere. The models and the detector are checked against their known SHA-256
before they are used.

## What it may do

![Zeca asks in the chat before running a command](../assets/island-chat-permission.png)

- It reads files in that folder freely.
- Every command and every edit shows up in the conversation as a card like the island's own: what
  Zeca means to do, the whole command or the file with its **+/−** lines, and **Deny** and **Allow**
  with their keys (**Ctrl+Alt+N** / **Ctrl+Alt+Y** answer it from anywhere). Nothing runs until you
  answer, and a card nobody answers is a no.
- Once the conversation has read content from outside (a web page or search, a command that ran
  `curl` or `git pull` or `gh`, an MCP tool other than those of `vults-zeca`, the server Vults gives the chat's Claude Code), every card
  after it says so in red: a page can carry words written to steer Zeca, so read what the card asks
  with care. Only a call that ran counts (a denied fetch read nothing), building and testing never
  do, and it holds for the rest of that conversation, also when it is opened again from the list. A
  new conversation starts clean.
- **Always in this chat** allows that exact call again (the whole command and input, not the card's
  text) without a card, until the conversation ends: **New**, another engine, or Zeca turned off. A
  card that can't name the call whole has no such button. Each time the rule answers, the chat says
  *Allowed for this conversation* and the audit log keeps it. Rules are never saved.

## Files

Drag a file over the island and it opens on a drop zone, Zeca waiting with his bill open (the **+**
tab shows it too). Drop it: he swallows it, a vulture carries it across while a bar fills, and the
zone asks what it is for. **Ask about it** keeps it on your next message and puts you in the input;
**Cancel** takes it off. You can send it with no text to ask what it is.
Images, PDFs, code and text work best. Files are copied into the app's inbox (up to 20 MB each) and
deleted after a week; a folder, or a file over 20 MB, is refused with a line saying why.

## Your subscription, your login

The chat runs the `claude` or `codex` command you already logged into, so it uses your own
subscription and Vults never reads your credentials. Both stream the reply as it is written:
Claude through `claude -p`, Codex through one long-lived `codex app-server`.

### Your setup

The chat reads your own Claude Code and Codex setup, as in your terminal: your `CLAUDE.md`, skills,
MCP servers, hooks and allow rules (a person icon at the top says so). A project's own settings and
`.mcp.json` servers join only in a folder you already trusted in Claude Code; its `CLAUDE.md` is
read either way. Your allow rules apply in every folder. Cards still ask whatever your rules don't
allow, whatever `defaultMode` your settings set, and your hooks never make the chat show up as an
agent session.

Turn it off in **Settings → Chat → Your Claude Code and Codex setup**, and Claude reads none of it;
Codex then only leaves out your hooks. The switch applies to the next conversation. Worth knowing:

- `claude -p` skips the trust prompt, so a repository you just cloned can't bring rules or hooks of
  its own.
- An allow rule such as `Bash(npm test:*)` runs a cloned project's test script with no card.
- With the switch off, Claude gets no MCP servers but Vults' own, and no saved rule skips a card.
- Codex's `config.toml` still brings its MCP servers, and its `AGENTS.md` is still read.
- A conversation already running keeps what it read.

See [Safety](../safety.md) for the rest.

## Without a CLI: an API key, or a local model

The chat can also use a provider's API with your own key, or a model running on your machine. In
**Settings → Chat**, pick the **Provider**, paste its key, and choose a **Model**; the provider's
name then appears at the top of the chat, next to Claude and Codex.

| Provider | Key | Notes |
|---|---|---|
| Anthropic | `sk-ant-…` | Claude Opus 5.5 until you pick another model |
| OpenAI | `sk-…` | |
| Google Gemini | `AIza…` (Google AI Studio) | |
| OpenRouter | `sk-or-…` | One key for hundreds of models |
| Groq, DeepSeek, Mistral, xAI | their own | |
| Ollama | none | Must be running on `127.0.0.1:11434` |
| LM Studio | none | Its server must be on, at `127.0.0.1:1234` |

- The models are listed live from the provider. Each provider keeps the model you chose for it.
- A key is kept in your [system keyring](../reference/settings.md#where-things-live-linux), is
  never shown again, and is only sent to its own provider; **Remove** deletes it. Usage is billed to
  your account with that provider.
- This chat can't run commands, edit files or look around your project. It reads what you type and
  the files you drop (images, text up to 512 KB, and PDFs with Anthropic), and calls tools when a
  question needs them: the flock tools (your sessions, the cards waiting, your plans' usage, your
  GitHub card), `recall` and `memory_propose` with the memory on, and the web tools with the web on.
  What a tool returns goes to the provider (commands with their secrets redacted); it never answers
  a card.
- Reasoning models that think out loud only show their answer: the `<think>` part is hidden and
  never sent back.
- A conversation keeps the provider and model it started with.
- With Anthropic, a request Claude declines on safety grounds is retried on another Claude model
  within the same call; if that one declines too, the bubble says so.

### The web

With **Settings → Chat → Web for Zeca** on, the API chat can search the web and read pages. Searches
go to Brave Search or Tavily with your own key (kept in the keyring), or to a SearXNG you run. Off,
the model is never offered either.

A page is untrusted, so the chat's model never sees it: a separate call to your API provider, with no
tools and told the page is data, reads it for the question, and only that answer comes back. A
search's titles and snippets do reach the model, marked untrusted. Pages are read only from public
addresses. Each search and page is a request on your behalf with a daily cap
([Safety](../safety.md#every-request-vults-makes)), and the conversation then counts as having read
from outside.
