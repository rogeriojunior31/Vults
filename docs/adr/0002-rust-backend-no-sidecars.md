# 0002. Rust for every backend part; no Python sidecars

**Status:** Accepted, 2026-10-04 (the rule dates from the start of the project).

## Context

The app runs all day next to the user's agents. Every extra runtime is memory, start-up time, a
packaging problem and one more thing to keep safe. Tools like small decision models are often
shipped as Python services.

## Decision

Every backend crate and tool is Rust. TypeScript only renders the UI. No Swift, no Go, and no
Python process started by the app. A model the app runs itself goes through a Rust binding
(whisper.cpp through `whisper-rs`; ONNX through `ort` if one is ever needed). A model server the
user already runs (Ollama, LM Studio) is reached over its local HTTP API.

## Consequences

- One toolchain, one binary, packages without an interpreter.
- A library that only exists in Python is out until it has a Rust path, however good it is.
