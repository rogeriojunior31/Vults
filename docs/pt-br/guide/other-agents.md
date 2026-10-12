# Outros agentes
<!-- source: 08e08d2f2d9f -->

Claude Code, Codex, Antigravity, OpenCode, Qwen Code, Copilot CLI, Cursor e Factory Droid já vêm integrados (**Settings →
Agents**). Qualquer outra ferramenta também pode colocar suas sessões no fio, se ela rodar um
comando nos seus eventos e mandar o JSON de hook do Claude Code pelo stdin (o formato que a maioria
das ferramentas de agente copia). Para ferramentas que rodam plugins em vez disso (Pi), um plugin
curto faz o mesmo; as receitas estão mais abaixo.

Aponte os hooks da ferramenta para o relay, com um nome à sua escolha (no Windows o relay fica em
`%LOCALAPPDATA%\Vults\bin\`):

```sh
~/.local/share/vults/bin/vults-hook --agent my-tool SessionStart
```

- O nome tem de 1 a 24 caracteres de `a-z`, `0-9` e `-`. `claude`, `codex`, `copilot`, `gemini`,
  `opencode`, `qwen` e `other` já estão em uso, para que nada se passe por um agente integrado (`gemini` segue em uso,
  embora o Vults não suporte mais o Gemini CLI: os hooks antigos dele não mandam nada). Com um nome que
  quebra essas regras, o hook não manda nada e sai na hora.
- O nome do evento vem de `hook_event_name` no JSON, ou do último argumento. Os nomes de evento do
  Claude Code são lidos, e os do Cursor também.
- Cada sessão precisa de um id: `session_id`, ou `conversation_id`, `sessionId` ou `conversationId`.
  Um evento sem id é descartado. As sessões aparecem com o nome da ferramenta, em violeta.

Os campos que ela lê estão em [Protocolo do hook](../reference/protocol.md). Para essas
ferramentas, o relay sempre sai com 0 e não imprime nada, mas um relay ausente faz o comando falhar:
termine todo comando de hook em `|| exit 0`, ou uma ferramenta que trata um hook com falha como um
"não" para de funcionar quando o app não está instalado.

<a id="what-each-agent-can-do"></a>

## O que cada agente consegue fazer

**Settings → Agents**, em *What each agent can do*, mostra cada agente embutido: se a CLI dele está no `PATH` e a versão, se os
hooks estão instalados e atualizados (e, no Codex, confiáveis), e o que ele consegue fazer com o
Vults. **Check again** pergunta de novo; a resposta fica guardada por dez minutos.

| | Permissões | Perguntas | Diffs | Parar um turno | Sandbox | Iniciado pelo Vults |
|---|---|---|---|---|---|---|
| Claude Code | um card | um card | linhas, numeradas | na hora, quando o Vults o iniciou | bubblewrap, se ligado | sim |
| Codex | um card | o terminal dele | linhas | na hora, quando o Vults o iniciou | o modo de sandbox dele | sim |
| Antigravity | o terminal dele | o terminal dele | nenhum | pelo Vults, ainda não | nenhum | ainda não |
| OpenCode | um card | um card (OpenCode 1) | linhas | na hora, quando o Vults o iniciou | nenhum | uma tarefa, por ACP |
| Qwen Code | um card | o terminal dele | linhas | na hora, quando o Vults o iniciou | um contêiner, se ligado | uma tarefa, por ACP |
| Copilot CLI | um card | nenhuma | linhas | na hora, quando o Vults o iniciou | nenhum | uma tarefa, por ACP |
| Cursor | o terminal dele | o terminal dele | nenhum | não pelo Vults | nenhum | não |
| Factory Droid | o terminal dele | o terminal dele | linhas | não pelo Vults | nenhum | não |

O Vults para um turno, e mantém a sessão, só numa sessão que ele iniciou (a do chat, ou a de uma
tarefa): nenhum sinal faz isso, um sinal encerra a sessão inteira. OpenCode, Qwen Code e
Copilot CLI pegam tarefas pelo [Agent Client Protocol](https://agentclientprotocol.com) (**Start with
another agent…** na aba Tasks, quando o agente está instalado); veja [o guia do
chat](chat.md#tasks). O sandbox do Codex depende do modo:
`workspace-write` escreve só na pasta dele, nas pastas temporárias e nas pastas que recebe. *Se
ligado*: o sandbox fica desligado até você ligar nesse agente. Nenhum desses sandboxes foi testado
com o Vults ainda.

<a id="permissions"></a>

## Permissões

A ilha nunca responde à permissão de uma ferramenta que não vem integrada: ela não tem como saber o
que essa ferramenta espera de volta. Um `PermissionRequest` aparece como uma pergunta esperando no
terminal, o hook não espera, e a ferramenta pergunta a você lá, como de costume.

## Antigravity

**Settings → Agents → Antigravity** instala um hook chamado `vults` em `~/.gemini/config/hooks.json`,
o arquivo que o CLI agy, o app Antigravity e a IDE compartilham (testado com o agy 1.2.16). Os
outros hooks dali nunca são tocados; se um hook seu já se chamar `vults`, o instalador não escreve
nada. As sessões dele aparecem como `antigravity`.

A ilha mostra o Antigravity pensando, cada passo (`view_file`, `run_command`…) e quando ele para. O
Antigravity não manda a resposta dele, então uma sessão terminada não tem linha de resumo, e não há
início nem fim de sessão: uma sessão sai do fio depois de um tempo em silêncio (10 minutos depois de
terminar, 30 nos outros casos). O hook dele roda antes de toda ferramenta e não diz se o Antigravity
teria perguntado, então o Antigravity pede as permissões ele mesmo
([Aprovando](approvals.md#antigravity)).

## Cursor

**Settings → Agents → Cursor** adiciona os hooks dele a `~/.cursor/hooks.json`, o arquivo que o
editor e o `cursor-agent` compartilham; os hooks de outras ferramentas nele ficam, e **Remove
hooks…** tira só os do Vults. Reinicie o Cursor depois. As sessões dele aparecem como `cursor`, no
projeto em que o Cursor trabalha.

A ilha mostra cada passo terminado, falhas incluídas, e quando um turno para. Os hooks dele que
rodam antes de um passo (`preToolUse`, `beforeSubmitPrompt`, `beforeShellExecution`…) ficam de fora:
o Cursor lê o que eles imprimem como uma decisão, então o Cursor pergunta as permissões ele mesmo
([Aprovando](approvals.md#cursor)). O que uma ferramenta imprimiu e o email da conta nunca saem do
hook.

O Cursor também roda os hooks do Claude Code (os hooks *third-party* dele, ligados por padrão). O
relay ignora os eventos do Cursor ali, então uma sessão nunca aparece duas vezes, uma como Claude
Code.

## Factory Droid

**Settings → Agents → Factory Droid** adiciona os hooks dele a `~/.factory/settings.json` (o
Droid lê os hooks do Claude Code; o resto do arquivo e os hooks de outras ferramentas ficam).
Reinicie o Droid depois. As sessões dele aparecem como `droid` (conferido com o droid 0.239.0).

A ilha mostra cada passo quando começa e termina, as linhas de uma edição e quando um turno para.
Um comando que falha não manda evento próprio: o passo dele termina com o turno. O Droid não tem
evento de permissão, e o `PreToolUse` dele roda antes de toda ferramenta, pedisse ele ou não, então
o Droid pergunta as permissões ele mesmo ([Aprovando](approvals.md#factory-droid)).

## OpenCode

**Settings → Agents → OpenCode** escreve um plugin em `~/.config/opencode/plugins/vults.js`, com o
mesmo diff, backup e clique dos hooks; reinicie o OpenCode depois. Ele informa cada passo e coloca as
permissões e as perguntas do OpenCode na ilha ([Aprovando](approvals.md#opencode)). Um arquivo ali
sem a primeira linha *Generated by Vults* é seu, e o instalador não mexe nele. O OpenCode não tem um
evento para quando ele fecha, então os pássaros dele saem do fio depois de um tempo em silêncio.

<a id="recipes"></a>

## Receitas

Estas receitas foram conferidas com a documentação e a cópia instalada de cada ferramenta (versões
abaixo), inclusive com o relay ausente; nenhuma foi rodada dentro de uma sessão real da ferramenta.
Cada uma só informa: nunca responde a uma permissão.

### Pi

O Pi (1.0) carrega extensões de `~/.pi/agent/extensions/`. Salve isto como
`~/.pi/agent/extensions/vults.ts`. Ele escuta `tool_execution_start` e `_end`, nunca `tool_call`:
um handler de `tool_call` que falha para a ferramenta.

```ts
import { spawn } from "node:child_process";
import { homedir } from "node:os";
import { join } from "node:path";

const RELAY = join(homedir(), ".local/share/vults/bin/vults-hook");

function toolInput(args: any): Record<string, unknown> {
  const a = args ?? {};
  const out: Record<string, unknown> = {};
  if (typeof a.command === "string") out.command = a.command;
  if (typeof a.path === "string") out[a.pattern === undefined ? "file_path" : "path"] = a.path;
  if (typeof a.pattern === "string") out.pattern = a.pattern;
  return out;
}

let queue: Promise<void> = Promise.resolve();

function run(event: string, payload: string): Promise<void> {
  return new Promise((done) => {
    try {
      const child = spawn(RELAY, ["--agent", "pi", event], { stdio: ["pipe", "ignore", "ignore"], detached: true });
      setTimeout(done, 2000).unref();
      child.on("error", () => done());
      child.on("exit", () => done());
      child.stdin?.on("error", () => {});
      child.stdin?.end(payload);
      child.unref();
    } catch { done(); }
  });
}

function send(event: string, ctx: any, extra: Record<string, unknown> = {}): void {
  try {
    const payload = JSON.stringify({ hook_event_name: event, session_id: ctx.sessionManager.getSessionId(), cwd: ctx.cwd, ...extra });
    queue = queue.then(() => run(event, payload));
  } catch {}
}

function text(content: any): string | undefined {
  if (!Array.isArray(content)) return undefined;
  const s = content.filter((c) => c?.type === "text").map((c) => c.text).join("\n").trim();
  return s || undefined;
}

export default function (pi: any) {
  let last: string | undefined;
  let failed = false;
  pi.on("session_start", (_e: any, ctx: any) => send("SessionStart", ctx));
  pi.on("session_shutdown", (_e: any, ctx: any) => send("SessionEnd", ctx));
  pi.on("before_agent_start", (_e: any, ctx: any) => { last = undefined; failed = false; send("UserPromptSubmit", ctx); });
  pi.on("tool_execution_start", (e: any, ctx: any) =>
    send("PreToolUse", ctx, { tool_name: e.toolName, tool_input: toolInput(e.args) }));
  pi.on("tool_execution_end", (e: any, ctx: any) =>
    send(e.isError ? "PostToolUseFailure" : "PostToolUse", ctx, { tool_name: e.toolName }));
  pi.on("agent_end", (e: any, ctx: any) => {
    try {
      const reply = [...(e.messages ?? [])].reverse().find((m: any) => m?.role === "assistant");
      if (reply?.stopReason === "error") { failed = true; send("StopFailure", ctx, { error: reply.errorMessage }); }
      else last = text(reply?.content) ?? last;
    } catch {}
  });
  pi.on("agent_settled", (_e: any, ctx: any) => { if (!failed) send("Stop", ctx, { last_assistant_message: last }); });
}
```

`ctx.sessionManager.getSessionId()` existe no Pi, mas não está na documentação dele; se um Pi futuro
o remover, a extensão não manda nada em vez de falhar.

<a id="tools-that-copy-claude-codes-hooks"></a>

### Ferramentas que copiam os hooks do Claude Code

Estas rodam um comando por evento com o JSON do Claude Code, então o relay com `--agent <name>` é
tudo de que precisam. Tirado da documentação delas, não testado aqui:

| Ferramenta | Onde ficam os hooks | Nome a usar |
|---|---|---|
| Kimi CLI | `[[hooks]]` no `config.toml` dele | `--agent kimi` |

Os nomes das ferramentas delas são diferentes dos do Claude Code (`WriteFile`, `ReadFile`…), então
alguns passos aparecem como trabalho genérico.

<a id="trying-it"></a>

## Testando

```sh
echo '{"hook_event_name":"SessionStart","session_id":"s1","cwd":"'$PWD'"}' \
  | ~/.local/share/vults/bin/vults-hook --agent my-tool
echo '{"hook_event_name":"PreToolUse","session_id":"s1","tool_name":"Bash","tool_input":{"command":"make"}}' \
  | ~/.local/share/vults/bin/vults-hook --agent my-tool
echo '{"hook_event_name":"SessionEnd","session_id":"s1"}' \
  | ~/.local/share/vults/bin/vults-hook --agent my-tool
```

O protocolo por trás disso está em [Protocolo do hook](../reference/protocol.md).
