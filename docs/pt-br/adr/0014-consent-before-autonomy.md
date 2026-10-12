# 0014. Uma nova regra de consentimento antes de qualquer autonomia
<!-- source: 8850b35d0d4b -->

**Status:** Aceita, 2026-10-10 (proposta em 2026-10-04). Emenda a [0004](0004-a-human-answers-permissions.md).
Emendada pela [0025](0025-destructive-calls-always-ask.md) (nenhuma regra *Always* responde a uma
chamada destrutiva também) e pela [0026](0026-the-chats-access-is-the-users-choice.md) (o chat do
Zeca). Construída: o log de auditoria e o motor de políticas existem (`docs/dev/plan-zeca.md`, S2 e
W2).

<a id="context"></a>

## Contexto

A 0.2.0 traz políticas (permitir este tipo de comando neste projeto) e autonomia (deixar um agente
seguir sem perguntar). As duas respondem a permissões sem um clique, o que a
[0004](0004-a-human-answers-permissions.md) proíbe hoje, exceto para regras *Always* exatas.

<a id="decision"></a>

## Decisão

Uma permissão também pode ser respondida por uma política quando o usuário a escreveu, a viu como
um diff e clicou para aceitá-la, como uma config de agente
([0005](0005-agent-configs-backup-diff-click.md)). As políticas classificam ações como leitura,
escrita ou destrutivas; um deny vence um allow; ações destrutivas sempre perguntam; toda resposta,
seja de um humano, de uma regra ou de uma política, vai para um log de auditoria só de acréscimo
(append-only) que o usuário pode ler. A "parada dura" de um orçamento só impede novas sessões que o
app iniciaria; ela nunca bloqueia nem mata um agente que o app apenas observa.

<a id="consequences"></a>

## Consequências

- A regra 2 do `CLAUDE.md` e o teste do core que a garante mudam no mesmo pull request que aceita
  este registro.
- O log de auditoria (passo P8) precisa existir antes.
