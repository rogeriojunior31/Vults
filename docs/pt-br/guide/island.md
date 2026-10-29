# A ilha
<!-- source: 1c26b49d42fc -->

A ilha fica pendurada na borda de cima da tela. No KDE Plasma, Hyprland, Sway e outros compositores
com layer-shell, ela fica acima de todas as janelas, como um painel; no GNOME, é uma janela comum
sempre no topo. No X11, ela aparece em todas as áreas de trabalho. Ela só pega o teclado enquanto você
digita no chat, ou depois de **Ctrl+Alt+Space** ([abaixo](#getting-to-a-session)).

Com mais de uma tela, o desktop escolhe uma (em geral, a tela em foco na inicialização). Para fixar a
ilha numa tela, escolha-a em **Settings → General → Screen**. Se essa tela sumir, a ilha passa para
outra e volta quando ela retornar.

No Windows (uma versão de prévia) ainda não há atalhos globais, notificações do desktop, **Open
terminal** nem predefinições na bandeja.

Os canvases das aves recriam o bitmap e os caches de sprites quando a escala da tela muda.
O tamanho do layout e a sessão selecionada permanecem iguais, inclusive com a animação pausada.

<a id="the-flock"></a>

## O bando

Cada sessão de agente é uma ave, um urubu. Um projeto é uma espécie: todas as sessões dele a
compartilham, e ele a mantém toda vez que o app inicia. Uma pequena faixa na base de cada pescoço
mostra o agente: laranja para Claude Code, verde-azulado para Codex, rosa para
OpenCode, índigo para Qwen Code, azul para Copilot CLI, e violeta para qualquer outra ferramenta (Antigravity, Pi, Cursor…).

O **Zeca** não é uma sessão: ele fica no fio quando nada roda, e é com ele que você fala no chat. A
sessão da frente (a que precisa de você, a que você clicou ou, senão, a primeira a chegar) ocupa o
lugar dele. Toda outra sessão é um **vult**.

![O bando do mundo todo: um grifo-do-himalaia na frente, depois um condor-dos-andes, um abutre-preto, um abutre-real-africano e um grifo](../../assets/island-flock.png)

As aves encenam o que as sessões fazem: leem, bicam o fio enquanto editam, levantam voo para a web e
abrem as asas quando uma permissão espera (com uma marca sobre a cabeça; ao amanhecer as aves
em descanso abrem as delas ao sol, sem marca). Sessões novas chegam voando; as que terminam vão embora.
Com movimento reduzido, elas ficam nos poleiros. **Settings → Flock & Zeca** escolhe o conjunto de espécies,
a espécie do Zeca e os visitantes raros. [Animações](../ANIMATIONS.md) lista cada clipe.

<a id="zecas-seasonal-looks"></a>

### Os visuais sazonais do Zeca

O Zeca se veste para a estação: um chapéu de bruxa em outubro, um gorro de Papai Noel em dezembro, um
chapéu de festa no Ano-Novo e orelhas de coelho na Páscoa, pela data do seu computador. **Settings →
Flock → Look** (com o Zeca desenhado ao lado: no visual que você escolhe, com as próprias penas no Auto) mantém um visual o ano todo, ou
nenhum, e acrescenta óculos escuros e catorze trajes
sem estação. **Clique com o botão direito no Zeca** na ilha para escolher ali mesmo: ele veste o que
estiver sob o ponteiro, um clique fica com ele, Escape o deixa como estava. Só o Zeca os usa.

![Os visuais do Zeca: os quatro sazonais, os óculos escuros e os catorze trajes com chapéus, óculos, grills e correntes](../../assets/zeca-looks.png)

<a id="without-zeca"></a>

### Sem o Zeca

O Zeca é opcional. **Settings → Flock & Zeca → Zeca** o desliga; o bando continua funcionando: sessões no
fio, cards de permissão e de pergunta, os atalhos de Allow / Deny, notificações, conectores e as
predefinições. O que vai com ele:

- O chat, sua aba, os arquivos soltos na ilha, o Chat do Nest, o *Chat…* da bandeja e o microfone
  do Chat. O [ditado independente](dictation.md#global-dictation-first-slice) e seu atalho continuam
  disponíveis. Os medidores de uso ficam: eles leem as assinaturas dos seus agentes.
- O olá dele na inicialização.
- O lugar dele no fio: a sessão da frente fica ali como a própria ave, e sem ninguém rodando a pílula
  diz *Nothing running*.

Ligue-o de volta no mesmo lugar; sem reiniciar, nos dois casos.

<a id="compact-open-and-hidden"></a>

## Compacta, aberta e escondida

![A ilha compacta: o Zeca com a sessão da frente, e vults com seus selos](../../assets/island-compact-flock.png)

**Compacta** (o **Mini** do seletor de tamanho), a ilha é uma pequena pílula sem texto, só o bando:
o Zeca à esquerda com a sessão da frente, até oito vults espalhados pelo fio e, embaixo, uma linha
fina por plano com o uso dele, nas cores dos medidores de uso (âmbar a partir de 70%, vermelho a
partir de 90%). Para as sessões, as cores significam o
mesmo em todo lugar: âmbar para uma permissão, ciano para uma pergunta, verde para repouso recente,
vermelho quando falhou. Um vult num desses estados usa um selo dessa cor e com uma forma própria,
para que nenhum estado dependa só da cor: um triângulo para uma permissão, um losango para uma
pergunta, um círculo para repouso recente, um quadrado quando falhou. Enquanto alguém precisa de você a
borda da pílula pulsa em âmbar; quando uma sessão falhou (e ninguém precisa de você) ela fica
vermelha; um ponto ciano no canto diz que um conector tem novidade. Deixe o ponteiro sobre um vult e ele olha para você (um
que está no ar desce); **clique** nele para abrir a ilha na sessão dele, ou **clique com o botão
direito** para as ações rápidas dele. As novidades de conectores esperam na ilha aberta.

Uma sessão ociosa ou concluída com sucesso mantém seu círculo verde por 30 segundos. Depois a marca
some e, de dia, sua ave circula no céu da ilha. Abrir o Quick ou voltar do Nest mantém esse prazo.
Trabalho, uma permissão ou uma pergunta trazem a ave de volta para baixo. Ao amanhecer, ao entardecer
e à noite, com movimento reduzido ou animação pausada, as aves ficam pousadas; aves ociosas também
ficam no fio enquanto toca música. A resposta concluída continua disponível depois que a marca
verde some.

**Clique** em qualquer outro ponto da pílula para abrir a ilha. Ela se recolhe 15 segundos depois que o ponteiro sai (uma
linha fina embaixo faz a contagem; voltar com o ponteiro cancela). O seletor de tamanho no canto
superior direito muda o quanto você vê, mantendo o que você olhava: **Mini** recolhe na hora para a
pílula, **Quick** é a ilha, **Room** abre o [Nest](nest.md) no que a ilha mostra (o bando, o chat,
as suas conexões ou esta semana; as Settings a partir da engrenagem) e o botão de tela cheia ao lado o abre na tela toda. Enquanto um card espera, ele
ocupa o lugar do bando. **Settings → General → Fold the island** define a
espera.

Com **Settings → General → Open on hover** ligado, deixar o ponteiro sobre a pílula a abre sem
clique. Aberta assim, ela se recolhe assim que o ponteiro sai, a não ser que você tenha clicado nela.
Desligado por padrão, e não vale no *Panel*.

Duas coisas a mantêm aberta até você terminar:

- **Um card de permissão.** Ele abre a ilha sozinho e fica até você responder ou escolher
  **Adiar por enquanto**. A solicitação adiada mantém seu selo e prazo; selecione a sessão
  para reabri-la. Nada mais a abre
  sozinho: uma sessão que termina, falha ou pergunta algo toca seu som e ganha seu selo.
- **O chat.** A ilha nunca se recolhe enquanto você digita.

Sem ninguém no fio por um minuto, a ilha **se esconde**. Leve o ponteiro ao meio da borda de cima da
tela para trazê-la de volta.

**Configurações > Geral > Aparência > Largura do Mini e Quick**, também disponível na engrenagem
do Quick em **Geral**, oferece 560, 680 ou 760 pixels lógicos. Os dois controles editam a mesma
preferência; uma alteração em um é refletida no outro. O fio do bando, as posições das aves, os
nomes e os alvos das sessões acompanham juntos a largura medida do conteúdo, mantendo a escala original dos sprites. A escolha é compartilhada pelo Mini e todas as abas do Quick e restaurada ao reiniciar.
**Padrão** restaura a largura original do Mini e as larguras por aba do Quick. Uma tela menor limita
a largura visível sem substituir a escolha salva. Room e Full mantêm seus próprios tamanhos. Se
não for possível salvar, selecione a largura novamente para tentar outra vez; a escolha salva
anteriormente continua ativa.

O Mini mantém 38 pixels lógicos de altura em qualquer largura, deixando a barra em repouso fina.
Com uma largura escolhida, o Quick cresce com seu conteúdo até 600 pixels lógicos, incluindo o zoom
e o estado de ditado na ilha. Telas menores reduzem esses limites. As aves do Mini mantêm a escala
e a animação originais; **Padrão** também restaura a altura anterior do Quick. Conteúdos longos
mantêm os controles de rolagem existentes.

Quando há mais sessões do que o Mini consegue mostrar, um ícone de lista abre **Room > Bando**
para acessar todas. Use Tab até o ícone e pressione Enter ou Espaço; o nome acessível e a dica
incluem o número de sessões adicionais. Ele fica âmbar quando alguma precisa de atenção. O ícone
tem espaço próprio e não cobre uma ave. O Quick mantém seu caminho **+N** para a mesma página do Bando.

Os seletores das configurações rápidas aceitam Seta para cima/baixo e Home/End. Enter ou Espaço
confirma uma escolha; Escape fecha a lista antes de recolher a ilha, e Tab sai sem aplicar uma
escolha. Mudanças de largura e falhas ao salvar mantêm o foco do teclado no seletor e preservam a
posição de leitura. Uma falha ao salvar a largura é anunciada e mantém a escolha salva ativa.

<a id="presence-how-much-it-shows"></a>

## Presença: quanto ela mostra

**Settings → General → Presence**, ou o menu da bandeja (Linux), escolhe uma de quatro
predefinições, na hora.

| Predefinição | Em repouso | Um card que precisa de você |
|---|---|---|
| *Island* (o padrão) | O bando no topo da tela | Abre a ilha, com seu som |
| *Panel* | Nada no topo; o Zeca na bandeja | Abre a ilha junto ao painel, com seu som |
| *Quiet* | Nada, e nenhum som para sessões que terminam ou falham nem para novidades de conectores. A borda de cima ainda traz a ilha de volta | Abre a ilha no topo, com seu som |
| *Paused* | Uma pílula que diz *Paused* | Nenhum: o agente pergunta no terminal, como se o app estivesse fechado |

Nada esconde ou silencia um card pelo qual um agente espera: nem uma predefinição (exceto *Paused*),
nem silenciar ou esconder um projeto, nem o Do not disturb. O card abre a ilha e espera o seu clique.

*Paused* é para nenhuma interrupção: permissões e perguntas, inclusive cards que já esperavam, vão
para o terminal do agente, e suas regras *Always* não respondem nada. Os conectores param de
verificar, e nenhuma notificação do desktop aparece. Escolha outra predefinição para voltar.

<a id="in-the-panel"></a>

### No painel

Nada é desenhado no topo da tela. O Zeca fica na bandeja do painel e mostra o estado do bando:
empoleirado, trabalhando, de asas abertas com uma marca âmbar (algo espera por você), brilhos verdes
(concluído), uma marca vermelha (falhou).

- **Clique** nele e a ilha abre no canto do painel. Ela se recolhe quando o ponteiro sai, com
  **Mini** ou Escape, e quando você abre as Settings.
- **Clique com o botão direito** nele para o menu: *Chat…* (com o Zeca ligado), *Nest…* (também com
  o Zeca desligado), *Set up agents…*, as quatro predefinições (Linux), *Quit*.
- Um card abre a ilha junto ao painel, e o Zeca chama atenção na bandeja até você responder. Allow e
  Deny são sempre cliques no card.
- Novidades de conectores tocam seu som mas não abrem a ilha: clique no Zeca para lê-las.

No KDE Plasma a ilha abre ao lado do painel que tem a bandeja; nos outros, no canto inferior direito.
A bandeja precisa de um painel que mostre StatusNotifierItems (o Plasma mostra; no GNOME, a extensão
AppIndicator, onde um clique esquerdo pode abrir o menu). Sem bandeja, inicie o Vults de novo para
abrir as configurações.

Nas outras predefinições o Zeca também fica na bandeja, com os mesmos estados e menu, e um clique
abre a ilha no topo.

<a id="the-corner-widget"></a>

## O widget de canto

**Settings → General → Corner widget** põe uma pequena janela num canto que você escolhe; fica
desligado até você escolher um. Funciona com todas as predefinições. Mostra até três aves (a sessão
da frente, depois as que mais querem você) e quantas sessões precisam de você ou estão trabalhando.
Uma borda âmbar significa que alguém precisa de você.

**Clique** nele para abrir a ilha, no card quando um espera. Clique numa ave para pôr a sessão dela na
frente antes; se o card dessa ave espera atrás de outro, ele vem primeiro. O widget nunca responde a
um card.

Ele fica longe dos painéis e embaixo de janelas em tela cheia. Sem layer-shell, é uma janela sempre
no topo. Custa cerca de 40 MiB de memória enquanto está ligado.

<a id="desktop-notifications"></a>

## Notificações do desktop

Só no *Panel*, onde a ilha fica fora de vista, o Vults também usa as notificações do desktop
(qualquer desktop Linux com um servidor de notificações). No topo da tela a ilha já mostra tudo,
então você nunca é avisado duas vezes; *Paused* não mostra nenhuma.

Elas avisam quando uma sessão **terminou** (com a primeira linha da resposta), **falhou** (com o
erro), **precisa de você** (um card espera) ou [**ficou quieta**](#a-quiet-bird). Cada sessão tem no
máximo uma, com o nome do projeto no título; ela some quando o card é respondido, a sessão volta a
trabalhar ou a sessão é encerrada; terminar uma resposta ou ficar em silêncio não remove a ave. O que aconteceu enquanto elas estavam desligadas não é avisado depois.

**Open** (ou um clique) traz a ilha com aquela sessão na frente. Uma notificação nunca tem Allow nem
Deny. Desligue-as em **Settings → General → Notifications**.

<a id="the-open-island"></a>

## A ilha aberta

![A ilha aberta em repouso: o bando, cada sessão com seu nome, e um chip para cada uma que pede um olhar](../../assets/island-busy-flock.png)

Aberta, a ilha tem no cabeçalho a marca e o nome, o botão de som, a busca (a paleta, como
**Ctrl+K**), o seletor de tamanho e o botão de tela cheia. Embaixo ficam as abas:

- **Flock**, com quantas sessões precisam de você;
- **Chat**, a conversa compacta do Zeca, com motor, pasta, anexos e microfone;
- **Now**, uma caixa única para cards pendentes (inclusive adiados), sessões com falha ou quietas,
  itens dos conectores que pedem uma ação, solicitações do Chat e propostas de memória. Cada linha
  abre seu próprio card ou detalhe; abrir nunca responde a uma solicitação, envia uma mensagem,
  aceita uma proposta nem faz merge;
- **Settings** (a engrenagem): **Presence** e **Fold the island** aqui, com as outras configurações
  disponíveis no Nest.

O Now mantém cada solicitação separada, mesmo quando uma sessão tem várias. Sua contagem destaca
solicitações e falhas. **Seen** e **Mark all seen** ocultam notícias tranquilas das conexões até elas
mudarem; falhas e solicitações continuam. As marcações ficam neste computador e não mudam o GitHub.
Uma caixa vazia mostra **Nothing needs your attention**. Com o Zeca desligado, o bando independente
e as conexões continuam; os itens do Chat e da memória dele desaparecem.

Um card pendente fica na frente até ser respondido ou explicitamente adiado. Depois, o Now pode
abrir e trazer qualquer card adiado de volta pela sua solicitação. Os cards mantêm o prazo original
e o rascunho da pergunta. **Activity** e **Connections** no rodapé do Now abrem as telas completas no
[Nest](nest.md). Voltar dessas telas para o Quick abre o Now; o Chat volta para o Chat.
Os chips das sessões que precisam de você ficam abaixo das abas sobre o Chat e as Settings.
A música e o uso dos seus planos ficam numa linha abaixo das abas.
Embaixo, na largura toda da ilha (cerca de 500 px na tela, o **Quick** do seletor de tamanho):

- **em repouso, o bando**: o vult de cada sessão num fio, com o dobro do tamanho, o nome do projeto
  embaixo (numa caixa para a que está na frente) e `!` quando precisa de você (até seis; além disso,
  cinco ficam e **+N** ocupa o último lugar, abrindo o Flock do Nest, em âmbar quando uma delas
  precisa de você), com o selo; deixe o ponteiro sobre um para ver o projeto, o agente e o que ele
  está fazendo. Embaixo do fio, um chip para cada sessão que pede um olhar: um card ou uma pergunta,
  uma falha, um pássaro quieto demais;
- **uma sessão**, quando você clica no vult ou no chip dela: **‹ Flock** volta, e um ponto por
  sessão, na cor do agente, abre outra. Um card de permissão abre aqui sozinho e mantém a sessão na
  frente até você responder. À esquerda o Zeca, grande, com um brilho e uma marca na cor do estado
  da sessão (um `!` âmbar para uma permissão, um `?` ciano, brilhos verdes quando concluiu, um `#`
  vermelho quando falhou, suor num limite de uso); ao lado o projeto, o agente e o estado, e embaixo
  dos dois o card da sessão.

| Sessão | Card |
|---|---|
| Trabalhando | O projeto, o agente, os passos até agora, os subagentes rodando e, para o Claude Code, *≈$1.37*: quanto os tokens dela custariam a preços de API (uma assinatura não é cobrada disso), depois o passo que acabou e o atual |
| Ociosa | *Waiting for the next prompt*, e o último passo |
| Precisa de permissão | O comando, arquivo ou URL exato, com **Deny** e **Allow** ([Aprovações](approvals.md)) |
| Tem uma pergunta | Para uma pergunta que a ilha não consegue responder (Qwen Code, OpenCode 2): a pergunta, e **Open terminal**. As perguntas do Claude Code e do OpenCode 1 ganham um [card](approvals.md#questions) |
| Concluída | O primeiro parágrafo da última resposta do agente, **OK** e **Open terminal** |
| Falhou | O erro, **OK** e **Open terminal** |

Um passo se lê como *Editing main.rs*, *Running cargo build* ou, para uma ferramenta MCP, *Calling
github · list_prs*.

Uma edição concluída mostra as linhas que acrescentou e removeu (*Editing main.rs* **+4 −2**). Clique
nelas para ver o diff no lugar do card, com números de linha quando o agente os manda (o Claude Code
manda; os diffs do Codex não têm). **×** ou **Esc** volta. Os diffs ficam guardados
para as últimas 32 edições da sessão, até 400 linhas cada; um mais longo avisa que para antes.

![O diff de uma edição concluída no lugar do card: ticker.ts +4 −2, com números de linha](../../assets/island-live-diff.png)

**OK** diz que você viu: a sessão aparece como ociosa. Para uma conclusão bem-sucedida, o prazo
original de repouso continua valendo. Sem ninguém no fio, o card
oferece **Ask Zeca**.

O Zeca repara em você: olha para o ponteiro, ajeita as penas se você o deixa sobre ele e se assusta
com um clique. Na inicialização ele diz olá, pelo seu primeiro nome quando a sua conta de usuário tem
um nome completo (nunca tirado da rede).

<a id="now-playing"></a>

## Tocando agora

Ligue **Settings → General → Now playing** e a ilha mostra a música que o seu player está tocando
(qualquer player que fale MPRIS: Spotify, uma aba do navegador, mpv). A ilha compacta a mostra quando
nada roda; a ilha aberta a mostra na barra de cima, com anterior, tocar/pausar e próxima ao passar o
ponteiro. Enquanto ela toca, o Zeca dança e as aves ociosas do bando fazem o cortejo no fio em vez de
planar.

<a id="subscription-usage"></a>

## Uso da assinatura

![A barra de cima com o uso do plano e a música tocando](../../assets/island-usage-music.png)

A barra de cima da ilha aberta mostra quanto dos limites dos seus planos você já usou, ao lado do
ponto de cada agente: *7d 12%* é 12% da janela semanal. Fica âmbar em 70% e vermelho em 90%; passe o
ponteiro para ver quando reinicia. Nenhum token seu é lido: cada número vem da CLI em que você fez
login.

- **Codex**: com **Settings → General → Codex usage** ligado, consultado a cada 5 minutos, só
  leitura, nunca com a tela bloqueada. Chega à OpenAI pelo seu codex, então fica desligado numa
  instalação nova.
- **Claude Code** (Pro ou Max): ele só informa o uso a uma linha de status, então instalar os hooks
  acrescenta uma nossa, que roda a sua se você tiver uma ([como](../reference/settings.md#agent-configs-vults-edits)).
  Os números chegam depois da primeira resposta de uma sessão.

<a id="getting-to-a-session"></a>

## Chegando a uma sessão

Clique num vult do bando para abrir aquela sessão; ela fica na frente até você escolher outra ou
ela sair. Um card esperando ainda vem primeiro. Pelo teclado (**Ctrl+Alt+Space**), **↑** e **↓**
passam de uma sessão a outra: em repouso o nome na caixa muda, numa sessão a próxima abre.

![Uma sessão aberta pelo bando: Flock para voltar, um ponto por sessão, o card dela e Open in Cursor](../../assets/island-session.png)

**Open terminal** no card dela traz o terminal para a frente:

- um painel do herdr, tmux, kitty ou wezterm, em qualquer desktop (o kitty precisa de
  `listen_on unix:/tmp/kitty` e `allow_remote_control socket-only` no `kitty.conf`);
- a própria janela do terminal, no KDE Plasma (Wayland ou X11) e em qualquer sessão X11, na sua área
  de trabalho e desminimizada (de várias janelas, primeiro a que tem no título a pasta da sessão);
- no GNOME, Hyprland ou Sway sob Wayland, só um terminal rodando pelo XWayland.

A navegação de painéis do WezTerm exige `WEZTERM_PANE` e `WEZTERM_UNIX_SOCKET` enviados pelo hook.
O Vults usa apenas esse socket local do mesmo usuário, ignora a configuração Lua e nunca inicia
nem descobre outro servidor. Uma sessão antiga sem o socket ainda pode abrir sua janela nos
desktops compatíveis; fora deles, **Abrir terminal** fica indisponível até um evento do hook trazer
os metadados. Isso serve apenas para navegação: o [ditado](dictation.md#exact-tmux-session-delivery)
direto no WezTerm continua na área de transferência.

Quando não há nada para trazer, o card avisa. Uma sessão iniciada no terminal do VS Code ou do Cursor
leva uma etiqueta **VS Code** ou **Cursor**, e o botão dela traz esse editor, no KDE e
em sessões X11.

Os atalhos globais funcionam de qualquer janela:

- **Ctrl+Alt+Y** / **Ctrl+Alt+N** permitem ou negam a permissão na tela
  ([Aprovações](approvals.md)).
- **Ctrl+Alt+J** / **Ctrl+Alt+K** põem a próxima ou a anterior sessão na frente. Um card esperando
  fica na frente; as teclas escolhem quem vem depois dele.
- **Ctrl+Alt+V**, segurado, inicia o [ditado independente](dictation.md#global-dictation-first-slice)
  em qualquer app em foco, inclusive o Vults. Nunca abre o Chat; use seu botão de microfone para ditar no rascunho. **Ctrl+Alt+X** cancela a voz;
  **Ctrl+Alt+B** copia novamente o último ditado.
- **Ctrl+Alt+Z**, segurado por pelo menos 300 ms, fala um [comando de voz local](dictation.md#voice-commands-development-status)
  para o Vults, inclusive com o Zeca desligado. Nunca digita em outro aplicativo nem responde a cards.
- **Ctrl+Alt+1**, **Ctrl+Alt+2** e **Ctrl+Alt+3** trocam para Mini, Quick e Room.
- **Ctrl+K**, na ilha aberta ou no Nest, abre a [paleta](nest.md).
- **Ctrl+Alt+Space** abre a ilha e entrega o teclado a ela até ela se recolher:
  - **↑ ↓** (ou **K J**) trocam a sessão da frente; **Enter** abre o terminal dela; **M** abre as
    ações rápidas dela; **C** abre o chat.
  - **Y** ou **N** respondem a uma permissão; **1** a **9** escolhem uma opção de uma pergunta.
  - **Esc** recolhe a ilha e devolve as teclas. No Wayland, é o caminho de volta ao seu terminal.

Aberta com um clique, a ilha não pega teclas. Os atalhos passam pelo portal do desktop: o KDE pergunta
uma vez, e **System Settings → Shortcuts** muda as teclas (vale a pena se o seu editor usa Ctrl+Alt+J
ou K, como as IDEs da JetBrains). **Settings → General** os lista.

<a id="quick-actions"></a>

### Ações rápidas

**Clique com o botão direito** num vult da pílula ou do bando, ou no card de uma sessão
(longe do Zeca) para as ações rápidas daquela sessão:

- **Open terminal**, como acima. Onde o desktop não consegue, fica acinzentado e oferece a pasta.
- **Open folder**: no VS Code quando `code` está no seu `PATH`, senão no gerenciador de arquivos. Só uma
  pasta que existe, pelo caminho completo, é aberta, sem shell.
- **Activity**: os últimos oito passos da sessão, cada edição concluída com seu diff.
- **View the last diff**, e **Open its file in VS Code** na primeira linha alterada (ou mostrá-lo na
  pasta; um arquivo nunca é executado nem aberto pelo programa associado ao tipo dele).
- **Keep in front**, ou *Let the flock choose*.
- **Mute**, **Pin** ou **Hide this project**, e **This project's bird…** ([abaixo](#per-project-mute-pin-hide)).

O menu nunca tem Allow nem Deny, e não abre sobre um card esperando. Para chegar a um card que espera
atrás de outro, veja [Mais de um por vez](approvals.md#more-than-one-at-a-time).

<a id="while-you-were-away"></a>

### Enquanto você estava fora

Com a tela bloqueada (num desktop que avisa pelo `org.freedesktop.ScreenSaver`, como o Plasma e o
GNOME), a ilha descansa (sem animação) e os conectores param de verificar. Agentes e
cards seguem como sempre.

Quando você desbloqueia, ou sai do *Paused*, uma linha diz o que aconteceu nesse meio-tempo: *While
you were away: 2 finished, 1 failed, 1 waits for you for 12 min.* No *Island* ela abre a ilha uma vez;
fica até você dispensá-la com **×**. Projetos escondidos e silenciados ficam de fora.

<a id="a-quiet-bird"></a>

### Uma ave quieta

Uma sessão que está trabalhando e não manda nada por **5 minutos** pode estar travada (um comando
esperando entrada, uma ferramenta pendurada). A ave dela ganha um selo cinza: *No news for 5 min*.
Aos **15 minutos** o selo fica âmbar, um alerta toca e, no *Panel*, uma notificação avisa. Pensar por
muito tempo não é sinalizado; qualquer novidade limpa o sinal.

O card oferece três respostas, e nenhuma mexe no agente:

- **Snooze**: some por 15 minutos, depois volta se ela ainda estiver quieta.
- **Keep going**: está tudo bem; o sinal cinza só volta depois de 30 minutos, o âmbar 10 minutos
  depois.
- **Dismiss**: nenhum sinal de novo até a próxima pergunta dela.

Cada resposta muda apenas o acompanhamento do silêncio. Uma sessão aberta continua no bando e pode voar ou descansar, inclusive quando o sinal volta. Terminar uma resposta e ficar inativa não a encerram. Um sinal explícito de encerramento, ou a saída confirmada de um processo local antes observado no Linux, remove a ave; uma presença desconhecida a mantém visível.

<a id="per-project-mute-pin-hide"></a>

### Por projeto: silenciar, fixar, esconder

Essas escolhas pertencem ao projeto (a pasta dele): toda sessão nela as segue, agora e depois.

- **Mute**: sem sons nem notificações quando as sessões dele terminam ou falham.
- **Pin**: as sessões dele vêm primeiro no fio, na pílula e no bando.
- **Hide**: as sessões dele ficam fora da ilha, da bandeja e do widget de canto.
- **This project's bird…**: escolhe a espécie de todas as sessões dele; **Automatic** o devolve ao
  sorteio do bando.

**Settings → Projects** os lista, e é o caminho de volta para um projeto escondido.

<a id="sounds"></a>

## Sons

Bipes curtos de 8 bits quando uma sessão precisa de você, termina ou falha, e para novidades de
conectores; outros mais suaves quando a ilha abre ou se recolhe e quando o Zeca reage a você. O botão
de alto-falante na ilha, ou **Settings → General → Sounds**, os desliga; **Volume** define o quão
alto. Um card que continua esperando soa de novo, e o **Do not disturb** silencia sons e notificações
por um tempo: veja [Quando ninguém responde](approvals.md#when-nobody-answers).
