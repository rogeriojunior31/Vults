# 0030. Um card é respondido onde você está
<!-- source: 01faf724053b -->

**Status:** Aceita, 2026-10-12. Substitui a ilha como o único lugar que responde cards na
[0008](0008-one-core-many-surfaces.md), na [0021](0021-four-sizes-one-nest.md) e na
[0024](0024-one-surface-at-a-time.md); a [0004](0004-a-human-answers-permissions.md) não muda.

<a id="context"></a>
## Contexto

A 0008 fez da ilha a única janela que responde um card, para manter a
[0004](0004-a-human-answers-permissions.md) simples de verificar. O Nest (0021) veio depois como uma
janela própria, e a 0024 tirou a ilha da frente enquanto ele está aberto. As duas mantiveram a
regra: o Nest mostrava um card e oferecia **Answer on the island** ou **Review on the island →**, e
um card trazia a ilha por cima do Nest. No uso isso são duas superfícies para um clique: o usuário
está no Chat do Nest, o Zeca pede para rodar um comando, e a resposta tem de ser dada em outro
lugar, por cima da janela em que ele está digitando. O mesmo vale para o card de um agente enquanto
ele acompanha o bando no Nest.

<a id="decision"></a>
## Decisão

- **Um card é respondido onde o usuário está.** O Nest responde os cards do bando (Allow, Deny,
  Always allow, as escolhas de uma pergunta, Reply in the terminal) no card do Flock da sessão da
  frente, e os cards do Chat dele no Chat, com os mesmos botões e regras da ilha: uma ação
  destrutiva mantém o Deny como o botão em destaque e não oferece Always (ADR 0025).
- **O usuário está no Nest** quando a janela dele tem o foco e mostra o Nest, não as Configurações.
  Aí um card não abre a ilha; a faixa de quem precisa de você e o som do Nest dizem que ele espera,
  e as teclas globais de Allow / Deny respondem o card na tela do Nest (o do Chat na aba dele, o da
  frente no Flock). Em qualquer outro lugar, um card abre a ilha como antes, para que um agente
  nunca espere sem ser visto (regra 1). Entrar no Nest recolhe uma ilha que segura um card; sair
  dele traz a ilha de volta.
- **Só a ilha e o Nest respondem.** O app confere o webview por trás de cada resposta: as
  Configurações, o widget e a paleta nunca respondem. Toda resposta continua sendo um clique (ou a
  tecla dele) do usuário, e é auditada como antes.

<a id="consequences"></a>
## Consequências

- Um clique onde o usuário já está. **Answer on the island** e **Review on the island →** saem; o
  *Review its card* da paleta abre o card no Flock do Nest, ou a ilha.
- O terminal do agente, enquanto um card espera, diz *Waiting for your answer in Vults*: não mais
  só a ilha.
- A 0004 é tão simples de verificar quanto antes: dois rótulos de webview em vez de um, ambos
  páginas do próprio app.
- O Flock do Nest ganha os corpos de permissão e de pergunta da ilha; os testes visuais cobrem os
  dois.
