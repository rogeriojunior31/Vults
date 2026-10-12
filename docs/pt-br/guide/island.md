# A ilha
<!-- source: 2b9e9614fdb3 -->

A ilha fica pendurada na borda de cima da tela. No KDE Plasma, Hyprland, Sway e outros compositores
com layer-shell, ela fica acima de todas as janelas, como um painel; no GNOME, é uma janela comum
sempre no topo. No X11, ela aparece em todas as áreas de trabalho. Ela só pega o teclado enquanto você
digita no chat, ou depois de **Ctrl+Alt+Space** ([abaixo](#getting-to-a-session)).

Com mais de uma tela, o desktop escolhe uma (em geral, a tela em foco na inicialização). Para fixar a
ilha numa tela, escolha-a em **Settings → General → Screen**. Se essa tela sumir, a ilha passa para
outra e volta quando ela retornar.

No Windows (uma versão de prévia) ainda não há atalhos globais, notificações do desktop, **Open
terminal** nem predefinições na bandeja.

<a id="the-flock"></a>

## O bando

Cada sessão de agente é uma ave, um urubu. Um projeto é uma espécie: todas as sessões dele a
compartilham, e ele a mantém toda vez que o app inicia. Uma pequena faixa na base de cada pescoço
mostra o agente: laranja para Claude Code, verde-azulado para Codex, azul para Gemini CLI, rosa para
OpenCode, índigo para Qwen Code, e violeta para qualquer outra ferramenta (Antigravity, Pi, Cursor…).

O **Zeca** não é uma sessão: ele fica no fio quando nada roda, e é com ele que você fala no chat. A
sessão da frente (a que precisa de você, a que você clicou ou, senão, a primeira a chegar) ocupa o
lugar dele. Toda outra sessão é um **vult**.

![O bando do mundo todo: um grifo-do-himalaia na frente, depois um condor-dos-andes, um abutre-preto, um abutre-real-africano e um grifo](../../assets/island-flock.png)

As aves encenam o que as sessões fazem: leem, bicam o fio enquanto editam, levantam voo para a web e
abrem as asas quando uma permissão espera. Sessões novas chegam voando; as que terminam vão embora.
Com movimento reduzido, elas ficam nos poleiros. **Settings → Flock** escolhe o conjunto de espécies,
a espécie do Zeca e os visitantes raros. [Animações](../ANIMATIONS.md) lista cada clipe.

<a id="zecas-seasonal-looks"></a>

### Os visuais sazonais do Zeca

O Zeca se veste para a estação: um chapéu de bruxa em outubro, um gorro de Papai Noel em dezembro, um
chapéu de festa no Ano-Novo e orelhas de coelho na Páscoa, pela data do seu computador. **Settings →
Flock → Look** mantém um visual o ano todo, ou nenhum, e acrescenta óculos escuros e catorze trajes
sem estação. **Clique com o botão direito no Zeca** na ilha para escolher ali mesmo: ele veste o que
estiver sob o ponteiro, um clique fica com ele, Escape o deixa como estava. Só o Zeca os usa.

![Os visuais do Zeca: os quatro sazonais, os óculos escuros e os catorze trajes com chapéus, óculos, grills e correntes](../../assets/zeca-looks.png)

<a id="without-zeca"></a>

### Sem o Zeca

O Zeca é opcional. **Settings → Flock → Zeca** o desliga; o bando continua funcionando: sessões no
fio, cards de permissão e de pergunta, os atalhos de Allow / Deny, notificações, conectores e as
predefinições. O que vai com ele:

- O chat, sua aba e a aba de soltar arquivos, o Chat do Nest, o *Chat…* da bandeja, o microfone e o atalho de
  fala. Os medidores de uso ficam: eles leem as assinaturas dos seus agentes.
- O olá dele na inicialização.
- O lugar dele no fio: a sessão da frente fica ali como a própria ave, e sem ninguém rodando a pílula
  diz *Nothing running*.

Ligue-o de volta no mesmo lugar; sem reiniciar, nos dois casos.

<a id="compact-open-and-hidden"></a>

## Compacta, aberta e escondida

![A ilha compacta: o Zeca com a sessão da frente, e vults com seus selos](../../assets/island-compact-flock.png)

**Compacta** (o **Mini** do seletor de tamanho), a ilha é uma pequena pílula sem texto, só o bando:
o Zeca à esquerda com a sessão da frente, e até oito vults ao longo do fio. As cores significam o
mesmo em todo lugar: âmbar para uma permissão, ciano para uma pergunta, verde quando concluiu,
vermelho quando falhou. Um vult num desses estados usa um selo dessa cor, e enquanto alguém precisa
de você a borda da pílula pulsa em âmbar. Deixe o ponteiro sobre um vult e ele olha para você (um
que está no ar desce); **clique** nele para abrir a ilha na sessão dele, ou **clique com o botão
direito** para as ações rápidas dele. As novidades de conectores esperam na ilha aberta.

**Clique** em qualquer outro ponto da pílula para abrir a ilha. Ela se recolhe 15 segundos depois que o ponteiro sai (uma
linha fina embaixo faz a contagem; voltar com o ponteiro cancela). O seletor de tamanho no canto
superior direito muda o quanto você vê, mantendo o que você olhava: **Mini** recolhe na hora para a
pílula, **Quick** é a ilha, **Room** abre o [Nest](nest.md) no que a ilha mostra (o bando, o chat ou
as suas conexões) e **Full** o abre na tela toda. **Settings → General → Fold the island** define a
espera.

Com **Settings → General → Open on hover** ligado, deixar o ponteiro sobre a pílula a abre sem
clique. Aberta assim, ela se recolhe assim que o ponteiro sai, a não ser que você tenha clicado nela.
Desligado por padrão, e não vale no *Panel*.

Duas coisas a mantêm aberta até você terminar:

- **Um card de permissão.** Ele abre a ilha sozinho e fica até você responder. Nada mais a abre
  sozinho: uma sessão que termina, falha ou pergunta algo toca seu som e ganha seu selo.
- **O chat.** A ilha nunca se recolhe enquanto você digita.

Sem ninguém no fio por um minuto, a ilha **se esconde**. Leve o ponteiro ao meio da borda de cima da
tela para trazê-la de volta.

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
- **Clique com o botão direito** nele para o menu: *Chat…* (com o Zeca ligado), *Set up agents…*, as
  quatro predefinições (Linux), *Quit*.
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
trabalhar ou sai (uma sessão concluída sai do fio depois de 10 minutos, uma calada depois de 30). O que aconteceu enquanto elas estavam desligadas não é avisado depois.

**Open** (ou um clique) traz a ilha com aquela sessão na frente. Uma notificação nunca tem Allow nem
Deny. Desligue-as em **Settings → General → Notifications**.

<a id="the-open-island"></a>

## A ilha aberta

![A ilha aberta: o habitat sobre o card de foco](../../assets/island-busy-flock.png)

Aberta, a ilha tem as abas como ícones: **Flock**, **Tasks** (as suas tarefas abertas), **Chat** e
**Drop a file** (como entregar um arquivo ao Zeca), e **Activity** (esta semana, contada neste
computador). Tasks e Activity são visões curtas, cada uma com **Open in the Nest** para o resto.
Quando o GitHub responde, mais uma aba mostra o card dele
([Conectores](connectors.md#the-github-card)). A seta no topo do chat abre o
[Nest](nest.md), a janela do próprio Vults, com abas próprias. À direita: um botão de som, o
botão de configurações e o seletor de tamanho; a música e o uso dos seus planos ficam numa linha
abaixo deles. As Settings abrem em **Agents** enquanto os hooks de um agente faltam ou estão
desatualizados, e em **General** nos outros casos. Embaixo, na largura toda da ilha (440 px, o
**Quick** do seletor de tamanho):

- **o habitat**: o vult de cada outra sessão num fio (até doze; além disso, **+N** abre o Flock do
  Nest, em âmbar quando uma delas precisa de você), com o selo; deixe o ponteiro sobre um para ver
  o projeto, o agente e o que ele está fazendo;
- **o card de foco**: o Zeca, grande, com um brilho e uma marca na cor do estado da sessão (um `!`
  âmbar para uma permissão, um `?` ciano, brilhos verdes quando concluiu, um `#` vermelho quando
  falhou, suor num limite de uso), e o card dessa sessão.

| Sessão | Card |
|---|---|
| Trabalhando | O projeto, o agente, os passos até agora, os subagentes rodando e, para o Claude Code, *≈$1.37*: quanto os tokens dela custariam a preços de API (uma assinatura não é cobrada disso), depois o passo que acabou e o atual |
| Ociosa | *Waiting for the next prompt*, e o último passo |
| Precisa de permissão | O comando, arquivo ou URL exato, com **Deny** e **Allow** ([Aprovações](approvals.md)) |
| Tem uma pergunta | Para uma pergunta que a ilha não consegue responder (Gemini CLI, Qwen Code, OpenCode 2): a pergunta, e **Open terminal**. As perguntas do Claude Code e do OpenCode 1 ganham um [card](approvals.md#questions) |
| Concluída | O primeiro parágrafo da última resposta do agente, **OK** e **Open terminal** |
| Falhou | O erro, **OK** e **Open terminal** |

Um passo se lê como *Editing main.rs*, *Running cargo build* ou, para uma ferramenta MCP, *Calling
github · list_prs*.

Uma edição concluída mostra as linhas que acrescentou e removeu (*Editing main.rs* **+4 −2**). Clique
nelas para ver o diff no lugar do card, com números de linha quando o agente os manda (o Claude Code
manda; os diffs do Codex e do Gemini CLI não têm). **×** ou **Esc** volta. Os diffs ficam guardados
para os últimos oito passos da sessão, até 400 linhas cada; um mais longo avisa que para antes.

![O diff de uma edição concluída no lugar do card: ticker.ts +4 −2, com números de linha](../../assets/island-live-diff.png)

**OK** diz que você viu: o selo some e a sessão aparece como ociosa. Sem ninguém no fio, o card
oferece **Ask Zeca**.

O Zeca repara em você: olha para o ponteiro, ajeita as penas se você o deixa sobre ele e se assusta
com um clique. Na inicialização ele diz olá, pelo seu primeiro nome quando a sua conta de usuário tem
um nome completo (nunca tirado da rede).

<a id="now-playing"></a>

## Tocando agora

Ligue **Settings → General → Now playing** e a ilha mostra a música que o seu player está tocando
(qualquer player que fale MPRIS: Spotify, uma aba do navegador, mpv). A ilha compacta a mostra quando
nada roda; a ilha aberta a mostra na barra de cima, com anterior, tocar/pausar e próxima ao passar o
ponteiro. As aves ociosas dançam enquanto ela toca.

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
  acrescenta uma nossa, que não mostra nada. Os números chegam depois da primeira resposta de uma
  sessão. Uma linha de status sua continua aparecendo: a nossa a roda, e remover os hooks a devolve.

<a id="getting-to-a-session"></a>

## Chegando a uma sessão

Clique num vult do habitat para pôr aquela sessão na frente, até você escolher outra ou ela
sair. Um card esperando ainda vem primeiro.

**Open terminal** no card dela traz o terminal para a frente:

- um painel do herdr, tmux, kitty ou wezterm, em qualquer desktop (o kitty precisa de
  `listen_on unix:/tmp/kitty` e `allow_remote_control socket-only` no `kitty.conf`);
- a própria janela do terminal, no KDE Plasma (Wayland ou X11) e em qualquer sessão X11, na sua área
  de trabalho e desminimizada (de várias janelas, primeiro a que tem no título a pasta da sessão);
- no GNOME, Hyprland ou Sway sob Wayland, só um terminal rodando pelo XWayland.

Quando não há nada para trazer, o card avisa. Uma sessão iniciada no terminal do VS Code ou do Cursor
leva uma etiqueta **VS Code** ou **Cursor**, e o botão dela traz esse editor, no KDE e
em sessões X11.

Os atalhos globais funcionam de qualquer janela:

- **Ctrl+Alt+Y** / **Ctrl+Alt+N** permitem ou negam a permissão na tela
  ([Aprovações](approvals.md)).
- **Ctrl+Alt+J** / **Ctrl+Alt+K** põem a próxima ou a anterior sessão na frente. Um card esperando
  fica na frente; as teclas escolhem quem vem depois dele.
- **Ctrl+Alt+V**, segurado, fala com o chat ([Falar](chat.md#speaking-instead-of-typing)).
- **Ctrl+Alt+1**, **Ctrl+Alt+2** e **Ctrl+Alt+3** trocam para Mini, Quick e Room.
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

**Clique com o botão direito** num vult da pílula ou do habitat, ou no card de foco
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

Cada resposta conta como novidade, então a ave fica no fio (uma calada sai depois de 30 minutos);
depois de *Keep going* ela fica até o sinal poder voltar.

<a id="per-project-mute-pin-hide"></a>

### Por projeto: silenciar, fixar, esconder

Essas escolhas pertencem ao projeto (a pasta dele): toda sessão nela as segue, agora e depois.

- **Mute**: sem sons nem notificações quando as sessões dele terminam ou falham.
- **Pin**: as sessões dele vêm primeiro no fio, na pílula e no habitat.
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
