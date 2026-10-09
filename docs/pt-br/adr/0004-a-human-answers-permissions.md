# 0004. Uma permissão é respondida por um humano, ou por uma regra exata que um humano criou
<!-- source: 6d8fe713f2a5 -->

**Status:** Aceita, 2026-10-04. Será substituída, no que diz respeito à autonomia, pela
[0014](0014-consent-before-autonomy.md) quando esta for aceita.

<a id="context"></a>

## Contexto

Permitir um comando é a única coisa que o app faz que pode causar dano. Isso nunca pode vir de um
timer, de um palpite, de um modelo ou de outro programa.

<a id="decision"></a>

## Decisão

O `core` transforma uma permissão pendente em resposta a partir de apenas três entradas:

- `Intent::Decide`: um clique em Allow ou Deny (ou Ctrl+Alt+Y/N com o card visível).
- `Intent::DecideAlways`: um clique em Always, que permite este pedido e salva uma regra.
- Uma regra salva que bate **exatamente**: o mesmo agente, pasta, ferramenta e alvo. `cargo test`
  não permite `cargo test && rm -rf build`.

Um teste alimenta todas as outras entradas e verifica que nenhuma delas responde. Todo novo
`Intent` entra nesse teste.

<a id="consequences"></a>

## Consequências

- Nenhuma notificação, item da bandeja, paleta ou assistente pode permitir nada; eles só podem
  trazer o card à tona ([0008](0008-one-core-many-surfaces.md)).
- Regras mais amplas (padrões, políticas) precisam antes de uma nova regra de consentimento
  ([0014](0014-consent-before-autonomy.md)).
