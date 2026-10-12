# 0004. Quem responde a uma permissão
<!-- source: 3956ca58fb6b -->

**Status:** Aceita, 2026-10-04. Consolidada em 2026-10-12 a partir da 0014, da 0017, da 0025 e da
0026.

<a id="context"></a>

## Contexto

Permitir um comando é a única coisa que o app faz que pode causar dano. Isso nunca pode vir de um
timer, de um palpite, de um modelo ou de outro programa, nem de uma mensagem que apenas diz ser o
usuário.

<a id="decision"></a>

## Decisão

O `core` transforma uma permissão pendente em resposta a partir apenas destas entradas, cada uma
uma escolha de um humano:

- **Um clique**: `Intent::Decide` (Allow ou Deny, ou Ctrl+Alt+Y/N com o card visível), ou
  `Intent::DecideAlways` (Always: permite este pedido e salva uma regra).
- **Uma regra exata** que o usuário salvou: o mesmo agente, pasta, ferramenta e alvo. `cargo test`
  não permite `cargo test && rm -rf build`.
- **Uma política** que o usuário escreveu, viu como diff e aceitou com um clique, como uma config de
  agente ([0005](0005-agent-configs-backup-diff-click.md)). As políticas classificam as ações como
  leitura, leitura externa, escrita, destrutiva, gasto ou externa (`core::policy`), e um deny vence
  um allow.
- **A resposta de um celular Android pareado**, que vale como clique só quando passa por estas
  verificações:
  - O pareamento é um QR mostrado no desktop: a chave pública do celular, a URL do relay e um
    segredo de uso único que vive 2 minutos. O usuário pode revogar um celular no Settings.
  - Toda resposta é assinada pela chave do aparelho pareado. Aprovar, responder, passar adiante,
    aceitar uma tarefa e uma ação de serviço também pedem uma chave guardada no Android Keystore
    com `BIOMETRIC_STRONG`, desbloqueada pela biometria do usuário para aquele único uso; negar e
    dispensar não pedem. Enviar mensagem ou mover dinheiro não existe no protocolo.
  - Toda resposta passa pelo ledger do `core`: presa a um hash do pedido inteiro, usada uma vez,
    recusada depois de `min(120 s, limits::SERVER_DECISION_TIMEOUT)`, com um contador por aparelho.
  - O desktop continua a fonte da verdade e o único que age; o celular só decide. Um relay vê só
    envelopes cifrados; ligar o celular é uma requisição em nome do usuário
    ([0019](0019-requests-on-the-users-behalf.md)). No desktop, a ilha continua o único lugar dos
    cards ([0021](0021-four-sizes-one-nest.md)).
  - Kotlin é permitido só para a casca Android; a lógica dela é o núcleo Rust via UniFFI. O Android
    é a única exceção a "Linux primeiro e só Linux"; iOS e macOS vêm depois, no mesmo protocolo.
- **Só no chat do Zeca, o que o usuário escolheu** em Settings → Chat & voice → *Who answers Zeca's
  cards*:
  - **Asks first** (o padrão): um card para cada chamada que nenhuma regra *Always* cobre (o
    *Always in this chat* é um comando exato, até a conversa acabar).
  - **Your policies**: as políticas aceitas respondem as chamadas do chat exatamente como respondem
    as do Claude Code na ilha: um deny nega, um allow permite (só chamadas do Claude Code, dentro da
    pasta da conversa), o resto pergunta.
  - **Full access**: toda chamada roda sem card, exceto que o que uma política nega é negado, o que
    uma política manda perguntar ainda pergunta, e depois que a conversa leu conteúdo de fora (uma
    página web, um fetch, uma ferramenta MCP) os cards dela voltam a perguntar pelo resto da
    conversa.
  - Mais acesso só se dá no Settings; o seletor do chat mostra a escolha e só pode devolver parte
    dela. O chat diz o que rodou com a palavra de quem, e a CLI dele continua rodando com
    `--permission-mode default`, para que as respostas venham do Vults. As sessões do bando não
    mudam.

Acima de tudo isso:

- **Uma chamada destrutiva sempre pergunta**: nenhuma política, nenhuma regra *Always* (inclusive
  uma salva antes desta regra) e nenhuma configuração do chat a responde. O card dela não oferece
  **Always allow**; se o core receber um mesmo assim, ele vale como um Allow simples para aquele
  card, auditado como tal, e nenhuma regra é salva.
- **Toda resposta** (de um humano, de uma regra, de uma política com o consentimento dela, do
  celular, o `full-access` do chat) vai para um log de auditoria só de acréscimo que o usuário pode
  ler.
- O "hard stop" de um orçamento só impede novas sessões que o app iniciaria; ele nunca bloqueia nem
  mata um agente que o app só observa.
- Nada mais responde. Um teste alimenta todas as outras entradas e verifica que nenhuma responde;
  todo novo `Intent` entra nele, com um caso para cada decisão do celular, válida e forjada (sem
  assinatura, aparelho revogado, oferta expirada ou reusada, binding divergente).

<a id="consequences"></a>

## Consequências

- Nenhuma notificação, item da bandeja, paleta, modelo, voz ou assistente pode permitir nada; eles
  só podem trazer o card à tona.
- Um celular perdido sem a biometria do dono não aprova nada; com ela, aprova até ser revogado.
- O acesso total confia no modelo dentro da pasta do chat: uma edição errada ou um `git push` roda.
  A configuração diz isso, o chip no chat fica vermelho, e as linhas de deny de uma política
  mantêm ações fora de alcance.
- Um teste fixa cada metade da regra destrutiva: Always num card destrutivo não salva regra, e uma
  regra antiga não responde a um.

<a id="history"></a>

## Histórico

- [0014](0014-consent-before-autonomy.md), 2026-10-10: políticas aceitas podem responder; um deny
  vence um allow, o destrutivo sempre pergunta, o log de auditoria, o hard stop do orçamento.
- [0017](0017-a-signed-phone-decision-is-a-click.md), 2026-10-10: a resposta assinada de um celular
  pareado vale como clique; Kotlin e o Android como a única exceção.
- [0025](0025-destructive-calls-always-ask.md), 2026-10-12: nenhuma regra *Always* responde a uma
  chamada destrutiva.
- [0026](0026-the-chats-access-is-the-users-choice.md), 2026-10-12: quem responde os cards do chat
  do Zeca é escolha do usuário.
