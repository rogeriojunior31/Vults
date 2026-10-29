# Outros agentes
<!-- source: 2ca88fac1a77 -->

Claude Code, Codex, Antigravity, OpenCode, Hermes Agent, Qwen Code, Copilot CLI, Cursor, Factory Droid e Kimi Code já vêm integrados (**Settings →
Agents**). Qualquer outra ferramenta também pode colocar suas sessões no fio, se ela rodar um
comando nos seus eventos e mandar o JSON de hook do Claude Code pelo stdin (o formato que a maioria
das ferramentas de agente copia). Para ferramentas que rodam plugins em vez disso (Pi), um plugin
curto faz o mesmo; as receitas estão mais abaixo.

Aponte os hooks da ferramenta para o relay, com um nome à sua escolha (no Windows o relay fica em
`%LOCALAPPDATA%\Vults\bin\`):

```sh
~/.local/share/vults/bin/vults-hook --agent my-tool SessionStart
```

- O nome tem de 1 a 24 caracteres de `a-z`, `0-9` e `-`, e não pode ser um para o qual o fio tem um
  tipo (`claude`, `codex`, `copilot`, `opencode`, `qwen` e `other`), nem `gemini`: o fio não tem mais
  um tipo para ele, mas os hooks antigos do Gemini CLI podem continuar instalados, então o nome
  segue reservado e eles não mandam nada. Os integrados que o
  Vults manda por nome (`antigravity`, `hermes`, `cursor`, `droid`, `kimi`) usam esses nomes eles mesmos, então
  não ficam reservados; uma ferramenta com qualquer nome não ganha card, inclusive um integrado
  mandado por nome. Com um nome que quebra essas regras, o hook não manda nada e sai na hora.
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
| Antigravity | o terminal dele | o terminal dele | linhas | pelo Vults, ainda não | nenhum | ainda não |
| OpenCode | um card | um card (OpenCode 1) | linhas | não pelo Vults | nenhum | não |
| Hermes Agent | no terminal dele | no terminal dele | linhas do patch e prévia do repositório | não pelo Vults | gerenciado pelo agente | não |
| Qwen Code | um card | o terminal dele | linhas | não pelo Vults | um contêiner, se ligado | não |
| Copilot CLI | um card | nenhuma | linhas | não pelo Vults | nenhum | não |
| Cursor | o terminal dele | o terminal dele | linhas | não pelo Vults | nenhum | não |
| Factory Droid | o terminal dele | o terminal dele | linhas | não pelo Vults | nenhum | não |
| Kimi Code | o terminal dele | o terminal dele | linhas | não pelo Vults | nenhum | não |

O Vults para apenas os próprios turnos do Chat ou uma sessão que retomou pelo `/continue`
([Chat](chat.md#continuing-a-session)). Uma sessão observada no terminal continua sob seu controle.
O sandbox do Codex depende do modo: `workspace-write` escreve só na pasta dele, nas pastas temporárias e nas pastas que recebe. *Se
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

A ilha mostra o Antigravity pensando, cada passo (`view_file`, `run_command`…) e quando ele para,
e o diff de cada edição, refeito a partir do texto que ela trocou (sem números de linha). O
Antigravity não manda a resposta dele, então uma sessão terminada não tem linha de resumo, e não há
início nem fim de sessão. O silêncio não a remove mais: uma sessão aberta pode descansar ou voar, e a limpeza exige confirmar a saída de um processo local antes observado. O hook dele roda antes de toda ferramenta e não diz se o Antigravity
teria perguntado, então o Antigravity pede as permissões ele mesmo
([Aprovando](approvals.md#agents-that-ask-in-their-own-prompt)).

## Cursor

**Settings → Agents → Cursor** adiciona os hooks dele a `~/.cursor/hooks.json`, o arquivo que o
editor e o `cursor-agent` compartilham; os hooks de outras ferramentas nele ficam, e **Remove
hooks…** tira só os do Vults. Reinicie o Cursor depois. As sessões dele aparecem como `cursor`, no
projeto em que o Cursor trabalha.

A ilha mostra cada passo terminado, falhas incluídas, e quando um turno para. Os hooks dele que
rodam antes de um passo (`preToolUse`, `beforeSubmitPrompt`, `beforeShellExecution`…) ficam de fora:
o Cursor lê o que eles imprimem como uma decisão, então o Cursor pergunta as permissões ele mesmo
([Aprovando](approvals.md#agents-that-ask-in-their-own-prompt)). Cada edição vem do
`afterFileEdit`, de que o Cursor não lê nada, com o seu diff (sem números de linha). O que uma
ferramenta imprimiu e o email da conta nunca saem do hook.

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
o Droid pergunta as permissões ele mesmo ([Aprovando](approvals.md#agents-that-ask-in-their-own-prompt)).

## Kimi Code

**Settings → Agents → Kimi Code** adiciona os hooks dele a `~/.kimi-code/config.toml`
(`KIMI_CODE_HOME` o move), como tabelas `[[hooks]]` ao lado das suas configurações: seus comentários
e a formatação ficam, e os hooks de outras ferramentas também. Reinicie o Kimi depois. As sessões
dele aparecem como `kimi` (conferido com o Kimi Code 2.1.1).

A ilha mostra cada passo, as linhas de uma edição, um comando que falhou e quando um turno para. O
Kimi avisa o hook `PermissionRequest` sobre uma permissão sem esperar resposta, então a ilha mostra
que ela espera no terminal do Kimi e o Kimi pergunta ele mesmo ([Aprovando](approvals.md#agents-that-ask-in-their-own-prompt)).

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

## Hermes Agent

Instale em **Configurações → Agentes → Hermes Agent**, execute
`hermes plugins enable vults --no-allow-tool-override` e abra uma nova sessão no CLI. O gateway
em execução pode recarregar o plugin imediatamente. Os arquivos ficam em
`~/.hermes/plugins/vults/{__init__.py,plugin.yaml}`; `HERMES_HOME` permite outra pasta. O instalador
mostra o diff, verifica os dois arquivos antes de escrever, mantém backups datados e preserva
arquivos de terceiros. Antes de remover, use `hermes plugins disable vults`.

O plugin acompanha sessões, turnos, ferramentas, falhas, identificadores nativos e subagentes.
Edições por `patch` mantêm seus diffs; a prévia do repositório também cobre scripts e escritas.
Permissões, perguntas e interrupção ficam no Hermes. Nenhum argumento ou resultado de ferramenta
é alterado.

As métricas usam os números de `post_api_request` e uma consulta somente leitura à sessão
conhecida em `HERMES_HOME/state.db`. Contexto significa os tokens da chamada mais recente,
incluindo cache, sem somar toda a conversa. A porcentagem exige um limite informado pelo Hermes.
O tempo de resposta soma durações de chamadas concluídas observadas desde o carregamento do
plugin, sem duplicar IDs repetidos; não inclui ferramentas nem intervalos ociosos. O custo usa
o valor real informado pelo Hermes ou sua estimativa explícita. Ausência de dados continua
desconhecida; assinatura incluída só vira zero quando o Hermes informa isso. Custos menores que
um centavo permanecem visíveis. Nenhuma chave, conversa ou resposta bruta é lida ou encaminhada,
e consultar as métricas não faz uma chamada ao modelo.

Validação no Hermes 0.22.2 instalado: o diagnóstico nativo aceitou o plugin e o dispatcher real
entregou sessão, métricas controladas de contexto/tempo e patch de +1/−1 ao Flock em execução,
sem inferência. A seleção de custo nativo é verificada com um banco isolado.

No Hermes Desktop, finalizar ou descarregar um runtime significa repouso: a conversa persistida pode continuar aberta. Um reset encerra apenas a sessão antiga explicitamente identificada e inicia/retoma a atual. O campo persistido `ended_at` no banco local ou um evento explícito de exclusão encerra a ave; dados de ciclo de vida ausentes/ilegíveis não a removem.
