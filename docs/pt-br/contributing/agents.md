# Adicionando um agente
<!-- source: 5cc0b9c2ed01 -->

O Vults mostra as sessões de um agente de código pelos hooks que o agente roda nos próprios eventos.
A maioria dos agentes cabe em poucas centenas de linhas: um arquivo em `crates/agents`, uma linha no
registro, uma sessão gravada e as docs. Esta página é o checklist, na ordem em que o trabalho anda.

<a id="watch-or-answer"></a>

## Observar ou responder

Decida isso primeiro, a partir de uma execução real do agente, nunca só pela documentação dele: o
Gemini CLI, o Antigravity e o Cursor pareciam respondíveis no papel e não são.

- **Só observar** (Antigravity, Cursor, Factory Droid): a ilha mostra os passos; o agente pergunta as
  permissões no próprio prompt. Esse é o padrão, e o primeiro PR certo.
- **Responder** (Claude Code, Codex, Qwen Code, Copilot CLI): o agente tem um evento enviado só quando
  perguntaria ao usuário (um `PermissionRequest`), aceita um allow ou um deny do hook e, com uma
  resposta vazia, pergunta no próprio prompt. Um hook que roda antes de *toda* ferramenta
  (`PreToolUse`) não basta: ele não diz se o agente teria perguntado.

Silêncio nunca é aprovação: sem clique, vale o prompt do próprio agente, nunca allow, nunca deny.

<a id="1-record-a-session"></a>

## 1. Grave uma sessão

O kit de spike roda o CLI de verdade contra um modelo falso e um app falso, sem conta e sem tocar em
nada fora de uma pasta de rascunho:

```sh
scripts/spike.sh <agent> [allow|deny|none] [script.json]
```

Adicione uma receita para o agente em `scripts/spike.sh`: aponte as configurações de modelo dele (a
maioria aceita uma URL base própria da OpenAI ou da Anthropic) para `$base`. Se o modelo falso não
conhece os nomes das ferramentas do agente, ensine `crates/spike/src/model.rs` (`plain()`), com um
teste. A execução deixa `events.jsonl` (o que o hook mandou), `replay.txt` (o que a ilha faz disso)
e `fixture.jsonl`. Um agente que precisa dos servidores do fornecedor não pode ser gravado assim:
diga isso no PR e no plano.

<a id="2-write-the-agent"></a>

## 2. Escreva o agente

Um arquivo, `crates/agents/src/<agent>.rs`, implementando a trait `Agent`:

- `parse`: os eventos do agente nas palavras do Claude Code. A maioria dos agentes copiou o JSON de
  hooks do Claude Code; mapeie o que difere (nomes de eventos, nomes de ferramentas como `Execute` →
  `Bash`, nomes de argumentos como `old_str` → `old_string`) e passe para `other::as_claude`.
  Eventos e ferramentas desconhecidos falham com suavidade: um passo genérico, nunca um crash.
- `config_file`, `hook_entries`, e `install` / `uninstall` / `installed` / `up_to_date` quando
  o arquivo não tem o formato do Claude Code. O `vults-agent-config` tem os escritores: hooks JSON
  no estilo do Claude Code, chaves com nome (`named`), listas planas por evento (`flat`), TOML
  (`toml`) e arquivos de plugin inteiros (`plugin`). Os hooks de outras ferramentas num arquivo
  compartilhado sempre ficam.
- Uma variável de pasta que move os arquivos do agente vai em `dirs.rs` (`DIR_VARS`), lida do jeito
  que o agente a lê.
- Se um hook que falha para o agente, termine todo comando em `|| exit 0`. Se o agente lê qualquer
  saída de um hook como decisão, não registre esse hook.
- Campos que levam a saída de uma ferramenta ou uma conta (`tool_output`, `user_email`) são
  descartados no hook (`DROPPED_FIELDS` em `crates/hook/src/lib.rs`); só um erro ou um código de
  saída fica.

Teste contra a fixture gravada (`crates/agents/tests/fixtures/<agent>-session.jsonl`, com os caminhos
reescritos para `/home/me`), mais uma instalação e uma remoção que mantêm o resto do arquivo.

<a id="3-add-the-registry-row"></a>

## 3. Adicione a linha no registro

`crates/agents/src/registry.rs` guarda uma `Entry` por agente: o id, o nome, o comando, como ele vai
no fio (`Wire::Named` para um agente só observado, enviado como `other` com o nome dele), a
instalação, como o Vults o roda, e as `Caps` dele (o que **Settings → Agents** mostra). Atualize os
testes do registro e depois regenere o gêmeo da UI:

```sh
VULTS_REGEN=1 cargo test -p vults-agents agents_ts
```

Um agente respondível também precisa do próprio `AgentKind` (um aumento do `VERSION` do protocolo),
o formato da resposta dele em `crates/hook/src/output.rs`, o nome dele em `crates/core/src/notify.rs`,
o gêmeo da view (`VULTS_REGEN=1 cargo test -p vults-core view_ts`), a cor dele na UI e as listas de
agentes dos alvos de fuzz. O PR do Copilot CLI é o exemplo.

<a id="4-settings-and-docs"></a>

## 4. Settings e docs

- Uma nota sob o card dele em **Settings → Agents** (`ui/src/surfaces/settings/main.ts`), com as
  traduções em `ui/src/i18n/{pt-BR,es,zh}.ts`.
- Em inglês e em português do Brasil: a seção dele em [Outros agentes](../guide/other-agents.md) (e a
  linha em *O que cada agente consegue fazer*; ele sai das receitas), [Aprovando](../guide/approvals.md),
  a tabela em [Primeiros passos](../getting-started.md), as listas de agentes nos READMEs,
  `docs/README.md` e `architecture.md`.
- Uma linha no `CHANGELOG.md`, e o passo dele marcado como feito em `docs/dev/plan-every-agent.md`
  com o que o spike mostrou.

Depois, as verificações em
[CONTRIBUTING.md](https://github.com/rogeriojunior31/Vults/blob/main/CONTRIBUTING.md#before-opening-a-pr);
um agente novo alarga um select ou dois, então os testes visuais podem precisar de baselines novas
de propósito.
