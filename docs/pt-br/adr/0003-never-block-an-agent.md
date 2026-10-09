# 0003. O hook nunca bloqueia um agente
<!-- source: 5416e1afec81 -->

**Status:** Aceita, 2026-10-04 (a regra existe desde o início do projeto).

<a id="context"></a>

## Contexto

O hook roda em cada evento de cada agente. Se ele travar ou imprimir a coisa errada, o trabalho do
usuário para, e ele não pediu para a gente estar nesse caminho.

<a id="decision"></a>

## Decisão

Em qualquer falha (app fechado, lento, uma mensagem ruim), o hook sai com código 0 e stdout vazio,
e o agente segue como se não estivéssemos lá. Um pedido de permissão espera a confirmação do app
por no máximo 800 ms; um humano, no máximo pelo prazo de decisão (110 s), e depois o agente
pergunta no próprio terminal.

<a id="consequences"></a>

## Consequências

- O hook usa só `std` e `serde_json`: inicia rápido e tem pouco para quebrar.
- Recursos que precisariam que o agente esperasse por nós (parar, pausar) não podem ser feitos só
  com o hook. Veja [0011](0011-only-actions-that-work.md).
