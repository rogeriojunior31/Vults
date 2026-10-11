# Chatting with Zeca

Open the **Chat** tab, choose **Chat…** from the tray icon, or drop a file on the island. Zeca sits on
the left of the conversation: he thinks while the reply comes, tilts his head when he asks you
something, and swallows the files you give him. Pick **Claude** or **Codex** at the top (or the
API provider you set up, see below). **New** starts a fresh conversation, and so does switching to
another provider; the one before stays in the list (see [Conversations](#conversations)). **Esc**
closes the chat from anywhere in it; the conversation is still there when you come back.

The chat is Zeca's: with him turned off (**Settings → Flock → Zeca**) there is no chat, no microphone
and the talk shortcut does nothing, and files dropped on the island are not taken. See
[Without Zeca](island.md#without-zeca).

The desktop still holds **Ctrl+Alt+V** for the app while Zeca is off, and the app ignores the
press. On KDE Plasma the desktop's shortcut service cannot set one key aside and keep it: the app
would have to drop the key, and then turning Zeca back on would ask you to bind it again. If you
want the key for something else while Zeca is off, clear *Hold to speak to the chat* in System
Settings → Shortcuts; the talk shortcut stays off until you give it a key there again.

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

The arrow at the top of the island's chat opens the **Nest**, Zeca's own window: the same
conversation, wide, with replies in full Markdown (headings, tables, quotes, numbered lists; a link
shows its address and is never followed). A message sent in either one shows in both, and so do
**New**, another engine, or a conversation opened from the list. A card shows in the Nest too, but
only the island answers cards: **Answer on the island** brings the island's chat up with it. The
Nest is built when you open it and gone when you close it, and turning Zeca off closes it. Short
answers still read best in the island's bubble.

The Nest's **Roosts** tab groups the flock by repository: one card for each repository a session
works in, with its folders and their branches (the worktrees of one repository share a card), its
sessions, your pull requests in it and its default branch's checks (from the GitHub card, when that
connector is on), and what Zeca remembers of the project. It reads the repositories on this
computer and asks nothing of the network. A conversation that starts in a repository brings that
memory along with the rest of Zeca's.

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

## What Zeca knows

Each message carries, before your words, the date and time on this computer and a line for each
session the island shows: its agent, its project, its state and the step it is on (secrets
redacted), never the command a card asks about. It goes to whoever runs the chat, the CLI's
provider or the API's, with the message.

Zeca's memory joins a conversation when it starts: who you are, what he learned and how to speak to
you, after his instructions. It stays as it was for the whole conversation, so a reply never sees
it change midway, and the prompt is the same on every turn (a provider's cache keeps working); a
memory added meanwhile joins the next conversation. Until his memory has something in it, only a
line saying what the date and session blocks are joins. A conversation begun before this version
gets neither.

## Memory

Zeca's memory is off until you turn it on in **Settings → Chat → Memory**. On, the app keeps a
journal on this computer: each agent turn's time, steps and changes, each card's answer with its
command or path, and the lines of Zeca's chat (your words and his), with secrets redacted before anything is written, for 90 days. Nothing of it
leaves the computer. Turning it off stops the writing and keeps what was written; **Settings →
Activity → Clear history** removes it.

With the memory on, **A summary of each day** can sum up each past day of the journal for Zeca to
recall later: once a day, the day before's journal goes to Claude Code's smallest model (Haiku), on
your subscription, with no tools and none of your settings, and the summary (at most 2,000 bytes)
is kept beside the journal. It is a request on your behalf: off until you turn it on, at most 8 a
day, each in the local trace. Only the days from when you turn it on go, never the journal kept
before. A day that fails is tried again an hour later, three times at most.

With the memory on, Zeca recalls: before each message, the app searches the journal and the days'
summaries for what your words are about, and up to three passages join the message (as the
`<memories>` block, context and never instructions); the API chat can also ask for more with its
`recall` tool. The search runs on this computer. When an [Ollama](https://ollama.com) runs here
with `nomic-embed-text` (`ollama pull nomic-embed-text`), it also finds summaries by meaning, not
only by their words; it is reached on `127.0.0.1` only.

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

- Click the mic and speak: a waveform shows what it hears. When you stop talking (about 1.5 s of
  quiet after your words) it stops by itself, or click again (the red stop) any time. Zeca thinks
  while it turns into text; the words land in the input for you to read and fix, then **Enter**
  sends them as usual. Silence before you start never stops it, so take your time, and a breath
  between phrases does not either; if a longer pause stops it, click the mic again to go on (the
  new words are added).
- Or hold **Ctrl+Alt+V** from anywhere: the chat opens, Zeca cocks his head and listens while you
  hold it, and letting go turns it into text (pauses never stop it while you hold the key). The
  key can be changed in System Settings → Shortcuts.
- **Esc** while it listens throws the recording away. A recording stops by itself after a minute.
- **Language you speak** follows your system's (Portuguese on a `pt_BR` desktop); pick another, or
  *Detect it each time*. A fixed language is far more reliable on short phrases. *Base* is weak
  outside English: in Portuguese use *Small*, or *Large v3 Turbo* with a GPU.
- Only your speech goes to whisper: the silence and noise around it are cut, so whisper does not
  make words up in them (*Thank you.*, *Obrigado.*), and a recording with no speech gives no text.
  A small speech detector (Silero VAD, under 1 MB, MIT) tells speech from silence; it downloads
  with a model, and by itself in the background whenever voice is on without it (a model from an
  older version, or a try that failed offline). Until it is there the mic only stops on a click
  and the cut goes by loudness.

On Linux it runs on the graphics card through Vulkan when there is one (AMD, Intel or NVIDIA):
a sentence takes a fraction of a second even with *Large v3 Turbo*. Without a usable GPU it runs on
the processor, where *Base* is the one to pick.

With a GPU you also see your words as you speak: about every 0.8 s the recording so far is read
again, and its text shows dimmed beside a shorter waveform, its newest words in view. It is a
preview only: when the mic stops, the final transcription replaces it and that is what lands in the
input. On the processor there is no preview (one pass takes over half a second even with *Base*, too
much to repeat while you talk), so the waveform and *Listening…* show as before.

Everything happens on this computer: whisper.cpp transcribes the audio in memory, and the audio is
never saved or sent anywhere. The models come from the whisper.cpp repositories on Hugging Face
(`ggerganov/whisper.cpp`, and `ggml-org/whisper-vad` for the speech detector) and are checked
against their known SHA-256 before they are used.

## What it may do

![Zeca asks in the chat before running a command](../assets/island-chat-permission.png)

- It reads files in that folder freely.
- Every command and every edit shows up in the conversation as a card like the island's own: what
  Zeca means to do, the whole command or the file with its **+/−** lines, and **Deny** and **Allow**
  with their keys (**Ctrl+Alt+N** / **Ctrl+Alt+Y** answer it from anywhere). Nothing runs until you
  answer, and a card nobody answers is a no.
- Once the conversation has read content from outside (a web page or search, a command that ran
  `curl` or `git pull` or `gh`, an MCP tool), every card after it says so in red: a page can carry
  words written to steer Zeca, so read what the card asks with care. Only a call that ran counts (a
  denied fetch read nothing), building and testing never do, and it holds for the rest of that
  conversation, also when it is opened again from the list. A new conversation starts clean.
- **Always in this chat** allows that exact command again, without a card, until the conversation
  ends: the whole command and the call's whole input, not the card's text, so a different command
  that starts the same way still asks. A card that can't name the call whole (an edit whose
  changes Codex didn't show) has no such button. Each time the rule answers, the chat says
  *Allowed for this conversation* with what ran, and the audit log keeps it as your rule's answer.
  **New**, another engine, or Zeca turned off ends the conversation and its rules. Rules are never
  saved: a conversation opened again from the list starts with none.

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

The chat reads your own Claude Code and Codex setup, as `claude` and `codex` do in your terminal:
your `CLAUDE.md`, skills, MCP servers, hooks and allow rules. A small person icon at the top
of the chat says so (its tooltip says what it reads). A project's own settings (its `.claude/settings.json` and
`settings.local.json`) join only in a folder you already trusted in Claude Code: `claude -p` skips
the trust prompt, so a repository you just cloned can't bring rules or hooks of its own. The
project's `CLAUDE.md` is read either way.

Cards still ask: a command or an edit your own allow rules don't cover waits for your Allow, even
if your settings set a `defaultMode` that would skip the prompt (`acceptEdits`,
`bypassPermissions`). Your hooks run, Vults' among them, but they only mark the chat's events as
Zeca's: the chat never shows up on the island as an agent session, and never asks twice.

Your own allow rules apply in every folder, as in your terminal: a rule like `Bash(npm test:*)` runs
a cloned project's test script with no card. A project's `.mcp.json` servers start only in a folder
you trust.

Turn it off in **Settings → Chat → Your Claude Code and Codex setup**, and Claude reads none of it:
no hooks, no settings, no MCP servers but its own, and no saved rule lets a command skip the card.
Codex then only leaves out your hooks; it always reads its own `config.toml`, with its MCP servers
and `AGENTS.md`. The switch applies to the next conversation; one that started keeps what it read.

## Without a CLI: an API key, or a local model

If you don't use Claude Code or Codex, the chat can use a provider's API with your own key, or a model
running on your machine. In **Settings → Chat**, pick the **Provider**, paste its key, and choose a
**Model**; the provider's name then appears at the top of the chat panel, next to Claude and Codex.

| Provider | Key | Notes |
|---|---|---|
| Anthropic | `sk-ant-…` | Claude Opus 5.5 until you pick another model |
| OpenAI | `sk-…` | |
| Google Gemini | `AIza…` (Google AI Studio) | |
| OpenRouter | `sk-or-…` | One key for hundreds of models |
| Groq, DeepSeek, Mistral, xAI | their own | |
| Ollama | none | Must be running on `127.0.0.1:11434` |
| LM Studio | none | Its server must be on, at `127.0.0.1:1234` |

- The models are listed live from the provider, so new ones show up without an update. Each provider
  keeps the model you chose for it.
- A key is kept in your system keyring, never in a file, the app never shows it again, and it is only
  ever sent to its own provider. **Remove** deletes it from the keyring, and the next message stops
  working at once. A local model needs no key, and nothing leaves your machine.
- Usage is billed to your account with that provider.
- This chat can't run commands, edit files or look around your project. It reads what you type and
  the files you drop (images as they are, text files inline up to 512 KB, and PDFs too with
  Anthropic), and what the island shows, through tools it calls when a question needs them: your
  sessions and what each is doing, the cards waiting, your plans' usage and your GitHub card. Ask
  "what is Codex doing?" and it looks. What a tool returns goes to the provider with the rest of
  the conversation (commands with their secrets redacted); a card waiting is never answered by it.
- Reasoning models that think out loud (DeepSeek-R1, Qwen3 and others on Ollama or LM Studio) only
  show their answer: the `<think>` part is hidden, and isn't sent back in the next message either.
- A conversation keeps the provider and model it started with; choosing another provider in
  Settings applies to the next one.
- With Anthropic, if Claude declines a request on safety grounds, the API retries it on another Claude
  model within the same call (Anthropic's server-side fallback). If that model declines too, the
  bubble says so.

### The web

With **Settings → Chat → Web for Zeca** on, the API chat can search the web and read pages when a
question needs it. Searches go to the engine you choose: Brave Search or Tavily with your own key
(kept in the system keyring), or a SearXNG you run yourself. Off, the model is never offered either.

A page is untrusted: it can carry words written to steer Zeca. So the chat's model never sees it,
not even its title. A separate call to your API provider, with no tools and told the page is data,
reads it for the question, and only that answer comes back. A search's results do reach the model,
as titles and short snippets marked untrusted. Each search and each page is a request on your behalf:
at most 200 a day, each in the local trace. Pages are read only from public addresses, never from
this computer or its network. The conversation then counts as having read from outside, and any
card after it says so.
