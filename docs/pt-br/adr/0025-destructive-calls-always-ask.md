# 0025. Uma chamada destrutiva sempre pergunta, mesmo com uma regra Always
<!-- source: 2e7eaa05679f -->

**Status:** Aceita, 2026-10-12. Emenda a [0004](0004-a-human-answers-permissions.md) e a
[0014](0014-consent-before-autonomy.md).

<a id="context"></a>
## Contexto

A [0014](0014-consent-before-autonomy.md) diz que ações destrutivas sempre perguntam, e o card diz o
mesmo: *"Destructive: no policy ever allows it"*. Mas o mesmo card oferecia **Always allow**, e uma
regra *Always* exata ([0004](0004-a-human-answers-permissions.md)) respondia a toda chamada idêntica
seguinte sem card. Um clique em `rm -rf target` liberava esse comando naquela pasta para sempre,
contra o que o card tinha acabado de dizer.

<a id="decision"></a>
## Decisão

- Uma chamada destrutiva sempre pergunta: nenhuma política e nenhuma regra *Always* a responde.
- O card dela não oferece **Always allow**. Se o core receber um assim mesmo (uma UI antiga, a
  paleta), ele vale como um Allow simples para aquele card, auditado como tal, e nenhuma regra é
  salva.
- Uma regra salva antes deste registro também nunca responde a uma chamada destrutiva.

<a id="consequences"></a>
## Consequências

- A regra 2 do `CLAUDE.md` e a página [Segurança](../safety.md) dizem que a exceção vale também
  para as regras.
- Um teste fixa cada metade: Always num card destrutivo não salva regra, e uma regra antiga não
  responde a uma chamada destrutiva.
