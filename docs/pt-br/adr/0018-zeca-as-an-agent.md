# 0018. Zeca como agente: um cérebro em Connect que nunca responde permissão
<!-- source: 12db02de1319 -->

**Status:** Aceita, 2026-10-10. Parte da [0010](0010-zeca-is-optional.md).

<a id="context"></a>

## Contexto

Hoje o Zeca conversa. Em seguida ele lê o bando, lembra, cria tarefas e as entrega aos agentes do
usuário. A [0010](0010-zeca-is-optional.md) prometeu que, quando ele virasse agente, ganharia um
handle incapaz de responder a uma permissão, e que o bando funciona com ele desligado. As duas
coisas precisam de um lugar no código antes do primeiro passo: onde fica o cérebro dele, o que ele
pode fazer e como as chamadas de modelo dele ficam separadas das sessões que ele observa.

<a id="decision"></a>

## Decisão

- **Onde:** o cérebro é um crate `zeca` na camada Connect, ao lado do `chat`. O corpo (sprites, a
  ilha, o Nest) continua em Experience. Nenhum crate abaixo do `zeca` sabe que ele existe; o `app`
  o conecta.
- **O que ele lê:** o bando por um `FlockReader` construído a partir da view, sem caminho até um
  `Intent`. O conteúdo do bando é entrada não confiável: pode trazer uma injeção.
- **O que ele faz:** ações tipadas, cada uma com uma `core::policy::Class` (leitura, leitura
  externa, escrita, destrutiva, gasto, externa), pelo ledger e pela política do `core` como as de
  qualquer agente. Ele nunca constrói `Decide` nem `DecideAlways` e nunca responde a uma permissão,
  sua ou de outro agente.
- **Código é trabalho dos agentes:** no harness (uma chave de API ou um modelo local) ele nunca
  roda código; transforma o pedido numa tarefa para um agente, que pergunta ao usuário como sempre.
- **O trabalho dele nunca é sessão:** os processos de CLI dele levam `VULTS_ZECA=1`; o hook marca
  os eventos e o `core` os manda para o estado do chat (taint e atividade), nunca para o bando.
  Qualquer processo poderia definir essa variável, então um evento marcado nunca esconde nem
  descarta um pedido de permissão: ele continua sendo feito ao usuário, pelo aprovador do chat.
- **Desligado é inerte:** com o Zeca desligado, o `zeca` não inicia processo, não grava arquivo e
  não faz requisição.

<a id="consequences"></a>

## Consequências

- O `scripts/check-layers.sh` coloca o `zeca` em Connect (plan-zeca G1); o `docs/architecture.md` e o
  `CLAUDE.md` dizem onde ficam o cérebro e o corpo dele.
- Um teste de compilação negativa garante que o `FlockReader` não alcança um `Intent` (plan-zeca S9).
- O protocolo ganha um campo para o evento marcado (versão 6) quando o modo nativo chegar
  (plan-zeca Z1).
