# 0009. Nenhum modo de presença deixa um card confirmado sem ser visto
<!-- source: 87d66e06e324 -->

**Status:** Aceita, 2026-10-04.

<a id="context"></a>

## Contexto

As predefinições de presença deixam o usuário escolher quanto o app mostra em repouso: a ilha, só
a bandeja, um modo silencioso ou uma pausa. O hook recebe a confirmação assim que o app enfileira
um card, não quando o card é desenhado, e a partir daí o agente espera até 110 s por um humano. Um
modo que escondesse um card confirmado faria os agentes esperarem por ninguém.

<a id="decision"></a>

## Decisão

- Em *Island*, *Panel* e *Quiet*, um card pendente abre a ilha e toca seu som. *Quiet* só silencia
  o que acontece em repouso (sessões indo e vindo, trabalho concluído).
- Em *Paused*, o `core` não confirma cards e os libera na hora: o agente pergunta no próprio
  terminal, sem espera. Os conectores param de consultar enquanto o app está pausado.
- Nenhum modo esconde um card confirmado.

<a id="consequences"></a>

## Consequências

- Um teste verifica cada predefinição: um card confirmado sempre tem a ilha mostrando-o, e
  *Paused* nunca confirma nenhum.
- Usuários que não querem interrupções pausam o app; aí os agentes se comportam como se ele
  estivesse fechado.
