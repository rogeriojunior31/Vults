# Safety

What Vults promises:

- **It never blocks your agent.** If the app is closed, slow or broken, the hook exits within
  milliseconds and prints nothing, and your agent asks in its terminal as usual.
- **Only you answer a permission.** No timer or default ever answers one. Besides your click, only
  these do: an exact rule you made with **Always**, for that command or file; a
  [policy](guide/approvals.md#policies) you wrote, saw as a diff and accepted with a click (a deny
  beats an allow); a phone
  you paired, when its answer is signed by its key for that very card, in time, never seen before
  (approving needs your biometrics on the phone, denying does not; see [The phone](guide/phone.md));
  and, in Zeca's chat only, the access you chose for its cards
  ([Who answers Zeca's cards](guide/chat.md#who-answers-zecas-cards)); under its full access, a
  policy's deny or ask still holds, and content from outside brings cards back. A destructive call (`rm -rf`,
  a forced push) always asks: its card offers no **Always**, and no rule, policy or full access ever
  answers it. A question from Claude Code is answered only with the choice or the words you gave on
  its card.
- **Only your user account can talk to it.** The socket lives in your private runtime folder with mode
  `0600`, and both ends check the other process runs as you (on Windows, by SID).
- **Looking at a session's folder only reads it.** To see the files a session's commands wrote,
  at each turn's start and end the app lists its folder's repository with `git ls-files` and
  compares each file's size and modification time itself. Git reads no file's content and writes
  nothing; fsmonitor and hooks are turned off, so no filter, hook or monitor in the repository's
  config runs.
- **It never edits an agent's config behind your back.** Every change gets a dated backup and a diff
  you approve; hooks from other tools are kept, a Claude Code status line of your own keeps running
  ([how](reference/settings.md#agent-configs-vults-edits)), and Codex's `trusted_hash` is never
  written (you trust our hooks yourself, in Codex's `/hooks`).
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
- **What the app asks on its own goes through one gate.** A request made without a click at that
  moment (a connector's check, the Codex usage read, a paired phone's cards and answers, the web
  tools, the journal summary, memory suggestions and Zeca's sleep) asks the app's network gate
  first: only a feature you switched on, each with its own daily cap (see the table), each request
  kept as a line in `vults.sqlite` (when, which feature, whether it answered; 30 days). A connector
  over its cap waits and says so in Settings → Connectors.

What it keeps stays on this computer, is never sent anywhere, and is listed in
[Settings and files](reference/settings.md#where-things-live-linux);
[Clear history](guide/activity.md#the-history) removes most of it:

- **The edits it shows stay in memory**: a finished edit's changed lines (400 at most), for the
  session's last 32 edits, and the paths of the files it changed with their counts, never written
  to a file or a log.
- **Connectors keep only their last answer**, so a restart does not replay old news
  ([Connectors](guide/connectors.md#the-github-card)).
- **The activity history is counts**: never a prompt, a reply, a command, a file name or a path
  ([Activity](guide/activity.md#what-a-turn-is)).
- **Zeca's turns are traced** for 30 days: engine, model, tokens, cost and each tool's redacted
  target, never a prompt or a reply ([The Nest](guide/nest.md#activity)).
- **Zeca's conversations are kept**, the newest 200, never a card's answer nor an *Always in this
  chat* rule ([Conversations](guide/chat.md#conversations)).
- **Zeca's memory is off until you turn it on**, and only you write it: what Zeca, a model or his
  MCP server would add waits for your yes, and a project's `CLAUDE.md` changes only over the file
  you saw, after a dated backup ([Zeca's memory](guide/memory.md)).
- **Every answer is on record.** An audit log keeps, for 90 days, each answer to a card, the flock's
  and Zeca's chat's: who gave it (you, your paired phone, an *Always* rule, the chat's *Always in this chat*, a
  [policy](guide/approvals.md#policies) with its consent, the chat's
  [full access](guide/chat.md#who-answers-zecas-cards), or the app when a card's time ran out or
  *Paused* sent it to the terminal), what it did (allow, deny, always allow, allow in this
  conversation, answer, send to the terminal, or time ran out), the agent, the project's folder name, the tool and
  the exact command or path, with tokens and `NAME=secret` values redacted first. An answer given in
  the agent's own terminal is not in it. Each agent config the installer writes is kept too. A line
  is never changed and none leaves before 90 days, except through **Clear history**, which leaves
  one line: that you cleared it, and when. It is always on.
- **What you agree to is kept for good.** Each policy you accept is a row in `vults.sqlite`'s
  consents: when, what it is about, the words you accepted and a SHA-256 of the diff you saw. A row
  is never changed or deleted, and **Clear history** does not touch it: a policy must never lose the
  proof that you said yes. Revoking turns the policy off at once; the row stays, marked revoked.
  The row's words are the policy itself, and one whose diff does not end at those words never
  answers.
- **Secrets stay in your OS keyring**, and there is no telemetry.

## Every request Vults makes

Nothing else reaches the network. CI checks it: `scripts/check-network.sh` fails on a crate that
depends on an HTTP client, or a file that names one or starts a network command, outside
`scripts/network-allowlist.txt`, which names each one and why; and on a webview allowed to connect
anywhere but the app.

| Request | Who starts it | What turns it on | Where |
|---|---|---|---|
| A chat turn through an API (Anthropic, OpenAI, Gemini, OpenRouter, Groq, DeepSeek, Mistral, xAI; Ollama and LM Studio on `127.0.0.1`) | You, with each message | The provider and key you chose; sent only to that provider | `crates/chat/src/api.rs`, `openai.rs`, `providers.rs` |
| The list of a provider's models | You, opening **Settings → Chat & voice** with a key saved (or a local Ollama or LM Studio), or with **Try again** | The provider and key you chose | `crates/chat/src/providers.rs` |
| A web search (Zeca's web, **Settings → Chat & voice**, off until you turn it on) | The API chat's model, during a turn you sent; through the network gate (200 a day, with page reads) | Brave Search or Tavily with your key, or your own SearXNG | `crates/chat/src/web.rs`, `app/src/web_tools.rs` |
| A web page read (the same switch) | The API chat's model, during a turn you sent; through the network gate (the same 200) | The page's site, public addresses only; its text then goes to your API provider in a call of its own, with no tools | `crates/chat/src/web.rs`, `app/src/web_tools.rs` |
| A paired phone's cards and answers (**Settings → Phone**, off until you turn it on) | The app: each card it sends, and a read of each phone's box every 25 s; through the network gate (10,000 a day) | The relay you chose, which sees only sealed messages; in **Straight**, this computer listens on the address you gave instead | `crates/relay/src/client.rs`, `app/src/phone.rs` |
| Codex's list of models | You, opening the chat's model picker with Codex (once per run) | The `codex` you logged into, through `codex app-server`'s `model/list` | `crates/chat/src/codex_server.rs` |
| A chat turn through `claude` or `codex` | You, with each message | The engine you chose; the CLI talks to its own service on your login | `crates/chat/src/claude.rs`, `codex.rs`, `codex_server.rs` |
| A continued session (`claude` or `codex app-server` in its existing folder) | You, typing `/continue <words>` in Chat; `/continue stop` stops only that execution | The CLI talks to its own service on your login, in its sandbox, each command asking first | `app/src/continuation.rs`, `crates/chat/src/claude.rs`, `codex_server.rs` |
| The Codex usage read (`account/rateLimits/read`, through `codex app-server`) | The app, every 5 minutes and half a minute after a window resets, never while the screen is locked | **Settings → General → Codex usage**, through the network gate (400 a day) | `app/src/usage.rs` |
| A past day of Zeca's journal summed up (**Settings → Chat & voice → Memory → A summary of each day**, off until you turn it on) | The app, in an hourly pass: each past day without a summary since you turned it on, up to 3 a pass; through the network gate (8 a day) | The `claude` you logged into, its smallest model, with no tools and none of your settings | `crates/chat/src/summary.rs`, `app/src/journal.rs` |
| An embedding of a journal summary or of your message, for recall (**Recall by meaning**, off until you turn it on) | The app, in the hourly journal pass for each summary without one, and before each chat message | An Ollama on `127.0.0.1` with `nomic-embed-text`, never through a proxy; nothing leaves the computer | `crates/chat/src/embed.rs`, `app/src/recall.rs` |
| Memory suggestions from a conversation you left (**Suggest what to remember**, off until you turn it on) | The app, when you start a new conversation or open another; through the network gate (20 a day), only with at least 2 of your messages, never for one that read outside content | The `claude` you logged into, its smallest model, with no tools and none of your settings; what it proposes waits for your yes | `crates/chat/src/summary.rs`, `app/src/memory.rs` |
| Zeca's sleep (**Let Zeca sleep on it**, off until you turn it on): what he remembers about you and across projects, and the last 7 days' summaries | The app, once a day from 2 a.m. while nothing works; through the network gate (4 a day) | The `claude` you logged into, its smallest model, with no tools and none of your settings; its merges wait for your yes | `app/src/sleep.rs`, `crates/chat/src/summary.rs` |
| A connector's check (GitHub, through `gh`) | The app, on its own schedule | Each connector's switch in **Settings → Connectors**, through the network gate (3,000 a day) | `crates/connectors/src/github.rs` |
| Your GitHub contribution calendar (through `gh`) | You, opening **Settings → Activity**'s GitHub tab | The GitHub connector on | `crates/connectors/src/github.rs` |
| A squash merge of your pull request (through `gh`), with a line in the local trace | You, clicking **Squash and merge** on its row in the island's Connections, then **Merge** | The GitHub connector on; only your own pull request, approved, with its checks passing | `crates/connectors/src/github.rs`, `app/src/connectors.rs` |
| A voice model's download | You, with a click in **Settings → Chat & voice → Voice** | Nothing until you click; from Hugging Face (`ggerganov/whisper.cpp`), checked against its SHA-256 | `crates/voice/src/models.rs` |
| The speech detector's download (Silero VAD, under 1 MB) | The app, by itself, outside the gate, while voice is on and it is missing | A voice model you chose; from Hugging Face (`ggml-org/whisper-vad`), checked against its SHA-256 | `crates/voice/src/models.rs`, `app/src/voice.rs` |
| Claude Code's plan usage | Claude Code, which hands it to our status line | No request of ours | the hook |
| The update check | Not built yet (ADR 0019) | It will be off until you turn it on, through the gate | — |
