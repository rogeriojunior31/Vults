# Getting started

There is no release yet. To build the hook relay from source:

```sh
git clone https://github.com/rogeriojunior31/vultures-ai
cd vultures-ai
cargo build --release -p vultures-ai-hook
cargo test --workspace
```

Installing the hooks into Claude Code and Codex will be done by the app, with a backup and a diff you
approve, once it exists (milestone M1).
