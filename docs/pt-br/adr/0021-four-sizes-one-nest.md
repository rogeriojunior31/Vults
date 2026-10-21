# 0021. Superfícies: um núcleo, quatro tamanhos, um card respondido onde você está
<!-- source: e1a0b1efd036 -->

**Status:** Aceita, 2026-10-11. Consolidada em 2026-10-12 a partir da 0008, da 0009, da 0022, da
0023, da 0024 e da 0030. Alterada em 2026-10-16 pela [0031](0031-tasks-leave-vults.md): o Nest não tem aba Tasks.
Alterada em 2026-10-17 para a N3 V0.3: a área de Ditado e seu contrato de apresentação de estado.
Essas adições chegam nas fases de seus recursos; a decisão não afirma entrega da interface em
produção nem validação nativa.

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
- **Caixa de ações do Quick**: Flock, Chat e Now, com Settings na engrenagem. O Now apresenta um
  único `ViewModel.inbox` do core/app, com cards adiados, problemas de sessões, itens dos conectores
  que pedem uma ação, solicitações do Chat e propostas de memória. Uma linha abre o responsável
  existente pela interação; nenhuma operação da caixa responde a uma permissão ou executa a ação
  do destino. Activity e Connections mantêm suas telas completas no Nest.
- **Quatro tamanhos**: **Mini** é a pílula (340 x 38, o bando e nenhum texto em repouso: novidades
  chegam a ela só como corpo, cor ou movimento; o ditado ativo tem o estado transitório abaixo).
  **Quick** é a ilha dentro da superfície de camada fixa
  de 880 x 640 pixels lógicos: normalmente com 440 CSS px de largura e 492 CSS px no Chat,
  antes do zoom de 1,14 da interface (502 e 561 pixels lógicos, respectivamente).
  A altura fica dentro de 556 CSS px (634 após o zoom), reduzida quando a área disponível
  é menor. Mini e Quick podem compartilhar uma largura escolhida pelo usuário de 560, 680 ou
  760 pixels lógicos, salva como estado local de apresentação. Padrão restaura as larguras
  originais por tamanho e aba e a altura do Quick; Room e Full não mudam. O Mini mantém 38 pixels
  lógicos de altura em qualquer largura. Uma largura escolhida limita o Quick a 600 pixels lógicos,
  incluindo o estado de ditado na ilha. Geral > Aparência e Quick > Geral editam a mesma preferência
  de apresentação; notificações do armazenamento compartilhado e retorno do foco mantêm os dois
  controles atualizados. As dimensões escolhidas são limitadas tanto pela área da página quanto pelo
  espaço disponível na tela, com o zoom do Quick aplicado uma única vez. A seleção nunca redimensiona a superfície de camada mapeada. Só o retângulo visível
  medido recebe entrada. O Mini usa um botão de lista apenas com ícone para as sessões que não
  cabem, com contagem localizada no nome acessível. Reserva um alvo de interação separado antes
  de posicionar as aves. Abre o Bando do Nest pela transição de superfície existente; nunca decide,
  libera nem altera uma solicitação. **Room** é a janela do Nest, redimensionável a partir de 480 x 360 px, sem máximo, que na
  primeira vez abre grande dentro da tela. **Full** é o Nest maximizado ou em tela cheia; Esc volta
  ao tamanho anterior. A sobreposição de layer-shell nunca passa do seu tamanho fixo.
- **As superfícies em repouso são superfícies de camada** mapeadas uma vez num tamanho fixo, cada
  uma com a sua região de entrada; o widget de canto fica num canto que o usuário escolhe e não é uma janela para arrastar. Em
  repouso vivem a ilha e a webview passiva de ditado no Linux, mais o widget se escolhido; o Nest
  é criado quando aberto. A superfície do ditado fica transparente quando ociosa. A
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
- **O Nest** tem Flock, Chat, Connections e Activity (Week com *By project*, History,
  Cost, Traces, Audit), com Dictation como quinta aba quando a N3 a entregar. Memory fica num
  painel lateral do Chat, com as Configurações atrás de uma
  engrenagem e a tabela de capacidades dos agentes em Configurações › Agents. Trocar de tamanho
  mantém a seleção: a sessão é o `focus` do core; a aba, os filtros e a conexão escolhida são
  aparência que o app entrega ao Nest. O Nest abre com o Zeca desligado, sem a aba Chat e sem *Ask
  Zeca* (D7). Activity mantém os números de gastos, incluindo registros históricos `task:N`;
  as Configurações mantêm o valor do orçamento. `/continue` continua no Chat; ele não traz de
  volta uma superfície de Tasks. Stop só fica disponível para uma execução que o Vults iniciou
  pelo driver suportado (D8), nunca só porque ele observa uma sessão do bando.
- **Ditado tem um responsável por cada tipo de conteúdo.** Sua área no Nest contém Histórico,
  Comandos de voz, Conversar com Zeca, Reuniões, Dicionário e Trechos prontos conforme essas
  fases são entregues. A aba Ditado também apresenta sua configuração em páginas separadas de
  Voz, Atalhos de teclado, Configurações dos comandos, Vocabulário, Estilos, Colagem, Privacidade
  e Diagnóstico, com a mesma navegação das páginas de trabalho. Voz reúne o modelo de transcrição
  e idioma falado compartilhados; Chat e voz oferece um link. Estatísticas abre **Activity >
  Dictation**, responsável pelos números agregados. A configuração continua na webview de
  Configurações, com as verificações de rótulo existentes; o Nest não ganha autoridade sobre
  modelos, chaves, orçamentos, permissão de inserção nem configuração dos agentes. Cada opção
  tem um único editor. Transcrições e conteúdo de reuniões ficam na webview do Nest; o Diagnóstico
  mostra medições técnicas, não transcrições nem texto dos agentes. Seleção, filtros, rascunhos e
  uma gravação ativa sobrevivem à troca de tamanho e às visitas às Configurações ou à Activity.
  Com Zeca desligado, o ditado independente continua disponível; Conversar com Zeca segue a
  ativação opcional dele.
- **O feedback de ditado no Linux tem sua cápsula passiva.** Uma superfície fixa centralizada
  embaixo mostra um microfone e bandas medidas durante a escuta, borda âmbar com mãos livres e os
  estados existentes de transcrição/resultado. Usa o tema de produção, nunca recebe teclado ou
  cliques e fica transparente quando ociosa. Uma legenda opcional acima mostra texto ouvido ou
  resultado e mantém a expiração existente de dois segundos após o resultado. Não há botões nem
  um responsável separado por áudio. O feedback de ilha/Nest fica oculto enquanto essa superfície
  existe; uma falha de criação mantém o feedback interno. É uma exceção passiva de status à regra
  das superfícies principais ilha/Nest.
- **Cards sempre têm prioridade sobre o ditado.** Cards pendentes, bloqueio de tela e presença
  Paused ocultam a cápsula; resultados expirados não voltam depois. O feedback nunca cobre,
  responde ou atrasa um card, concede inserção nem rouba foco. Cards mantêm seu encaminhamento
  existente entre ilha/Nest. A superfície fixa continua mapeada com região de entrada vazia,
  mesmo quando visível.
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
  pedida. A configuração do Ditado usa essa mesma webview privilegiada e mantém a aba Ditado
  selecionada.
- **Só a webview das Configurações tem os poderes**: escrever a config de um agente
  ([0005](0005-agent-configs-backup-diff-click.md)), salvar ou apagar uma chave, criar ou retirar
  uma política, parear, revogar ou alcançar um celular ([0004](0004-a-human-answers-permissions.md)),
  e ligar uma requisição em nome do usuário ([0019](0019-requests-on-the-users-behalf.md)). Os
  comandos conferem o label da webview que chamou, nunca o da janela (`settings_page`,
  `policies::from_settings`). As duas webviews dividem o armazenamento da origem, que as Configurações nunca leem. As Configurações não mostram texto de nenhum agente: a moldura delas
  tem nomes e contagens das abas, a troca de tamanho, som, tela cheia, e um rodapé que liga e
  desliga o Zeca e traz à frente o terminal da sessão da frente (ela guarda só quem é essa sessão).
  As páginas são General, Agents, Chat & voice, Approvals, Projects, Activity, Connectors, Phone,
  Flock & Zeca e About. A configuração do Ditado na N3 usa essa webview sob sua própria aba,
  fora da lista global de Configurações.

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
- As verificações de interface da N3 devem exercitar cada estado de ditado junto à chegada de
  uma permissão ou pergunta, com Zeca ligado e desligado, Black e Light e movimento reduzido.
  A validação nativa cobre geometria, foco e regiões de entrada no Mini/Quick, além do Room em
  480 x 360 e tamanhos maiores, no X11 e no Plasma Wayland. A revisão visual P12 do protótipo e
  os aceites posteriores no app em execução continuam separados.

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

- 2026-10-17: N2.2 adiciona a caixa Now do Quick, revisão pelo pedido exato e links explícitos aos
  detalhes existentes do Nest; os cards mantêm sua autoridade, registro e prazo originais.
- 2026-10-17: a N3 V0.3 adiciona a quinta aba do Nest por fases, a responsabilidade das
  Configurações e da Activity, o estado transitório do ditado e a prioridade dos cards. Segue a
  decisão revisada de R6 por Configurações centrais e a remoção de Tasks; não introduz sexta aba
  nem configurações privilegiadas próprias da área.

- 2026-10-18: reunir a configuração do ditado na aba Ditado a pedido do mantenedor, com um
  editor por opção e navegação compartilhada. Mover os controles de transcrição compartilhados
  para lá e oferecer um link no Chat. Manter a configuração na webview de Configurações e as
  páginas com transcrições no Nest; a autoridade sobre aprovações, gravação e comandos privilegiados
  continua igual.

- 2026-10-19: mover o status de ditado no Linux para uma cápsula passiva inferior, com feedback
  interno como alternativa na falha de criação; preservar prioridade de cards/bloqueio e foco capturado.

- 2026-10-21: UX7 adiciona uma preferência compartilhada de largura para Mini/Quick e uma
  sobreposição fixa de 880 x 640 reservada na inicialização. Mantém as larguras padrão originais,
  a altura do Mini, o comportamento de Room/Full e a autoridade; a seleção muda apenas a
  geometria do conteúdo e seu retângulo de entrada medido de forma síncrona.

- 2026-10-21: UX7.2 aplica as alturas de referência de 56/600 pixels lógicos apenas às larguras
  escolhidas. Preserva os padrões originais e a escala dos sprites; limita toda a caixa do Quick,
  incluindo o estado de ditado na ilha, antes da medição síncrona do retângulo de entrada.

- 2026-10-21: UX7.3 reserva uma ação no Mini para as sessões além dos poleiros disponíveis.
  Mantém a pílula em repouso sem texto visível, com alvo separado de teclado e a navegação
  existente para o Bando do Nest. Os cartões mantêm a prioridade e o responsável original pelas ações.

- 2026-10-21: o retorno do mantenedor restaura a altura de 38 px do Mini para qualquer largura,
  evitando altura extra da barra em repouso sobre as abas dos aplicativos. Expõe a mesma preferência
  de layout em Geral e no Quick, com armazenamento compartilhado, sem nova configuração
  privilegiada nem autoridade sobre permissões.

- 2026-10-21: UX8 começa com Perfis de agentes dentro do Bando, mantendo as cinco abas principais. O Nest lê o catálogo integrado e preserva o destino limitado `flock-profiles`. O catálogo independe do Zeca e nunca controla permissões nem grava configurações de agentes.
- 2026-10-21: Os perfis de agentes no Bando podem criar cópias locais e editar seu nome/instruções. Somente o Nest controla esses salvamentos; os rascunhos sobrevivem à navegação na janela e são mantidos após falhas ao salvar. Não há nova aba principal, escrita de configurações de agentes nem responsável por permissões.
- 2026-10-21: O editor de perfis adiciona seletores nativos de espécie/acessório e uma prévia estática do quadro completo. Edições de aparência usam o mesmo Salvar/Cancelar local e limite de revisão; não modificam configurações do Zeca nem sessões observadas.
- 2026-10-21: Os registros de conversas vinculadas a perfis mantêm seu próprio snapshot imutável após edições de perfil, renomeação, reinício e reabertura. O Chat atual do Zeca não expõe esses registros nem os reinterpreta como conversas do companheiro; nenhum botão de Chat do perfil ou seletor de destinatário é habilitado nesta etapa de armazenamento.
- 2026-10-21: O primeiro runtime especializado é exclusivo do Claude Code, com briefing fixo e sem contexto automático ou MCP do companheiro. Nenhum seletor de destinatário, ação de Chat do perfil ou reabertura de conversa especializada é habilitado nesta etapa exclusiva de runtime; o Nest e o Quick existentes continuam com o responsável atual pela conversa.
- 2026-10-21: A restauração nativa de conversas de perfil preserva o modelo, esforço, sessão e snapshot do perfil salvos, em vez das edições atuais do perfil ou configurações do companheiro. A verificação anterior ao turno rejeita destinatários e origens incompatíveis e substituições não suportadas por voz/menções/origem. A interface atual do Chat ainda recusa registros de perfil até poder apresentar e preservar esses responsáveis entre superfícies.
