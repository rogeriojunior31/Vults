# Safety

What Vults promises:

- **It never blocks your agent.** If the app is closed, slow or broken, the hook exits within
  milliseconds and prints nothing, and your agent asks in its terminal as usual.
- **Only a click approves a tool call.** No timer or default ever answers a permission request; the
  only exception is a rule you created yourself with **Always**, for that exact command or file.
  A question from Claude Code is answered only with the choice or the words you gave on its card.
  A phone you paired counts as a click only when its answer is signed by its key for that very
  card, in time, never before seen; approving needs your biometrics on the phone, denying does not
  (see [The phone](guide/phone.md)).
- **Only your user account can talk to it.** The socket lives in your private runtime folder with mode
  `0600`, and both ends check the other process runs as you (on Windows, by SID).
- **It never edits an agent's config behind your back.** Every change gets a dated backup and a diff you
  approve; hooks from other tools are kept, and a Claude Code status line of your own is saved beside the hook, keeps running, and comes back as it was when you remove ours; Codex's `trusted_hash` is never written (you trust our hooks yourself, in Codex's `/hooks`).
- **Plan usage comes from the CLIs only.** Claude Code's status line input is cut down in the hook
  to its `rate_limits` and the session's cost before anything leaves it; Codex is asked with a read-only call, only while
  **Settings → General → Codex usage** is on.
- **The chat uses the CLIs you logged into** (`claude`, `codex`), and Vults never reads their
  credentials; every command and edit your own rules don't allow waits for your Allow (see
  [Your setup](guide/chat.md#your-setup)).
- **API keys stay in your OS keyring.** If you give the chat a provider's API key, it is written
  only to the keyring (Secret Service on Linux), never to a file or a log, never sent back to a
  window, and only ever sent to that provider. Local models (Ollama, LM Studio) are reached on
  `127.0.0.1` only. The API chat can't run commands or edit files. Its tools read what the island
  shows (sessions, cards waiting, usage, your GitHub card); with the memory on, `recall` reads the
  journal and `memory_propose` writes only a proposal; with the web on, it gets the web tools. What
  they return goes to that provider with the conversation, commands with their secrets redacted.
- **Each chat message says what the island shows.** Through a CLI or an API, a message to Zeca
  carries the date and time and a line for each session (agent, project, state, its step with
  secrets redacted), never a card's command. With the memory on, what Zeca remembers joins when a
  conversation starts, and up to 3 recalled passages go with each message.
- **The microphone only listens while you hold a recording.** Voice is off until you download a
  model; then the mic records into memory only while its button is red (a minute at most), and
  whisper.cpp transcribes it on this computer. The audio is never saved or sent anywhere.
- **Now playing is off until you turn it on.** Then the app reads your media players over the
  session bus (MPRIS on Linux) to show the song; it is never stored or sent anywhere.
- **The edits it shows stay in memory.** For the island's diff, the hook forwards a finished edit's
  changed lines (400 at most); the app keeps them for the session's last few steps only, and never
  writes them to a file or a log.
- **What the app asks on its own goes through one gate.** A request made without a click at that
  moment (a connector's check, the Codex usage read, a paired phone's cards and answers, the web
  tools, the journal summary, memory suggestions and Zeca's sleep) asks the app's network gate
  first: only a feature you switched on, each with its own daily cap (see the table), each request
  kept as a line in `vults.sqlite` (when, which feature, whether it answered; 30 days). A connector
  over its cap waits and says so in Settings → Connectors.
- **Connectors keep only their last answer.** GitHub's last check (titles, links, check and review
  states of your open pull requests and recent repositories) is saved in the app's data folder,
  `connectors/github.json`, so a restart does not replay old news. Your contribution calendar, for
  Settings → Activity, is asked only when its tab opens and kept an hour in memory. Nothing else
  is stored, and it is never sent anywhere.
- **The activity history is counts, on this computer.** For Settings → Activity, each finished
  turn of an agent is kept in the data folder (`vults.sqlite`: each turn for 12 weeks, each day's
  totals for a year): when it ended, how long it ran, the agent, the project's folder name, and
  counts (steps, commands, files and lines changed, your answers). Never a prompt, a reply, a
  command, a file name or a path. It is never sent anywhere; turn it off or clear it in Settings →
  Activity.
- **Zeca's turns are traced, on this computer.** Each chat turn and each tool it asked to run is a
  span in `vults.sqlite`, kept 30 days: the engine and model, when it started and ended, the tokens
  in and out, the prompt cache, the cost when the engine reports it, and a tool's name and target
  with secrets redacted. Never a prompt or a reply. Nothing of it leaves the computer; **Clear
  history** removes it too.
- **Zeca's conversations are kept, on this computer.** So one goes on after a restart, the chat
  saves what it shows (your messages, the names of the files you sent, Zeca's replies) in the same
  `vults.sqlite`, the newest 200, and what resumes it: the CLI's own session id, or the API chat's
  history, where each file sent is kept as its name, never its contents. Never a card's answer nor an *Always in this chat* rule. Delete one from the chat's list;
  **Clear history** removes them all. Nothing of it leaves the computer.
- **Zeca's memory is off until you turn it on, and is kept on this computer.** Off, nothing of it
  reaches a model or a window (the Memory beside the Nest's Chat says only that it is off); a conversation already
  running keeps the frozen part it began with, and one reopened from the list starts without his
  facts. On, a journal in `vults.sqlite` keeps each agent turn's numbers, each card's
  answer with its command or path, and the chat's lines for 90 days, secrets redacted by the
  database layer before anything is written; his facts are a git repository in the data folder.
  What reaches the chat's provider is his facts when a conversation starts, the recalled passages
  with each message, and what his tools return (`recall`, `journal_day`, `roost_memory`) when the
  model calls them. The day summary, the suggestions and the sleep are switches of their own, and
  send their text to Claude Code's smallest model with no tools, so it can only become words. His MCP server, which the
  chat's Claude Code gets through `--mcp-config` (never a change to your Claude config), is this
  app run again: it reads the journal, the memory and a redacted snapshot of the island's sessions
  (removed when the app quits), never upgrades `vults.sqlite`, sends nothing anywhere, and Claude
  Code asks before each call your rules don't allow. **Clear history** removes the journal and what derives from
  it, not his facts (see [Zeca's memory](guide/memory.md)).
- **Only you write Zeca's memory, and a project's `CLAUDE.md`.** `/remember` and `/forget` are your
  words, saved as yours, and need the memory on; what Zeca, a model or his MCP server would add waits in the Nest until you
  accept it, and each change is a commit you can undo. A proposal's text passes the same scan as
  the memory (no hidden characters, no words that address a model, no key shapes). To let Claude
  Code read the `AGENTS.md` Codex reads, the Nest shows the exact change first; **Apply** writes it
  over the very file you saw, after a dated backup outside the repository.
- **Every answer is on record, on this computer.** An audit log in the same `vults.sqlite` keeps,
  for 90 days, each answer to a card, the flock's and Zeca's chat's: who gave it (you, one of your
  *Always* rules or the chat's *Always in this chat*, one of your
  [policies](guide/approvals.md#policies) with its consent, or the app when a card's time ran out or
  *Paused* sent it to the terminal), what it did (allow, deny, always allow, allow in this
  conversation, answer, send to the terminal), the agent, the project's folder
  name, the tool and the exact command or path it was about, with tokens and `NAME=secret` values
  redacted first (never what follows them). An answer given in the agent's own terminal is not
  Vults', and is not in it. Each agent config the installer writes is kept too. A line is never changed, and none leaves before 90 days, except through **Clear history** in
  Settings → Activity, which removes it with the history and leaves one line: that you cleared it,
  and when. It is always on, and never sent anywhere.
- **What you agree to is kept for good, on this computer.** Each policy you accept is a row in
  `vults.sqlite`'s consents: when, what it
  is about, the words you accepted and a SHA-256 of the diff you saw. A row is never changed or
  deleted, nothing prunes it, and **Clear history** does not touch it: a policy must never lose the
  proof that you said yes. Revoking is the only way to take one back, and it turns the policy off
  at once; the row stays, marked revoked. The row's words are the policy itself: nothing else holds
  it, and one whose diff does not end at those words never answers. Never sent anywhere.
- **Secrets stay in your OS keyring**, and there is no telemetry.

## Every request Vults makes

Nothing else reaches the network. CI checks it: `scripts/check-network.sh` fails on a crate that
depends on an HTTP client, or a file that names one or starts a network command, outside
`scripts/network-allowlist.txt`, which names each one and why; and on a webview allowed to connect
anywhere but the app.

| Request | Who starts it | What turns it on | Where |
|---|---|---|---|
| A chat turn through an API (Anthropic, OpenAI, Gemini, OpenRouter, Groq, DeepSeek, Mistral, xAI; Ollama and LM Studio on `127.0.0.1`) | You, with each message | The provider and key you chose; sent only to that provider | `crates/chat/src/api.rs`, `openai.rs`, `providers.rs` |
| The list of a provider's models | You, opening **Settings → Chat** with a key saved (or a local Ollama or LM Studio), or with **Try again** | The provider and key you chose | `crates/chat/src/providers.rs` |
| A web search (Zeca's web, **Settings → Chat**, off until you turn it on) | The API chat's model, during a turn you sent; through the network gate (200 a day, with page reads) | Brave Search or Tavily with your key, or your own SearXNG | `crates/chat/src/web.rs`, `app/src/web_tools.rs` |
| A web page read (the same switch) | The API chat's model, during a turn you sent; through the network gate (the same 200) | The page's site, public addresses only; its text then goes to your API provider in a call of its own, with no tools | `crates/chat/src/web.rs`, `app/src/web_tools.rs` |
| A paired phone's cards and answers (**Settings → Phone**, off until you turn it on) | The app: each card it sends, and a read of each phone's box every 25 s; through the network gate (10,000 a day) | The relay you chose, which sees only sealed messages; in **Straight**, this computer listens on the address you gave instead | `crates/relay/src/client.rs`, `app/src/phone.rs` |
| Codex's list of models | You, opening the chat's model picker with Codex (once per run) | The `codex` you logged into, through `codex app-server`'s `model/list` | `crates/chat/src/codex_server.rs` |
| A chat turn through `claude` or `codex` | You, with each message | The engine you chose; the CLI talks to its own service on your login | `crates/chat/src/claude.rs`, `codex.rs`, `codex_server.rs` |
| A task's session (`claude` or `codex app-server` in the task's worktree) | You, clicking a task's **Start with Claude Code** or **Start with Codex**; its turn runs until it ends or you **Stop** it | The CLI talks to its own service on your login, in its sandbox, each command asking first | `app/src/runs.rs`, `crates/chat/src/claude.rs`, `codex_server.rs` |
| The Codex usage read (`account/rateLimits/read`, through `codex app-server`) | The app, every 5 minutes and half a minute after a window resets, never while the screen is locked | **Settings → General → Codex usage**, through the network gate (400 a day) | `app/src/usage.rs` |
| A past day of Zeca's journal summed up (**Settings → Chat → Memory → A summary of each day**, off until you turn it on) | The app, in an hourly pass: each past day without a summary since you turned it on, up to 3 a pass; through the network gate (8 a day) | The `claude` you logged into, its smallest model, with no tools and none of your settings | `crates/chat/src/summary.rs`, `app/src/journal.rs` |
| An embedding of a journal summary or of your message, for recall (**Recall by meaning**, off until you turn it on) | The app, in the hourly journal pass for each summary without one, and before each chat message | An Ollama on `127.0.0.1` with `nomic-embed-text`, never through a proxy; nothing leaves the computer | `crates/chat/src/embed.rs`, `app/src/recall.rs` |
| Memory suggestions from a conversation you left (**Suggest what to remember**, off until you turn it on) | The app, when you start a new conversation or open another; through the network gate (20 a day), only with at least 2 of your messages, never for one that read outside content | The `claude` you logged into, its smallest model, with no tools and none of your settings; what it proposes waits for your yes | `crates/chat/src/summary.rs`, `app/src/memory.rs` |
| Zeca's sleep (**Let Zeca sleep on it**, off until you turn it on): what he remembers about you and across projects, and the last 7 days' summaries | The app, once a day from 2 a.m. while nothing works; through the network gate (4 a day) | The `claude` you logged into, its smallest model, with no tools and none of your settings; its merges wait for your yes | `app/src/sleep.rs`, `crates/chat/src/summary.rs` |
| A connector's check (GitHub, through `gh`) | The app, on its own schedule | Each connector's switch in **Settings → Connectors**, through the network gate (3,000 a day) | `crates/connectors/src/github.rs` |
| Your GitHub contribution calendar (through `gh`) | You, opening **Settings → Activity**'s GitHub tab | The GitHub connector on | `crates/connectors/src/github.rs` |
| An issue from a task (through `gh`) | You, clicking a task's **Open an issue** in the Nest | The GitHub connector on | `crates/connectors/src/github.rs` |
| A voice model's download | You, with a click in **Settings → Chat → Voice** | Nothing until you click; from Hugging Face (`ggerganov/whisper.cpp`), checked against its SHA-256 | `crates/voice/src/models.rs` |
| The speech detector's download (Silero VAD, under 1 MB) | The app, by itself, outside the gate, while voice is on and it is missing | A voice model you chose; from Hugging Face (`ggml-org/whisper-vad`), checked against its SHA-256 | `crates/voice/src/models.rs`, `app/src/voice.rs` |
| Claude Code's plan usage | Claude Code, which hands it to our status line | No request of ours | the hook |
| The update check | Not built yet (ADR 0016) | It will be off until you turn it on, through the gate | — |
