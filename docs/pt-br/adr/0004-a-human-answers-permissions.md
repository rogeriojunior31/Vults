# 0004. Uma permissão é respondida por um humano, ou por uma regra exata que um humano criou
<!-- source: 8207edf1cc5c -->

**Status:** Aceita, 2026-10-04. Emendada pela [0014](0014-consent-before-autonomy.md) (uma política
que o usuário aceitou pode responder), pela [0017](0017-a-signed-phone-decision-is-a-click.md)
(também a resposta assinada de um celular pareado), pela [0025](0025-destructive-calls-always-ask.md)
(nenhuma regra ou política responde a uma chamada destrutiva) e pela
[0026](0026-the-chats-access-is-the-users-choice.md) (no chat do Zeca, também o que o usuário
escolheu em Settings).

<a id="context"></a>

## Contexto

Permitir um comando é a única coisa que o app faz que pode causar dano. Isso nunca pode vir de um
timer, de um palpite, de um modelo ou de outro programa.

<a id="decision"></a>

## Decisão

O `core` transforma uma permissão pendente em resposta a partir apenas destas entradas:

- `Intent::Decide`: um clique em Allow ou Deny (ou Ctrl+Alt+Y/N com o card visível).
- `Intent::DecideAlways`: um clique em Always, que permite este pedido e salva uma regra.
- Uma regra salva que bate **exatamente**: o mesmo agente, pasta, ferramenta e alvo. `cargo test`
  não permite `cargo test && rm -rf build`.

As emendas do status acrescentam suas próprias entradas, cada uma uma escolha de um humano; nada
mais responde. Um teste alimenta todas as outras entradas e verifica que nenhuma delas responde. Todo novo
`Intent` entra nesse teste.

<a id="consequences"></a>

## Consequências

- Nenhuma notificação, item da bandeja, paleta ou assistente pode permitir nada; eles só podem
  trazer o card à tona ([0008](0008-one-core-many-surfaces.md)).
- Regras mais amplas (padrões, políticas) precisam antes de uma nova regra de consentimento
  ([0014](0014-consent-before-autonomy.md)).
