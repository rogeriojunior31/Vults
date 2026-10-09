# 0002. Rust em todo o backend; nenhum sidecar em Python
<!-- source: aee5a29e4a96 -->

**Status:** Aceita, 2026-10-04 (a regra existe desde o início do projeto).

<a id="context"></a>

## Contexto

O app roda o dia todo ao lado dos agentes do usuário. Cada runtime a mais é memória, tempo de
inicialização, um problema de empacotamento e mais uma coisa para manter segura. Ferramentas como
pequenos modelos de decisão costumam ser distribuídas como serviços em Python.

<a id="decision"></a>

## Decisão

Todo crate e ferramenta do backend é em Rust. TypeScript só renderiza a UI. Nada de Swift, nada de
Go, e nenhum processo Python iniciado pelo app. Um modelo que o próprio app roda passa por um
binding em Rust (whisper.cpp via `whisper-rs`; ONNX via `ort`, se um dia for preciso). Um servidor
de modelos que o usuário já roda (Ollama, LM Studio) é acessado pela API HTTP local dele.

<a id="consequences"></a>

## Consequências

- Uma toolchain, um binário, pacotes sem interpretador.
- Uma biblioteca que só existe em Python fica de fora até ter um caminho em Rust, por melhor que
  seja.
