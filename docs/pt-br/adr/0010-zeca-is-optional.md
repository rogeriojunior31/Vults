# 0010. O Zeca é opcional e fica separado do bando
<!-- source: 347bc8416f69 -->

**Status:** Aceita, 2026-10-04.

<a id="context"></a>

## Contexto

O app faz dois trabalhos: mostra o que os agentes do usuário estão fazendo (o bando) e oferece um
companheiro que conversa, escuta e talvez, mais tarde, aja (o Zeca). Alguns usuários querem só o
primeiro. O segundo vai ganhar mais poder com o tempo, e esse poder não pode vazar para o primeiro.

<a id="decision"></a>

## Decisão

- Tudo o que diz respeito ao bando funciona com o Zeca desligado: sessões, cards, notificações,
  conectores.
- Com o Zeca desligado: sem chat, sem microfone, sem atalho para falar, sem *Chat…* na bandeja, e a
  ilha tem seu próprio visual ocioso.
- Quando o Zeca virar um agente, ele recebe um handle que não consegue construir `Decide` nem
  `DecideAlways`, por tipo. Os comandos e edições dele perguntam ao usuário como qualquer chat faz
  hoje.

<a id="consequences"></a>

## Consequências

- A ilha precisa de um visual ocioso sem o Zeca.
- O crescimento do companheiro nunca muda as garantias da
  [0004](0004-a-human-answers-permissions.md).
