# 0017. Uma decisão assinada num celular pareado vale como um clique
<!-- source: aba6d1bc201e -->

**Status:** Aceita, 2026-10-10. Emenda a [0008](0008-one-core-many-surfaces.md) (a ilha deixa de
ser o único lugar dos cards) e a prioridade só-Linux do `CLAUDE.md`, apenas para o Android.

<a id="context"></a>

## Contexto

Um card espera por um humano por até 108 s. Longe da mesa, o usuário só pode deixá-lo ir para o
terminal. Um celular poderia respondê-lo, mas a [0004](0004-a-human-answers-permissions.md) e a
[0014](0014-consent-before-autonomy.md) aceitam só um clique no desktop, uma regra ou uma política
que o usuário fez, e a [0008](0008-one-core-many-surfaces.md) mantém todo card na ilha. Uma
mensagem que apenas diz vir do celular do usuário não é o clique de um humano: qualquer coisa na
rede poderia enviá-la.

<a id="decision"></a>

## Decisão

- Um celular Android pareado pode responder a um card. A resposta vale como clique só quando é
  assinada por uma chave guardada no Android Keystore com `BIOMETRIC_STRONG`, desbloqueada pela
  biometria do usuário para aquele único uso.
- O pareamento é um QR mostrado no desktop: a chave pública do celular, a URL do relay e um
  segredo de uso único que vive 2 minutos. O usuário pode revogar um celular no Settings.
- Toda resposta passa pelo ledger do `core`: presa a um hash do pedido inteiro, usada uma vez,
  recusada depois de `min(120 s, limits::SERVER_DECISION_TIMEOUT)`, com um contador por aparelho.
- Negar e dispensar não pedem biometria; aprovar, responder, passar adiante, aceitar uma tarefa e
  uma ação de serviço pedem. Enviar mensagem ou mover dinheiro não existe no protocolo.
- O desktop continua a fonte da verdade e o único que age; o celular só decide. Um relay, quando
  usado, vê só envelopes cifrados. Ligar o celular segue a
  regra das requisições em nome do usuário (0019, `docs/dev/plan-zeca.md` G9).
- Kotlin é permitido só para a casca Android; a lógica é o núcleo Rust via UniFFI.
- O Android é a única exceção a "Linux primeiro e só Linux". iOS e macOS vêm depois, no mesmo
  protocolo.

<a id="consequences"></a>

## Consequências

- A decisão de design D1 (`docs/dev/road-to-0.2.md`) passa a ser "a ilha e o celular": a ilha
  continua o único lugar dos cards no desktop.
- O teste da regra 2 ganha um caso para cada decisão do celular, válida e forjada (sem assinatura,
  aparelho revogado, oferta expirada ou reusada, binding divergente).
- Um celular perdido sem a biometria do dono não aprova nada; com ela, aprova até ser revogado.
