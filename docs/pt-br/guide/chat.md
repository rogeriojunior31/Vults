# Conversando com o Zeca
<!-- source: 526f835b9fbd -->

Abra a aba **Chat**, escolha **Chat…** no ícone da bandeja ou solte um arquivo na ilha. O Zeca fica
à esquerda da conversa: ele pensa enquanto a resposta vem, inclina a cabeça quando pergunta algo a
você e engole os arquivos que você dá a ele. Escolha **Claude** ou **Codex** no topo (ou o provedor
de API que você configurou, veja abaixo). **New** começa uma conversa nova, e trocar de provedor
também; a anterior fica na lista (veja [Conversas](#conversations)). **Esc** fecha o chat de qualquer
ponto dele; a conversa continua lá quando você volta.

O chat é do Zeca: com ele desligado (**Settings → Flock → Zeca**) não há chat nem microfone, o atalho
de fala não faz nada e os arquivos soltos na ilha não são aceitos. Veja
[Sem o Zeca](island.md#without-zeca).

O desktop continua reservando **Ctrl+Alt+V** para o app enquanto o Zeca está desligado, e o app
ignora o toque. No KDE Plasma, o serviço de atalhos do desktop não consegue deixar uma tecla de lado
e mantê-la: o app teria que soltar a tecla, e aí religar o Zeca pediria que você a associasse de
novo. Se você quiser a tecla para outra coisa enquanto o Zeca está desligado, limpe *Hold to speak
to the chat* em Configurações do Sistema → Atalhos (System Settings → Shortcuts); o atalho de fala
fica desligado até você dar uma tecla a ele lá de novo.

<a id="where-it-works"></a>

## Onde ele trabalha

Antes da primeira mensagem, a pasta no topo (*in vults ▾*) diz onde a conversa vai trabalhar: a
pasta da sessão em primeiro plano, a menos que você escolha outra das sessões no fio, ou uma pasta
vazia só dela. Depois que a conversa começa, ela fica lá até **New**.

<a id="model-and-effort"></a>

## Modelo e esforço

Ao lado da pasta, *Default model ▾* escolhe o modelo e o quanto ele pensa, para o Claude e para o
Codex. O Claude oferece Opus, Sonnet e Haiku (os nomes do Claude Code para o mais recente de cada
um); o Codex lista os modelos que o seu próprio `codex` oferece, perguntados na primeira vez que você
abre o seletor com o Codex. O esforço vai de **Low** a **Max**, só com os que o modelo escolhido
aceita. **Default** deixa cada um para a CLI, como as suas configurações definem.

A escolha é lembrada por pasta e por motor: uma conversa nova naquela pasta começa com ela. Como a
pasta, ela fica fixa quando a conversa começa, então uma resposta nunca troca de modelo no meio;
**New** começa outra. O modelo do chat por API é escolhido em **Configurações → Chat**.

<a id="the-nest"></a>

## O Nest

A seta no topo do chat da ilha abre o **Nest**, a janela do próprio Zeca: a mesma conversa, larga, com
respostas em Markdown completo (títulos, tabelas, citações, listas numeradas; um link mostra o
endereço e nunca é seguido). Uma mensagem enviada em qualquer um aparece nos dois, e o mesmo vale para
**New**, outro motor, ou uma conversa aberta pela lista. Um card também aparece no Nest, mas só a ilha
responde cards: **Answer on the island** traz o chat da ilha com ele. O Nest é criado quando você o
abre e some quando você o fecha, e desligar o Zeca o fecha. Respostas curtas continuam melhores na
bolha da ilha.

A aba **Roosts** do Nest agrupa o bando por repositório: um card para cada repositório em que uma
sessão trabalha, com as pastas e os branches delas (as worktrees de um repositório dividem um card),
as sessões, os seus pull requests nele e os checks do branch padrão (do card do GitHub, quando esse
conector está ligado), e o que o Zeca lembra do projeto. Ela lê os repositórios neste computador e
não pede nada à rede. Uma conversa que começa num repositório leva essa memória junto com o resto
da do Zeca.

<a id="conversations"></a>

## Conversas

Toda conversa é salva neste computador enquanto acontece, e o botão de lista no topo do chat mostra
todas, a usada por último primeiro, com o motor e a pasta de cada uma. Clique numa para continuar de
onde parou, mesmo depois de reiniciar: ela mantém o motor, a pasta, o modelo e o esforço, e a CLI dela
retoma a própria sessão (a sessão do Claude Code, a thread do Codex; o chat por API reenvia o que foi
dito). O lápis renomeia; o xis apaga depois de um segundo clique, com tudo o que foi dito nela.

O que é salvo é o que o chat mostra: suas mensagens e os nomes dos arquivos que você mandou, as
respostas do Zeca, erros e paradas; nunca a resposta a um card (o log de auditoria guarda essas) e
nunca uma regra *Sempre nesta conversa*, então uma conversa aberta de novo pergunta como uma nova
perguntaria. As 200 mais recentes ficam guardadas, no mesmo `vults.sqlite` local do histórico de
atividade; **Settings → Activity → Clear history** apaga todas. Uma conversa cuja pasta sumiu não pode
ser aberta de novo.

<a id="what-zeca-knows"></a>

## O que o Zeca sabe

Cada mensagem leva, antes das suas palavras, a data e a hora deste computador e uma linha para cada
sessão que a ilha mostra: o agente, o projeto, o estado e o passo em que está (segredos redigidos),
nunca o comando sobre o qual um card pergunta. Isso vai para quem roda o chat, o provedor da CLI ou
o da API, junto com a mensagem.

A memória do Zeca entra numa conversa quando ela começa: quem você é, o que ele aprendeu e como
falar com você, depois das instruções dele. Ela fica como estava durante a conversa toda, então uma
resposta nunca a vê mudar no meio, e o prompt é o mesmo a cada turno (o cache de um provedor
continua funcionando); uma memória adicionada nesse meio-tempo entra na próxima conversa. Enquanto a
memória dele não tiver nada, só entra uma linha dizendo o que são os blocos de data e de sessões. Uma conversa começada antes
desta versão não recebe nenhum dos dois.

<a id="memory"></a>

## Memória

A memória do Zeca fica desligada até você ligá-la em **Settings → Chat → Memory**. Ligada, o app
mantém um diário neste computador: o tempo, os passos e as mudanças de cada turno dos agentes, a
resposta de cada card com o comando ou caminho, e as linhas do chat do Zeca (as suas palavras e as
dele), com os segredos redigidos antes de qualquer coisa
ser escrita, por 90 dias. Nada disso sai do computador. Desligá-la para a escrita e mantém o que já
foi escrito; **Settings → Activity → Clear history** apaga.

Com a memória ligada, **A summary of each day** pode resumir cada dia passado do diário para o Zeca
lembrar depois: uma vez por dia, o diário do dia anterior vai para o menor modelo do Claude Code
(Haiku), na sua assinatura, sem ferramentas e sem nenhuma das suas configurações, e o resumo (no
máximo 2.000 bytes) fica guardado junto do diário. É uma requisição em seu nome: desligada até você
ligá-la, no máximo 8 por dia, cada uma no rastro local. Só vão os dias a partir de quando você a
liga, nunca o diário guardado antes. Um dia que falha é tentado de novo uma hora depois, no máximo
três vezes.

Com a memória ligada, o Zeca lembra: antes de cada mensagem, o app procura no diário e nos resumos
dos dias o que as suas palavras tratam, e até três trechos entram na mensagem (como o bloco
`<memories>`, contexto e nunca instruções); o chat por API também pode pedir mais com a ferramenta
`recall`. A busca roda neste computador; o chat de hoje fica de fora (é a conversa em andamento), e
também tudo o que uma conversa disse depois de ler conteúdo de fora. Com **Recall by meaning**
ligado e um [Ollama](https://ollama.com) rodando aqui com o `nomic-embed-text`
(`ollama pull nomic-embed-text`), ele também acha resumos pelo sentido, não só pelas palavras; ele
é acessado só em `127.0.0.1`. Os trechos vão para o provedor do chat junto com a sua mensagem.

Só você escreve a memória do Zeca, de três jeitos:

- **`/remember <fato>`** no chat guarda um fato no que o Zeca aprendeu; **`/remember project:
  <fato>`** no projeto em que a conversa está. **`/forget <palavras>`** remove o único fato que as
  contém (com vários, ele os lista para você dizer mais). Os dois respondem na hora, sem modelo:
  as suas próprias palavras, guardadas como suas.
- **O que o Zeca propõe** espera na aba **Memory** do Nest: o modelo do chat por API pode propor um
  fato com a ferramenta `memory_propose`, e com **Suggest what to remember** ligado, quando você
  começa uma conversa nova ou abre outra, o menor modelo do Claude Code lê a que você deixou, com os
  segredos redigidos, e propõe até três (nunca uma que leu conteúdo de fora, e só quando ela tem
  linhas novas desde a última leitura; uma requisição em seu nome, no máximo 20 por dia, no rastro
  local). Cada proposta mostra para onde vai (um projeto pelo nome e pela pasta) e a mudança como um
  diff; **Remember** guarda, **Turn down** descarta. Um fato já lembrado, ou o mesmo fato (sem
  contar maiúsculas e espaços) decidido no último mês, não é proposto de novo.
- A aba Memory também lista o que o Zeca lembra, arquivo por arquivo com o quanto cada um está
  cheio, e o histórico: quem escreveu cada mudança (você, ou o Zeca com o seu sim) e **Undo**, que
  a desfaz numa mudança própria (recusado quando uma mudança depois mexeu no mesmo arquivo; nunca
  oferecido numa mudança que o Vults não escreveu). A memória
  dele é um repositório git de pequenos arquivos Markdown em `~/.local/share/vults/memory/`.

<a id="writing-and-reading"></a>

## Escrevendo e lendo

- **Enter** envia, **Shift+Enter** começa uma nova linha. Você pode digitar a próxima mensagem
  enquanto uma resposta chega, e enviá-la quando a resposta terminar.
- Enquanto o Zeca responde, o botão de enviar vira **Stop**: ele encerra o turno ali, e tudo o que o
  turno estava esperando vira um não.
- A resposta chega à medida que é escrita. Role para cima para ler e ela fica onde você está, com um
  botão *↓ new text* para voltar ao fim.

<a id="speaking-instead-of-typing"></a>

## Falando em vez de digitar

Ligue uma vez em **Settings → Chat → Voice**: baixe um modelo (*Base* tem 60 MB e é rápido; *Small*
e *Large v3 Turbo* entendem melhor sotaques e nomes). Um microfone aparece ao lado do botão de
enviar.

- Clique no microfone e fale: uma forma de onda mostra o que ele ouve. Quando você para de falar
  (cerca de 1,5 s de silêncio depois das suas palavras) ele para sozinho, ou clique de novo (o stop
  vermelho) quando quiser. O Zeca pensa enquanto o áudio vira texto; as palavras caem no campo de
  entrada para você ler e corrigir, e aí **Enter** as envia como sempre. O silêncio antes de você
  começar nunca o interrompe, então vá com calma, e uma respirada entre frases também não; se uma
  pausa mais longa o interromper, clique no microfone de novo para continuar (as palavras novas são
  acrescentadas).
- Ou segure **Ctrl+Alt+V** de qualquer lugar: o chat abre, o Zeca inclina a cabeça e escuta enquanto
  você segura, e soltar transforma tudo em texto (pausas nunca o interrompem enquanto você segura a
  tecla). A tecla pode ser trocada em Configurações do Sistema → Atalhos.
- **Esc** enquanto ele escuta descarta a gravação. Uma gravação para sozinha depois de um minuto.
- **Language you speak** (idioma que você fala) segue o do seu sistema (português num desktop
  `pt_BR`); escolha outro, ou *Detect it each time*. Um idioma fixo é muito mais confiável em frases
  curtas. O *Base* é fraco fora do inglês: em português use o *Small*, ou o *Large v3 Turbo* com uma
  GPU.
- Só a sua fala vai para o whisper: o silêncio e o ruído em volta dela são cortados, para o whisper
  não inventar palavras neles (*Thank you.*, *Obrigado.*), e uma gravação sem fala não gera texto.
  Um pequeno detector de fala (Silero VAD, menos de 1 MB, MIT) separa fala de silêncio; ele é baixado
  junto com um modelo, e sozinho em segundo plano sempre que a voz está ligada sem ele (um modelo de
  uma versão mais antiga, ou uma tentativa que falhou offline). Até ele estar lá, o microfone só para
  com um clique e o corte é feito pelo volume.

No Linux, ele roda na placa de vídeo via Vulkan quando há uma (AMD, Intel ou NVIDIA): uma frase leva
uma fração de segundo, mesmo com o *Large v3 Turbo*. Sem uma GPU utilizável ele roda no processador,
e aí o *Base* é o modelo a escolher.

Com uma GPU você também vê suas palavras enquanto fala: mais ou menos a cada 0,8 s, a gravação até
ali é lida de novo, e o texto dela aparece esmaecido ao lado de uma forma de onda mais curta, com as
palavras mais novas à vista. É só uma prévia: quando o microfone para, a transcrição final a
substitui, e é ela que cai no campo de entrada. No processador não há prévia (uma passada leva mais
de meio segundo mesmo com o *Base*, demais para repetir enquanto você fala), então a forma de onda e
*Listening…* aparecem como antes.

Tudo acontece neste computador: o whisper.cpp transcreve o áudio na memória, e o áudio nunca é salvo
nem enviado para lugar nenhum. Os modelos vêm dos repositórios do whisper.cpp no Hugging Face
(`ggerganov/whisper.cpp`, e `ggml-org/whisper-vad` para o detector de fala) e são conferidos contra
o SHA-256 conhecido deles antes de serem usados.

<a id="what-it-may-do"></a>

## O que ele pode fazer

![O Zeca pergunta no chat antes de rodar um comando](../../assets/island-chat-permission.png)

- Ele lê arquivos daquela pasta livremente.
- Todo comando e toda edição aparecem na conversa como um card igual ao da própria ilha: o que o Zeca
  pretende fazer, o comando inteiro ou o arquivo com suas linhas **+/−**, e **Deny** e **Allow** com
  suas teclas (**Ctrl+Alt+N** / **Ctrl+Alt+Y** respondem de qualquer lugar). Nada roda até você
  responder, e um card que ninguém responde é um não.
- Depois que a conversa lê conteúdo de fora (uma página ou busca na web, um comando que rodou
  `curl`, `git pull` ou `gh`, uma ferramenta MCP), todo card seguinte avisa em vermelho: uma página
  pode trazer palavras escritas para conduzir o Zeca, então leia com cuidado o que o card pede. Só
  conta uma chamada que rodou (um fetch negado não leu nada), compilar e testar nunca contam, e o
  aviso vale pelo resto daquela conversa, também quando ela é aberta de novo pela lista. Uma conversa
  nova começa limpa.
- **Sempre nesta conversa** permite aquele comando exato de novo, sem card, até a conversa acabar:
  o comando inteiro e a entrada inteira da chamada, não o texto do card, então um comando diferente
  que começa igual ainda pergunta. Um card que não consegue nomear a chamada inteira (uma edição
  cujas mudanças o Codex não mostrou) não tem esse botão. Toda vez que a regra responde, o chat mostra
  *Permitido nesta conversa* com o que rodou, e o log de auditoria guarda isso como resposta da sua
  regra. **New**, outro motor, ou o Zeca desligado encerram a conversa e as regras dela. As regras
  nunca são salvas: uma conversa aberta de novo pela lista começa sem nenhuma.

<a id="files"></a>

## Arquivos

Arraste um arquivo sobre a ilha e ela abre uma área de soltar, com o Zeca esperando de bico aberto (a
aba **+** também a mostra). Solte: ele engole o arquivo, um urubu o leva até o outro lado enquanto
uma barra enche, e a área pergunta para que ele serve. **Ask about it** o mantém na sua próxima
mensagem e coloca você no campo de entrada; **Cancel** o retira. Você pode enviá-lo sem texto para
perguntar o que ele é. Imagens, PDFs, código e texto funcionam melhor. Os arquivos são copiados para
a caixa de entrada do app (até 20 MB cada) e apagados depois de uma semana; uma pasta, ou um arquivo
com mais de 20 MB, é recusado com uma linha dizendo por quê.

<a id="your-subscription-your-login"></a>

## Sua assinatura, seu login

O chat roda o comando `claude` ou `codex` em que você já fez login, então ele usa a sua própria
assinatura e o Vults nunca lê suas credenciais. Os dois transmitem a resposta à medida que ela é
escrita: o Claude por `claude -p`, o Codex por um único `codex app-server` de longa duração.

<a id="your-setup"></a>

### Sua configuração

O chat lê a sua própria configuração do Claude Code e do Codex, como o `claude` e o `codex` fazem no
seu terminal: seu `CLAUDE.md`, skills, servidores MCP, hooks e regras de permissão. Um pequeno ícone de
pessoa no topo do chat avisa (a dica dele diz o que é lido). As configurações do próprio projeto (o
`.claude/settings.json` e o `settings.local.json` dele) só entram numa pasta em que você já confiou
no Claude Code: o `claude -p` pula o prompt de confiança, então um repositório que você acabou de
clonar não traz regras nem hooks próprios. O `CLAUDE.md` do projeto é lido de qualquer jeito.

Os cards continuam perguntando: um comando ou uma edição que suas regras não cobrem espera o seu
Allow, mesmo que suas configurações definam um `defaultMode` que pularia a pergunta (`acceptEdits`,
`bypassPermissions`). Seus hooks rodam, o do Vults entre eles, mas só marcam os eventos do chat como
do Zeca: o chat nunca aparece na ilha como uma sessão de agente e nunca pergunta duas vezes.

As suas próprias regras de permissão valem em qualquer pasta, como no seu terminal: uma regra como
`Bash(npm test:*)` roda o script de testes de um projeto clonado sem card. Os servidores do
`.mcp.json` de um projeto só sobem numa pasta em que você confia.

Desligue em **Configurações → Chat → Sua configuração do Claude Code e do Codex**, e o Claude não lê
nada disso: nenhum hook, nenhuma configuração, nenhum servidor MCP além dos dele, e nenhuma regra
salva deixa um comando pular o card. O Codex então só deixa de fora os seus hooks; ele sempre lê o
próprio `config.toml`, com os servidores MCP e o `AGENTS.md`. A chave vale para a próxima conversa;
uma que já começou mantém o que leu.

<a id="without-a-cli-an-api-key-or-a-local-model"></a>

## Sem um CLI: uma chave de API, ou um modelo local

Se você não usa o Claude Code nem o Codex, o chat pode usar a API de um provedor com a sua própria
chave, ou um modelo rodando na sua máquina. Em **Settings → Chat**, escolha o **Provider**, cole a
chave dele e escolha um **Model**; o nome do provedor então aparece no topo do painel do chat, ao lado
de Claude e Codex.

| Provedor | Chave | Observações |
|---|---|---|
| Anthropic | `sk-ant-…` | Claude Opus 5.5 até você escolher outro modelo |
| OpenAI | `sk-…` | |
| Google Gemini | `AIza…` (Google AI Studio) | |
| OpenRouter | `sk-or-…` | Uma chave para centenas de modelos |
| Groq, DeepSeek, Mistral, xAI | a de cada um | |
| Ollama | nenhuma | Precisa estar rodando em `127.0.0.1:11434` |
| LM Studio | nenhuma | O servidor dele precisa estar ligado, em `127.0.0.1:1234` |

- Os modelos são listados ao vivo pelo provedor, então os novos aparecem sem uma atualização. Cada
  provedor guarda o modelo que você escolheu para ele.
- Uma chave fica guardada no chaveiro do sistema, nunca num arquivo, o app nunca a mostra de novo, e
  ela só é enviada para o próprio provedor. **Remove** a apaga do chaveiro, e a próxima mensagem para
  de funcionar na hora. Um modelo local não precisa de chave, e nada sai da sua máquina.
- O uso é cobrado na sua conta com aquele provedor.
- Este chat não pode rodar comandos, editar arquivos nem olhar seu projeto. Ele lê o que você digita
  e os arquivos que você solta (imagens como estão, arquivos de texto inline até 512 KB, e PDFs
  também com a Anthropic), e o que a ilha mostra, por ferramentas que ele chama quando uma pergunta
  precisa: suas sessões e o que cada uma está fazendo, os cards esperando, o uso dos seus planos e o
  seu card do GitHub. Pergunte "o que o Codex está fazendo?" e ele olha. O que uma ferramenta devolve
  vai para o provedor junto com o resto da conversa (comandos com os segredos redigidos); ele nunca
  responde um card que espera.
- Modelos de raciocínio que pensam em voz alta (DeepSeek-R1, Qwen3 e outros no Ollama ou no LM Studio)
  só mostram a resposta: a parte `<think>` fica oculta, e também não é enviada de volta na próxima
  mensagem.
- Uma conversa mantém o provedor e o modelo com que começou; escolher outro provedor nas
  configurações vale para a próxima.
- Com a Anthropic, se o Claude recusar um pedido por motivos de segurança, a API tenta de novo com
  outro modelo Claude na mesma chamada (o fallback do lado do servidor da Anthropic). Se esse modelo
  também recusar, o balão avisa.

<a id="the-web"></a>

### A web

Com **Settings → Chat → Web for Zeca** ligado, o chat por API pode buscar na web e ler páginas quando
uma pergunta precisa. As buscas vão para o serviço que você escolher: Brave Search ou Tavily com a sua
própria chave (guardada no chaveiro do sistema), ou um SearXNG que você mesmo roda. Desligado, o
modelo nunca recebe nenhum dos dois.

Uma página não é confiável: ela pode trazer palavras escritas para conduzir o Zeca. Por isso o
modelo do chat nunca a vê, nem o título dela. Uma chamada separada ao seu provedor de API, sem
ferramentas e avisada de que a página é dado, a lê para a pergunta, e só essa resposta volta. Os
resultados de uma busca chegam ao modelo, como títulos e trechos curtos marcados como não
confiáveis. Cada busca e cada página é uma
requisição em seu nome: no máximo 200 por dia, cada uma no registro local. Páginas só são lidas de
endereços públicos, nunca deste computador ou da rede dele. A conversa então conta como tendo lido
de fora, e todo card depois avisa.
