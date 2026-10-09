# Protocolo do hook
<!-- source: 7c88821a7dbe -->

O `vults-hook` e o app trocam um objeto JSON por linha através de um socket local.

<a id="transport"></a>

## Transporte

| Plataforma | Endpoint | Controle de acesso |
|---|---|---|
| Linux | `$XDG_RUNTIME_DIR/vults.sock` (alternativa `/tmp/vults-<uid>/`, pasta `0700`) | socket `0600`, `SO_PEERCRED` |
| Windows | `\\.\pipe\vults-<SID>` | as duas pontas verificam o SID do outro processo |
| macOS | planejado | `getpeereid` |

<a id="messages"></a>

## Mensagens

Do hook para o app, versão 3:

```json
{ "kind": "event", "v": 3, "id": "18f…-1a2b", "agent": "claude", "event": "PermissionRequest",
  "wants_reply": true,
  "terminal": { "cwd": "/home/me/project", "pid": 4242, "env": { "TERM_PROGRAM": "kitty" } },
  "payload": { "tool_name": "Bash", "tool_input": { "command": "cargo test" } } }
```

- `agent`: `claude`, `codex`, `gemini`, ou `other` para qualquer outra ferramenta (veja [Outros agentes](../guide/other-agents.md)).
- `agent_name`: só com `other`, o nome da ferramenta: de 1 a 24 caracteres entre `a-z`, `0-9` e `-`, nunca `claude`,
  `codex`, `gemini` ou `other`. Ausente nos outros casos.
- `id`: único por mensagem, opaco.
- `terminal`: todos os campos são opcionais. `env` lista só as variáveis que identificam o terminal e estavam definidas.
  `cwd` é o `cwd` do payload, senão a primeira entrada de `workspacePaths` (Antigravity) ou `workspace_roots` (Cursor), senão a
  pasta de trabalho do próprio hook.
- `payload`: o JSON de hook do agente sem `tool_response`, `transcript_path` (e os `transcriptPath` e
  `artifactDirectoryPath` do Antigravity); as strings são limitadas
  a 2000 bytes. Duas coisas sobrevivem em `tool_response`: o `error` de uma ferramenta e, num `PostToolUse`, o
  `structuredPatch` do Claude Code, cortado nas primeiras 400 linhas (com `"cut": true` quando linhas ficaram de fora).
  Num `PostToolUse` do `apply_patch` do Codex, `tool_input.command` (o patch) guarda até 64 KiB.
  Os campos são opcionais e o envelope não mudou, então isso não precisou de uma versão nova.

Um evento espera uma resposta (`wants_reply`) quando é um `PermissionRequest` do Claude Code ou do
Codex, ou um `PreToolUse` do Claude Code para `AskUserQuestion` enviado por uma entrada instalada com `--ask`
(`vults-hook --agent claude --ask PreToolUse`, com um timeout de 120 segundos). Uma entrada sem a
flag tem o timeout curto de todos os outros eventos, então ela nunca espera.

Do app para o hook, só quando `wants_reply` é true:

```json
{ "kind": "decision", "v": 3, "id": "18f…-1a2b", "decision": "allow" }
{ "kind": "answer", "v": 3, "id": "18f…-1a2b", "answers": ["Blue", ["S", "M"]] }
{ "kind": "unsupported", "v": 3, "id": "18f…-1a2b" }
```

`answers` tem uma entrada por pergunta, na ordem de `tool_input.questions`: uma string (o rótulo de uma
opção, ou as palavras do próprio usuário) ou, numa seleção múltipla, uma lista delas. Vai por posição porque
o app só viu as perguntas com as strings limitadas; o hook associa as respostas às
perguntas que ele leu.

Um evento de `gemini` ou `other` nunca tem `wants_reply`: os hooks do Gemini não conseguem aprovar uma ferramenta, e
nada no app responde à permissão de outra ferramenta.

`StatusLine` é a entrada de statusLine do Claude Code, enviada por `vults-hook --agent claude --statusline`.
O payload dele é só `rate_limits` (as janelas de 5 horas e semanal do plano) e `session_id`; os
caminhos, o custo e o modelo da sessão nunca saem do hook. Antes da primeira resposta da sessão não há
`rate_limits`, e nada é enviado. O hook imprime só o que o status line do próprio usuário imprime
(salvo na instalação, veja [configurações](settings.md)), ou nada quando não há nenhum, então o status line
do Claude Code fica como era antes.

A versão 2 adicionou `other` e `agent_name`, depois `gemini`. A versão 3 adicionou `answer`. O app instala o próprio hook quando inicia, então os dois
sempre falam a mesma versão; um evento de outra versão recebe `unsupported`.

Uma conexão que não recebe resposta, uma resposta para outro `id`, ou `unsupported` fazem o hook não
imprimir nada.

<a id="limits"></a>

## Limites

| Limite | Valor |
|---|---|
| Tamanho da mensagem | 1 MiB |
| Hook: conectar | 300 ms |
| Hook: evento sem resposta | 2 s no total |
| Hook: esperando uma decisão | 110 s |
| App: confirmação de um card pela UI | 800 ms |
| App: decisão | 108 s |
| App: leitura de uma mensagem | 5 s |
| App: conexões simultâneas | 32 |

<a id="agent-output"></a>

## Saída para o agente

O hook transforma uma decisão no formato que cada agente espera. O Claude Code e o Codex leem a mesma
saída de `PermissionRequest`:

```json
{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}
{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"Denied from Vults"}}}
```

Uma resposta vira a saída de `PreToolUse` do Claude Code: a ferramenta roda com as respostas adicionadas à
própria entrada, associadas ao texto de cada pergunta (verificado com o Claude Code 2.1.286):

```json
{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow",
  "updatedInput":{"questions":[…],"answers":{"Which color?":"Blue","Which sizes?":["S","M"]}}}}
```

Quando o número de respostas não bate com o de perguntas, o hook não imprime nada e o Claude Code
pergunta no terminal dele.
