# 0021. Superfícies: um núcleo, quatro tamanhos, um card respondido onde você está
<!-- source: efac77eb9e4e -->

**Status:** Aceita, 2026-10-11. Consolidada em 2026-10-12 a partir da 0008, da 0009, da 0022, da
0023, da 0024 e da 0030. Alterada em 2026-10-16 pela [0031](0031-tasks-leave-vults.md): o Nest não tem aba Tasks.

<a id="context"></a>

## Contexto

Os usuários querem vários níveis de presença: a ilha, um ícone na bandeja, um widget de canto,
notificações, uma paleta, um app completo. Feita de forma ingênua, cada uma reimplementaria quem
precisa de atenção, qual sessão está na frente e como um card terminou. O protótipo Living Flock do
dono (`design/explorations/08-living-flock.html`) desenha o app como uma identidade em quatro
distâncias, com o bando num céu compartilhado em todos os tamanhos. Algumas superfícies mostram
texto dos agentes, que pode trazer palavras feitas para conduzir quem as lê.

<a id="decision"></a>

## Decisão

- **Significado no core, aparência na superfície.** O `core` diz quem precisa do usuário (um nível
  de atenção ordenado), qual sessão está em foco e como cada card terminou (aqui, no terminal,
  expirado, por uma regra). As superfícies escolhem clipes, ícones, cores e sons, a partir de um
  único view model pequeno (uma projeção por superfície só quando um payload cresce), e falam só em
  `Intent`s, pelos mesmos comandos.
- **Quatro tamanhos**: **Mini** é a pílula (340 x 38, o bando e nenhum texto: novidades chegam a
  ela só como corpo, cor ou movimento). **Quick** é a ilha dentro da superfície de camada fixa
  de 720 x 640 pixels lógicos: normalmente com 440 CSS px de largura e 492 CSS px no Chat,
  antes do zoom de 1,14 da interface (502 e 561 pixels lógicos, respectivamente).
  A altura fica dentro de 556 CSS px (634 após o zoom), reduzida quando a área disponível
  é menor. Só o retângulo visível medido recebe entrada. **Room** é a janela do Nest, redimensionável a partir de 480 x 360 px, sem máximo, que na
  primeira vez abre grande dentro da tela. **Full** é o Nest maximizado ou em tela cheia; Esc volta
  ao tamanho anterior. A sobreposição de layer-shell nunca passa do seu tamanho fixo.
- **As superfícies em repouso são superfícies de camada** mapeadas uma vez num tamanho fixo, cada
  uma com a sua região de entrada; o widget de canto fica num canto que o usuário escolhe e não é uma janela para arrastar. Em
  repouso vive uma webview (a ilha, mais o widget se escolhido); o Nest é criado quando aberto. A
  paleta não é uma superfície: ela abre dentro da página da ilha ou do Nest.
- **Um card é respondido onde o usuário está: na ilha ou no Nest.** A ilha guarda os cards de
  permissão e de pergunta e o Ctrl+Alt+Y/N (no modo painel ela se ancora pela bandeja, a mesma
  janela). O Nest responde os cards do bando (Allow, Deny, Always allow, as escolhas de uma
  pergunta, Reply in the terminal) no card do Flock da sessão da frente, e os cards do Chat dele no
  Chat, com os botões e as regras da ilha: uma ação destrutiva mantém o Deny em destaque e não
  oferece Always ([0004](0004-a-human-answers-permissions.md)). O usuário está no Nest quando a
  janela dele tem o foco e mostra o Nest, não as Configurações; aí um card não abre a ilha, a faixa
  de quem precisa de você e o som do Nest dizem que ele espera, e o Ctrl+Alt+Y/N responde o card na
  tela de lá. Em qualquer outro lugar um card abre a ilha, para que um agente nunca espere sem ser
  visto. Entrar no Nest recolhe uma ilha que segura um card; sair dele traz a ilha de volta. Só
  esses dois respondem: o app confere a webview por trás de cada resposta, e as Configurações, o
  widget e a paleta nunca respondem. Toda resposta continua sendo um clique (ou a tecla dele) do
  usuário, e é auditada. Um celular pareado também pode responder
  ([0004](0004-a-human-answers-permissions.md)).
- **Nenhum modo de presença deixa um card confirmado sem ser visto.** Em *Island*, *Panel* e
  *Quiet*, um card pendente abre a ilha e toca o seu som, ou, no Nest em que o usuário está, aparece na
  faixa de quem precisa de você com o seu som (o *Quiet* só silencia o que acontece em
  repouso). Em *Paused*, o `core` não confirma cards e os libera na hora, então o agente pergunta no
  terminal sem esperar, e os conectores param o polling.
- **O Nest** tem quatro abas, Flock, Chat, Connections e Activity (Week com *By project*, History,
  Cost, Traces, Audit), com Memory num painel lateral do Chat, as Configurações atrás de uma
  engrenagem e a tabela de capacidades dos agentes em Configurações › Agents. Trocar de tamanho
  mantém a seleção: a sessão é o `focus` do core; a aba, os filtros e a conexão escolhida são
  aparência que o app entrega ao Nest. O Nest abre com o Zeca desligado, sem a aba Chat e sem *Ask
  Zeca* (D7). Activity mantém os números de gastos, incluindo registros históricos `task:N`;
  as Configurações mantêm o valor do orçamento. `/continue` continua no Chat; ele não traz de
  volta uma superfície de Tasks. Stop só fica disponível para uma execução que o Vults iniciou
  pelo driver suportado (D8), nunca só porque ele observa uma sessão do bando.
- **Uma superfície por vez** (a ilha e o Nest continuam duas superfícies por baixo). Enquanto o Nest está aberto e à vista (Room ou Full), a ilha fica
  escondida, pílula, céu e tudo, sem nenhuma região que pegue o mouse. Ela volta quando o Nest
  fecha ou volta para Quick ou Mini (ou é minimizado, onde o sistema diz isso); o **Back to the
  island** do rodapé é o Quick. Nada mais a acorda, a não ser um card enquanto o usuário está fora
  do Nest; respondido, ela se recolhe depois do atraso de sempre e volta a ficar escondida.
- **A janela do Nest** (`nest-window`) se move, se redimensiona e aparece no Alt+Tab, sem a barra de
  título do sistema: a moldura arredondada do protótipo, um cabeçalho que a move, bordas que a
  redimensionam. Ela tem duas webviews sob um único cabeçalho, a do Nest (`nest`) e a das
  Configurações (`settings`), uma visível por vez. A engrenagem mostra as Configurações; uma aba
  mostra o Nest; `open_settings(section)` abre a janela nas Configurações, numa página quando
  pedida.
- **Só a webview das Configurações tem os poderes**: escrever a config de um agente
  ([0005](0005-agent-configs-backup-diff-click.md)), salvar ou apagar uma chave, criar ou retirar
  uma política, parear, revogar ou alcançar um celular ([0004](0004-a-human-answers-permissions.md)),
  e ligar uma requisição em nome do usuário ([0019](0019-requests-on-the-users-behalf.md)). Os
  comandos conferem o label da webview que chamou, nunca o da janela (`settings_page`,
  `policies::from_settings`). As duas webviews dividem o armazenamento da origem, que as Configurações nunca leem. As Configurações não mostram texto de nenhum agente: a moldura delas
  tem nomes e contagens das abas, a troca de tamanho, som, tela cheia, e um rodapé que liga e
  desliga o Zeca e traz à frente o terminal da sessão da frente (ela guarda só quem é essa sessão).
  As páginas são General, Agents, Chat & voice, Approvals, Projects, Activity, Connectors, Phone,
  Flock & Zeca, About.

<a id="consequences"></a>

## Consequências

- Um teste confere cada preset: um card confirmado sempre aparece, na ilha ou no Nest em que o
  usuário está, e o *Paused* nunca confirma um. Enquanto um card espera, o terminal do agente diz
  *Waiting for your answer in Vults*; o *Review its card* da paleta o abre no Nest ou na ilha. O Nest aberto vale, para a ilha, como o repouso do preset Panel.
- Uma página futura escolhe a sua webview pelo que mostra: texto de agentes vai na do Nest, poderes
  na das Configurações. Uma superfície que precise de um poder abre as Configurações na página dele.
- Webviews filhas precisam da feature `unstable` do Tauri. No Linux, as webviews de uma janela
  dividem a caixa dela, então são trocadas, nunca sobrepostas, e a escondida fica fora do show-all
  do GTK. Abrir nas Configurações carrega também a página do Nest, escondida e em repouso. Um evento
  que o app manda chega aos listeners de todas as webviews; uma página continua desenhando só o que
  pediu.
- No Wayland, o compositor posiciona a janela do Nest; ela não pode ser presa embaixo do lugar da
  ilha.

<a id="history"></a>

## Histórico

- 0008, 2026-10-04: um núcleo, várias superfícies; a ilha é o
  único lugar dos cards.
- 0009, 2026-10-04: nenhum modo de presença esconde um card
  confirmado.
- 0022, 2026-10-12: só as Configurações têm os poderes (então
  numa janela própria).
- 0023, 2026-10-12: as Configurações viram uma webview da
  janela do Nest; as verificações vão pelo label da webview.
- 0024, 2026-10-12: a ilha sai enquanto o Nest está aberto.
- 0030, 2026-10-12: o Nest também responde um card, enquanto o usuário está nele.

- 2026-10-15: o Chat usa uma composição Quick mais larga dentro da superfície fixa atual.
  A geometria aplica o zoom uma única vez e limita as dimensões à área disponível antes
  da medição. A configuração nativa inicial acompanha os 720 x 640 do runtime; Mini
  e as outras abas do Quick mantêm suas larguras. Larguras pareadas escolhidas pelo
  usuário continuam como trabalho futuro.
- 2026-10-16, [0031](0031-tasks-leave-vults.md): remover Tasks do Nest; preservar D7, gastos e
  orçamento e o limite de D8 para `/continue`.
