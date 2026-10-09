# 0008. Um núcleo, várias superfícies na área de trabalho; a ilha é a única que exibe cards
<!-- source: eb7ca75a0304 -->

**Status:** Aceita, 2026-10-04.

<a id="context"></a>

## Contexto

A ilha no topo da tela é a única visão hoje. Os usuários querem outros níveis de presença: um
ícone na bandeja, um widget no canto, notificações, uma paleta e, mais tarde, um painel lateral e
um app completo. Feita de forma ingênua, cada uma reimplementaria quem precisa de atenção, qual
sessão está na frente e como um card terminou. Parte dessa lógica hoje vive só no renderizador da
ilha.

<a id="decision"></a>

## Decisão

- **O significado no core, o visual na superfície.** O `core` diz quem precisa do usuário (um
  nível de atenção ordenado), qual sessão está em foco e como cada card terminou (aqui, no
  terminal, expirado, por uma regra). As superfícies escolhem clipes, ícones, cores e sons.
- **Um único view model para todas as superfícies.** Ele é pequeno; uma projeção por superfície só
  aparece quando um payload cresce (histórico).
- **Toda superfície fala em `Intent`s** pelos mesmos comandos. Nenhuma superfície ganha um caminho
  próprio até o estado.
- **A janela da ilha é a única que exibe cards.** Os cards de permissão e de pergunta, e os atalhos
  Ctrl+Alt+Y/N, vivem ali. No modo painel, a ilha se ancora junto à bandeja em vez do topo, mas é
  a mesma janela. Todas as outras superfícies só podem trazer o card à tona.
- **Novas superfícies são layer surfaces** mapeadas uma vez em tamanho fixo, como a ilha. O widget
  do canto fica fixo em um canto que o usuário escolhe; não é uma janela para arrastar.
- **Uma webview fica ativa em repouso** (a ilha, mais o widget se escolhido). A paleta e o app
  completo são criados quando abertos e fechados ao terminar.

<a id="consequences"></a>

## Consequências

- Levar atenção, foco e desfechos para o `core` vem antes da primeira nova superfície, e precisa
  deixar a ilha exatamente com a mesma cara (testes visuais sem novas baselines).
- O código de layer-shell passa a ser por janela (uma região de entrada por superfície).
- Responder a um card continua em um só lugar, o que mantém a
  [0004](0004-a-human-answers-permissions.md) simples de verificar.
