# 0027. Uma resposta aberta para qualquer ferramenta que peça uma
<!-- source: d7f5fb9b2581 -->

**Status:** Proposta, 2026-10-12. Emenda a [0004](0004-a-human-answers-permissions.md) e a
[0004](0004-a-human-answers-permissions.md), dentro da [0003](0003-never-block-an-agent.md) e da
[0004](0004-a-human-answers-permissions.md).

<a id="context"></a>
## Contexto

O Vults responde a uma permissão só para os agentes que conhece: cada um lê o próprio formato de
resposta (o `hookSpecificOutput` do Claude Code, o `behavior` puro do Copilot CLI), então cada um
foi adicionado à mão depois de uma sessão gravada. Qualquer outra ferramenta já pode pôr as sessões
dela no fio com `vults-hook --agent <name>` e o JSON de hooks do Claude Code, mas o
`PermissionRequest` dela aparece como uma pergunta esperando no terminal e o hook nunca espera: nada
no app sabe o que essa ferramenta leria de volta (veja *Permissões* em
[Outros agentes](../guide/other-agents.md)).

Isso deixa a porta fechada para as ferramentas que as pessoas constroem (scripts, agentes internos,
orquestradores que rodam vários agentes), que são justamente para o que o bando e os cards servem.
Uma ferramenta que quer um card deveria poder pedir um, se aceitar uma resposta fixa e a honrar.

<a id="decision"></a>
## Decisão

- **A ferramenta pede a resposta.** O comando de hook dela adiciona `--reply vults`:
  `vults-hook --agent <name> --reply vults PermissionRequest`. Sem isso nada muda: o hook nunca
  espera por uma ferramenta com nome, como hoje. Os nomes integrados (`claude`, `codex`,
  `copilot`, `gemini`, `opencode`, `qwen`, `other`, e todo agente que o registro lista, como
  `cursor` ou `kimi`) não podem usá-lo.
- **Ela manda o JSON de hooks do Claude Code**, o formato que a maioria das ferramentas já copia: um
  `PermissionRequest` com `session_id`, `cwd`, `tool_name` e `tool_input`. O card é montado a
  partir dele como para qualquer agente: o comando, o arquivo ou a URL inteiros, nunca um resumo que
  a ferramenta escreveu.
- **Ela lê uma resposta fixa** no stdout, uma linha JSON:
  `{"decision":"allow"}` ou `{"decision":"deny","reason":"Denied from Vults"}`.
  Qualquer outra coisa, stdout vazio incluído, significa *pergunte ao usuário você mesma*: nenhum
  clique a tempo, o app fechado ou pausado, uma versão de protocolo não suportada, qualquer falha. O
  hook continua saindo com 0 e nunca bloqueia a ferramenta ([0003](0003-never-block-an-agent.md));
  silêncio nunca é um allow.
- **Quem responde é quem responde hoje** ([0004](0004-a-human-answers-permissions.md)): um clique,
  uma resposta assinada de um celular pareado ([0004](0004-a-human-answers-permissions.md)),
  ou uma regra *Always* exata que o usuário fez no card daquela ferramenta. As regras são guardadas
  pelo nome da ferramenta, a pasta dela, a ferramenta chamada e o alvo: uma regra feita para uma
  ferramenta com nome nunca responde a outra, nem a um agente integrado.
- **As políticas** ([0004](0004-a-human-answers-permissions.md)) alcançam uma ferramenta com nome só
  quando a nomeiam (`agents my-tool`), e então só para **negar** ou **perguntar**: uma política
  nunca libera a chamada de uma ferramenta com nome, assim como hoje não libera a de nenhum agente
  além do Claude Code. Uma chamada destrutiva sempre pergunta
  ([0004](0004-a-human-answers-permissions.md)). Toda resposta é auditada com o nome da ferramenta.
- **Uma ferramenta com nome não ganha nada que o card de um agente integrado não mostraria.** Ela
  chega ao mesmo socket que qualquer processo do usuário já alcança (mesmo usuário, verificado pelo
  `peer`); o card dela mostra exatamente o que ela mandou; um Allow volta só para a chamada que
  pediu. O Vults não pode obrigar uma ferramenta a honrar a resposta, assim como não pode obrigar
  nenhum agente: uma ferramenta que a ignora é uma que nunca pediu.
- **Um nome que vira agente integrado** depois perde as regras dele: elas não passam para um agente
  que lê outra resposta.

<a id="consequences"></a>
## Consequências

- O G2 constrói isso: a flag e a resposta do hook, um aumento do `VERSION` do protocolo (um app
  antigo responde `unsupported`, então a ferramenta pergunta ela mesma), regras e políticas
  indexadas pelo nome no core, as ferramentas com nome em **Settings → Agents** com as regras delas,
  e o [Protocolo de hooks](../reference/protocol.md) escrito como uma especificação, com uma
  ferramenta de exemplo em shell e em TypeScript.
- A regra 2 do `CLAUDE.md`, a página [Segurança](../safety.md) e [Outros agentes](../guide/other-agents.md)
  dizem que uma ferramenta que pede com `--reply vults` ganha um card, e como ele é respondido.
- Uma ferramenta de teste em shell ganha um card, um allow e um deny, e pergunta ela mesma quando o
  app está fechado; um hook antigo fica em silêncio.
