# 0024. Uma superfície por vez: a ilha sai para o Nest
<!-- source: 3a92bdc0a573 -->

**Status:** Aceita, 2026-10-12. Emenda a [0021](0021-four-sizes-one-nest.md).

<a id="context"></a>
## Contexto

O protótipo Living Flock desenha Mini, Quick, Room e Full como uma superfície só, que cresce a
partir do topo da tela: a pílula vira a ilha, a ilha vira o painel. O app os construiu como duas: a
ilha, uma camada no topo, e o Nest, uma janela (0021, 0023). Com o Nest aberto a ilha continuava no
topo, a pílula e as aves dela ao lado da janela que já mostra o bando inteiro. O veredito do
usuário: no tamanho maior, os urubus lá em cima não são necessários.

<a id="decision"></a>
## Decisão

- Enquanto o Nest está aberto e à vista (Room ou Full), a ilha sai: escondida, com pílula e céu,
  sem região que pegue o mouse e sem nada que a acorde.
- Ela volta quando o Nest fecha, volta para Quick ou Mini, ou é minimizado. O **Back to the island**
  do rodapé é o Quick: fecha o Nest e abre a ilha.
- Um card ainda abre a ilha com o Nest aberto: só a ilha responde um card (D1), e um agente nunca
  espera sem ser visto (regra 1). Respondido, a ilha sai de novo.
- O Nest continua uma janela normal (move, redimensiona, aparece no Alt+Tab), e as Configurações
  continuam um webview próprio nela (0023). A ilha e o Nest continuam duas superfícies por baixo; só
  uma fica na tela.

<a id="consequences"></a>
## Consequências

- Uma coisa na tela por vez, como o protótipo desenha.
- No Wayland o compositor posiciona a janela do Nest; ela não pode ser presa no lugar da ilha.
- Os presets (0009) mantêm o sentido; o Nest aberto se comporta, para a ilha, como o repouso do
  preset Painel enquanto durar.
