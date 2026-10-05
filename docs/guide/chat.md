# Chatting with Zeca

Open the **Chat** tab, choose **Chat…** from the tray icon, or drop a file on the island. Zeca sits on
the left of the conversation: he thinks while the reply comes, tilts his head when he asks you
something, and swallows the files you give him. Pick **Claude** or **Codex** at the top (or the
API provider you set up, see below). **New** starts a fresh conversation; when there is one to lose, it asks first, and so
does switching to another provider. **Esc** closes the chat from anywhere in it; the conversation is
still there when you come back.

## Where it works

Before the first message, the folder at the top (*in vultures-ai ▾*) says where the conversation
will work: the folder of the session in front, unless you pick another one of the sessions on the
wire, or an empty folder of its own. Once the conversation starts it stays there until **New**.

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

- Click the mic and speak: a waveform shows what it hears. When you stop talking (about 0.6 s of
  quiet after your words) it stops by itself, or click again (the red stop) any time. Zeca thinks
  while it turns into text; the words land in the input for you to read and fix, then **Enter**
  sends them as usual. Silence before you start never stops it, so take your time; a long pause
  in the middle of a sentence does, so click the mic again to go on (the new words are added).
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
  with a model, or when you click **Use** on one. If your model was downloaded before it existed,
  click **Turn off**, then **Use** on your model to get it. Without it the mic only stops on a
  click and the cut goes by loudness, as before.

On Linux it runs on the graphics card through Vulkan when there is one (AMD, Intel or NVIDIA):
a sentence takes a fraction of a second even with *Large v3 Turbo*. Without a usable GPU it runs on
the processor, where *Base* is the one to pick.

Everything happens on this computer: whisper.cpp transcribes the audio in memory, and the audio is
never saved or sent anywhere. The models come from the whisper.cpp repositories on Hugging Face
(`ggerganov/whisper.cpp`, and `ggml-org/whisper-vad` for the speech detector) and are checked
against their known SHA-256 before they are used.

## What it may do

<img src="../assets/island-chat-permission.png" width="640" alt="Zeca asks in the chat before running a command">

- It reads files in that folder freely.
- Every command and every edit shows up in the conversation as a card like the island's own: what
  Zeca means to do, the whole command or the file with its **+/−** lines, and **Deny** and **Allow**
  with their keys (**Ctrl+Alt+N** / **Ctrl+Alt+Y** answer it from anywhere). Nothing runs until you
  answer, and a card nobody answers is a no. There is no **Always** here: each chat asks every time.

## Files

Drag a file over the island and it opens on a drop zone, Zeca waiting with his bill open (the **+**
tab shows it too). Drop it: he swallows it, a vulture carries it across while a bar fills, and the
zone asks what it is for. **Ask about it** keeps it on your next message and puts you in the input;
**Cancel** takes it off. You can send it with no text to ask what it is.
Images, PDFs, code and text work best. Files are copied into the app's inbox (up to 20 MB each) and
deleted after a week; a folder, or a file over 20 MB, is refused with a line saying why.

## Your subscription, your login

The chat runs the `claude` or `codex` command you already logged into, so it uses your own
subscription and Vultures AI never reads your credentials. Both stream the reply as it is written:
Claude through `claude -p`, Codex through one long-lived `codex app-server`.

Chat turns ignore your hooks, settings and MCP servers, so a chat never shows up on the island as an
agent session, and no saved permission rule lets a command skip the card.

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
- This chat only talks: it has no tools, so it can't run commands, edit files or look around your
  project. It reads what you type and the files you drop: images as they are, text files inline (up
  to 512 KB), and PDFs too with Anthropic.
- Reasoning models that think out loud (DeepSeek-R1, Qwen3 and others on Ollama or LM Studio) only
  show their answer: the `<think>` part is hidden, and isn't sent back in the next message either.
- Choosing another provider starts a new conversation: the new one has none of the old.
- With Anthropic, if Claude declines a request on safety grounds, the API retries it on another Claude
  model within the same call (Anthropic's server-side fallback). If that model declines too, the
  bubble says so.
