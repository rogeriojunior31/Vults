# 0026. Quem responde os cards do chat é escolha do usuário
<!-- source: d5dcc9cbebdc -->

**Status:** Aceita, 2026-10-12. Emenda a [0004](0004-a-human-answers-permissions.md) e a
[0014](0014-consent-before-autonomy.md) só para o chat do Zeca.

<a id="context"></a>

## Contexto

O chat do Zeca pergunta antes de cada comando ou edição que a CLI dele quer rodar, e só o *Always
in this chat* (um comando exato, até a conversa acabar) ou as allow rules do próprio Claude Code do
usuário pulam um card. As políticas respondem os cards do bando, mas não os do chat. Quem trabalha
com o Zeca o dia todo responde card atrás de card para chamadas que permitiria de qualquer jeito, e
pediu para escolher: as suas políticas, ou nenhum card.

O chat não é uma das sessões do bando. Ele roda numa pasta que o usuário escolheu, numa CLI que o
próprio Vults iniciou, e os cards dele aparecem e são respondidos só no Vults. O que ele deixa rodar
é decisão do usuário, desde que a escolha seja dele, deliberada, visível e registrada.

<a id="decision"></a>

## Decisão

Settings → Chat & voice → **Who answers Zeca's cards** tem três escolhas:

- **Asks first** (o padrão): um card para cada chamada que nenhuma regra *Always* cobre, como antes.
- **Your policies**: as políticas que o usuário aceitou (ADR 0014) respondem as chamadas do chat
  também, exatamente como respondem as do Claude Code na ilha: um deny nega, um allow permite (só
  chamadas do Claude Code, dentro da pasta da conversa), e o resto pergunta.
- **Full access**: toda chamada roda sem card, exceto:
  - uma chamada destrutiva (`rm`, um push forçado) sempre pergunta, como a
    [0025](0025-destructive-calls-always-ask.md) vale para regras e políticas;
  - o que uma política nega é negado, e o que uma política manda perguntar ainda pergunta;
  - depois que a conversa lê conteúdo de fora (uma página web, um fetch, uma ferramenta MCP), os
    cards dela voltam a perguntar pelo resto daquela conversa: palavras de uma página não podem
    rodar comandos que ninguém viu.

Mais acesso só é dado em Settings, como uma política; o seletor do próprio chat mostra a escolha e
só pode devolver acesso (de Full access para Your policies ou Asks first). Toda resposta que a
escolha dá vai para o log de auditoria, como a da política (com o seu consentimento) ou como
`full-access`, e o chat diz o que rodou e por ordem de quem. As sessões do bando não mudam: os cards
delas seguem a regra 2 como a ADR 0014 a escreveu.

<a id="consequences"></a>

## Consequências

- A regra 2 do `CLAUDE.md` cita a escolha do chat ao lado das outras formas de responder um card.
- Full access confia no modelo dentro da pasta do chat: uma edição ou um `git push` por engano roda
  (só o que não pode ser desfeito ainda pergunta). A configuração avisa, o chip no chat fica
  vermelho, e as linhas deny de uma política continuam sendo o jeito de deixar algumas ações fora de
  alcance.
- A CLI do chat continua rodando com `--permission-mode default`: as respostas vêm do Vults, então o
  log de auditoria, a proteção contra conteúdo de fora e o Stop continuam funcionando.
