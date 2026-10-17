# Conversando com o Zeca
<!-- source: 1d16118ea79c -->

Abra a aba **Chat**, escolha **Chat…** no ícone da bandeja ou solte um arquivo na ilha. O Zeca tem um
poleiro ao lado do chat: ele pensa enquanto a resposta vem, inclina a cabeça quando pergunta algo a
você e engole os arquivos que você dá a ele. Escolha **Claude** ou **Codex** no seletor de motor (ou o provedor
de API que você configurou, veja abaixo); na ilha um controle compacto mostra o motor acima
do modelo, com fundo ao passar o mouse ou enquanto está aberto. Seu menu traz o motor, o modelo
e o esforço. No Quick, as ações da conversa ficam ao lado do Zeca, e o motor fica abaixo do rascunho.
Seu menu abre para cima usando a largura do rodapé, com uma única área de rolagem para listas
longas e opções de motor, modelo e esforço que quebram linha. Uma conversa iniciada explica
seu modelo fixado dentro do menu, sem uma dica flutuante cobrindo as opções.
O Chat no Quick é um pouco mais largo que as outras abas (cerca de 561 pixels lógicos após o zoom).
O chat vazio no Quick fica baixo: o Zeca usa seu retrato original em 3x ao lado de
uma saudação, com o rascunho e os controles logo abaixo. O título some e o retrato
passa para 2x quando chegam mensagens. A superfície cresce com o conteúdo real da conversa
até atingir a altura disponível da ilha; então só a conversa rola. Começar uma conversa
vazia libera a altura anterior. Abrir, carregar e fechar o menu de motor não muda essa altura:
o menu fica sobre a conversa e rola dentro do espaço acima do rodapé. Uma área baixa mantém
o mesmo dimensionamento por conteúdo e controles menores.
A linha de uso e mídia não aparece nesse chat. Os chips de notificação só ocupam espaço quando
há itens reais: entram com expansão e somem completamente quando vazios, respeitando a redução
de movimento. Contêineres de alertas vazios não deixam espaçamento. A conversa usa a altura disponível,
sem limitar as respostas a 200 px. Os anexos rolam numa faixa horizontal, o rascunho tem altura
limitada e as mensagens mantêm sua própria área de rolagem. Uma área baixa esconde e pausa o retrato
para priorizar leitura e envio; voltar a uma área mais alta retoma a mesma cena.

Os motores vêm em dois grupos, **Your setup** (os
CLIs do Claude Code e do Codex, cada um *Signed in*, *Installed* ou *Not installed*: o Vults só olha
se o CLI e o arquivo de login dele estão lá, nunca os lê) e **Vults harness** (a API, *Ready* com uma chave ou um modelo local),
com **Engines in Settings ›** embaixo. **New** (**+** na ilha) começa uma conversa nova, e trocar de provedor
também; a anterior fica na lista (veja [Conversas](#conversations)). **Esc** primeiro fecha um seletor aberto e depois fecha o chat; a conversa continua lá quando você volta.
Só um seletor fica aberto por vez. **↑/↓** no botão abre a lista e foca uma opção;
**↑/↓**, **Home** e **End** percorrem as opções sem aplicá-las. **Enter** ou um clique escolhe;
**Esc** devolve o foco ao botão. Clicar fora fecha a lista. O carregamento dos modelos mantém
na opção o foco, e as atualizações preservam um título de conversa ainda em edição.

O Chat vazio saúda você pelo horário local do desktop: bom dia a partir das 05:00, boa
tarde a partir das 12:00 e boa noite das 18:00 às 04:59. O texto de apoio prioriza uma sessão
esperando resposta, depois uma falha, depois trabalho ativo ou um bando tranquilo. No Room
e no Full, os cartões de perguntas seguem essas mesmas regras. Uma pergunta sobre sessão
inclui sua referência **@** exata; mantém seu texto, arquivos e pasta de execução.
Edite o rascunho antes de apertar **Enviar** ou **Enter**; clicar numa sugestão não a envia.

Sob a última resposta concluída, **Explique de forma mais simples** e **O que devo verificar
agora?** acrescentam uma pergunta ao rascunho editável. **Abrir Flock** aparece quando há
sessões e abre o Flock existente, salvando esse rascunho. Esses controles também aparecem
no Quick; somem durante um turno ou quando o último resultado falhou ou foi interrompido.
As saudações e sugestões usam regras locais, sem chamada ao modelo, fala, notificação ou
resposta a uma permissão. Com o Zeca desligado, elas não abrem o Chat.

O Zeca escuta brevemente enquanto você digita, pensa enquanto espera pelo modelo ou pela
transcrição de voz e busca enquanto uma ferramenta informada está em execução. Quando
chega texto novo na resposta, ele move o bico e se acomoda após uma pausa curta; se o fluxo
fica quieto, volta a pensar até o turno terminar. É uma reação visual, sem áudio de fala.
Abrir respostas salvas ou restaurar um rascunho não a reproduz. Toda permissão pendente
do Chat mantém a pose de pergunta, mesmo quando chegam arquivos ou outra ferramenta
informa progresso. O movimento reduzido mostra uma pose estática para o estado atual;
os companheiros escondidos param de desenhar.

No Quick, o compositor começa com uma linha de escrita e cresce conforme você digita, até o limite de altura.
O clipe, o microfone opcional e Enviar ficam ao lado do rascunho; o clipe mantém o nome acessível
e a dica **Anexar um arquivo**. Os cartões de arquivos ficam numa faixa própria acima dele. Suas mensagens ficam em balões à direita;
as respostas seguem na coluna da conversa.

O chat é do Zeca: com ele desligado (**Settings → Flock & Zeca → Zeca**) não há chat nem microfone, o atalho
de fala não faz nada e os arquivos soltos na ilha não são aceitos. Veja
[Sem o Zeca](island.md#without-zeca). No KDE Plasma, o desktop continua reservando **Ctrl+Alt+V**
para o app enquanto isso; para usar a tecla em outra coisa, limpe *Hold to speak to the chat* em
Configurações do Sistema → Atalhos; a tecla de fala então fica desligada até você dar uma tecla a
ela de novo ali.

<a id="where-it-works"></a>

## Onde ele trabalha

Antes da primeira mensagem, a pasta (*in vults ▾*, no topo no Nest, embaixo do campo na ilha) diz onde a conversa vai trabalhar: a
pasta da sessão em primeiro plano, a menos que você escolha outra das sessões no fio, ou uma pasta
vazia só dela. Depois que a conversa começa, ela fica lá até **New**.

<a id="model-and-effort"></a>

## Modelo e esforço

Ao lado da pasta (na ilha, no botão abaixo do rascunho), *Default model ▾* escolhe o modelo e o quanto ele pensa, para o Claude e para o
Codex. O Claude oferece Opus, Sonnet e Haiku (os nomes do Claude Code para o mais recente de cada
um); o Codex lista os modelos que o seu próprio `codex` oferece, perguntados na primeira vez que você
abre o seletor com o Codex. O esforço vai de **Low** a **Max**, só com os que o modelo escolhido
aceita. **Default** deixa cada um para a CLI, como as suas configurações definem.

No Quick e no Nest, um anel e a porcentagem ao lado do modelo mostram a janela ativa mais
cheia do plano da CLI selecionada. **0%** é um zero informado; janelas ausentes ou expiradas
ficam sem indicador. **Atenção** começa em 70%, e **Perto do limite** em 90%. Passe o mouse no
controle para ver cada janela informada e seu horário de renovação, quando conhecido. O nome
acessível do controle inclui o uso e o estado. As janelas somem no horário de renovação sem
esperar outra leitura.

Isso reaproveita as leituras existentes da linha de status do Claude Code e as leituras
opcionais do plano do Codex nas configurações; abrir o Chat não adiciona consultas ao provedor.
O chat por API não tem indicador de assinatura. As atualizações de uso mantêm o rascunho,
os arquivos, a posição de leitura e o menu de modelo aberto.

Se uma CLI não conseguir listar os modelos, o seletor mostra uma explicação e
mantém seu rascunho e sua escolha de modelo. Feche e reabra o seletor para tentar
de novo; digitar ou atualizar o chat não repete continuamente a consulta que falhou.

A escolha é lembrada por pasta e por motor: uma conversa nova naquela pasta começa com ela. Como a
pasta, ela fica fixa quando a conversa começa, então uma resposta nunca troca de modelo no meio;
**New** começa outra. O modelo do chat por API é escolhido em **Settings → Chat & voice**.

<a id="referencing-the-flock"></a>

## Mencionando o bando

Digite **@** no rascunho para encontrar uma sessão ou projeto aberto. Busque pelo projeto,
agente, pasta ou ID da sessão. **↑/↓** percorrem as sugestões; **Enter**, **Tab** ou um clique
inserem a referência escolhida. O foco continua no rascunho, e escolher nunca envia uma
mensagem. **Esc** fecha as sugestões. Uma referência aparece como `@"Sessão: Codex · site"`
ou `@"Projeto: site"`; nomes repetidos mostram a pasta completa ou o ID da sessão. Você pode
selecionar até oito referências, incluindo várias sessões numa mensagem.

Uma referência de sessão seleciona aquele agente e ID exatos. Uma referência de projeto
seleciona as sessões do bando naquela pasta exata, incluindo seus diferentes agentes.
Ao enviar, o contexto do bando e as ferramentas que o consultam leem somente essas seleções
naquele turno. Isso funciona nos motores CLI e API. As referências não mudam a pasta de
trabalho da conversa nem respondem aos cards, e as mensagens anteriores continuam na
conversa. Sem referências, o contexto habitual do bando continua igual.

As referências acompanham o rascunho ainda não enviado entre Quick e Nest, junto com os
anexos. Remova a referência inteira entre aspas para deixar de incluí-la. Se editar, colar
ou recuperar uma referência sem sua seleção, escolha-a novamente nas sugestões de **@**.
Se uma sessão ou projeto mencionado tiver saído do bando, o Vults pede para remover a
referência e escolher novamente antes de iniciar um turno, mantendo o rascunho e os anexos
editáveis. Uma conversa salva mantém o texto legível da referência; selecione-a novamente se
trouxer esse texto para um novo rascunho.

<a id="the-nest"></a>

## O Nest

O texto ainda não enviado, os anexos e a pasta escolhida acompanham você entre a ilha e o Nest.
Voltar do Chat do Nest abre o Chat no Quick. O rascunho fica em memória até você
enviar, começar ou abrir outra conversa, ou sair do Vults.

A seta no topo do chat da ilha abre o [Nest](nest.md), a janela do próprio Vults. A aba **Chat**
dele é a mesma conversa, larga, com respostas em Markdown completo; os cards dela são respondidos
ali também. **Memory**, ao lado, mostra o que o Zeca lembra.

O Chat vazio no Nest centraliza o retrato original do Zeca em 3x acima de **Perguntar ao Zeca**
e de uma lista vertical de sugestões. Quando chegam mensagens, a mesma cena usa um poleiro
menor em 2x acima do compositor. Seu espaço próprio mantém as mensagens livres da ave
durante toda a rolagem. O cabeçalho baixo, o espaçamento menor entre mensagens e o rascunho
inicial de uma linha deixam mais espaço para leitura; o campo cresce conforme você digita.
Conversa e compositor compartilham uma coluna de até 760 px de largura. Os anexos ficam numa
faixa horizontal com altura limitada, com nomes longos legíveis dentro de cada cartão.
Janelas baixas limitam o campo de escrita e omitem a dica de teclado; com até 560 px de altura,
também recolhem o retrato e usam controles menores no compositor.

O Zeca fica no poleiro enquanto a conversa rola. Passe o mouse nele para chamar sua atenção, ou
clique em **Cumprimentar o Zeca** para ele acenar enquanto está ocioso. Pensamento, voz, perguntas
e arquivos usam as mesmas animações da ilha; um cumprimento nunca responde a um card. Em janelas
baixas, o poleiro recolhe para manter o compositor e as ações acessíveis. A animação para de desenhar
quando o Chat está oculto, a janela está minimizada ou a tela está bloqueada.

<a id="continuing-a-session"></a>

## Continuando uma sessão

`/continue <palavras>` entrega as suas palavras a uma sessão do Claude Code ou do Codex na pasta da
conversa que parou nos últimos dez minutos, como o próximo turno dela: o Vults a retoma aqui, com
as configurações dela, no sandbox dela, e ela volta a aparecer no bando com os cards dela.
Só uma sessão cujo processo acabou: uma que ainda está aberta no terminal é recusada, então você
fala com ela lá. Uma por vez; `/continue stop` encerra. Não vai para nenhum modelo do Zeca.

<a id="conversations"></a>

## Conversas

Toda conversa é salva neste computador enquanto acontece, e o botão de lista no topo do chat mostra
todas, a usada por último primeiro, com o motor e a pasta de cada uma. Clique numa para continuar de
onde parou, mesmo depois de reiniciar: ela mantém o motor, a pasta, o modelo e o esforço, e a CLI dela
retoma a própria sessão (a sessão do Claude Code, a thread do Codex; o chat por API reenvia o que foi
dito, com cada arquivo mandado guardado só como um rótulo: um arquivo de texto pelo nome, um documento pelo título, uma imagem como "uma imagem", nunca pelo conteúdo). O lápis renomeia; o xis apaga depois de um segundo clique, com tudo o que foi dito nela.

O que é salvo é o que o chat mostra: suas mensagens e os nomes dos arquivos que você mandou, as
respostas do Zeca, erros e paradas; nunca a resposta a um card (o log de auditoria guarda essas) e
nunca uma regra *Always in this chat*, então uma conversa aberta de novo pergunta como uma nova
perguntaria. As 200 mais recentes ficam guardadas, no mesmo `vults.sqlite` local do histórico de
atividade; **Settings → Activity → Clear history** apaga todas. Uma conversa cuja pasta sumiu não pode
ser aberta de novo.

<a id="what-zeca-knows"></a>
<a id="memory"></a>

## O que o Zeca sabe

Cada mensagem leva, antes das suas palavras, um bloco `<today>` com a data e a hora deste computador
e um bloco `<flock>` com uma linha para cada sessão que a ilha mostra (até 12, depois "and N
more"): o agente, o projeto, o estado e o passo em que está (segredos redigidos), nunca o comando
sobre o qual um card pergunta. Isso vai para quem roda o chat, o provedor da CLI ou o da API, junto
com a mensagem.

Com a memória ligada, o que o Zeca lembra e o que ele recupera também entram: veja
[A memória do Zeca](memory.md).

<a id="writing-and-reading"></a>

## Escrevendo e lendo

- **Enter** envia, **Shift+Enter** começa uma nova linha. Você pode digitar a próxima mensagem
  enquanto uma resposta chega, e enviá-la quando a resposta terminar. **↑** com o campo vazio traz
  de volta a sua última mensagem, para mudá-la ou enviá-la de novo.
- Enquanto o Zeca responde, o botão de enviar vira **Stop**: ele encerra o turno ali, e tudo o que o
  turno estava esperando vira um não.
- A resposta chega à medida que é escrita. Role para cima para ler e ela fica onde você está, com um
  botão *↓ new text* para voltar ao fim.
- **Copy** embaixo da última resposta copia ela inteira, no Markdown em que foi escrita; uma resposta
  mais antiga mostra o botão quando o ponteiro passa sobre ela. Um bloco de código tem o seu próprio
  **Copy**. Se a cópia falhar, o botão avisa e permite tentar de novo.
- Tanto Quick quanto Room mostram títulos, listas, citações, código e tabelas. Código e tabelas
  largas têm uma área de rolagem própria que aceita foco pelo teclado. Links continuam como
  texto, com o endereço ao lado do rótulo.
- Um turno que falha diz por quê, com **Try again**: ele envia as mesmas palavras e arquivos mais uma
  vez.
- Enquanto um turno roda, *Thinking…* avisa, com os segundos depois que ele passa de três.
- No Nest a conversa é lida em uma coluna da largura do campo de texto, por mais larga que seja a
  janela. Cada resposta vem assinada pelo Zeca e pelo motor que a escreveu, e um bloco de código diz
  a sua linguagem.

<a id="speaking-instead-of-typing"></a>

## Falando em vez de digitar

Ligue uma vez em **Settings → Chat & voice → Voice**: baixe um modelo (*Base* tem 60 MB e é rápido; *Small*
e *Large v3 Turbo* entendem melhor sotaques e nomes). Um microfone aparece ao lado de Anexar na barra de ações do compositor,
tanto no Nest quanto na ilha.

- Clique no microfone e fale. Ele para sozinho depois de cerca de 1,5 s de silêncio após as suas
  palavras, ou quando você clica de novo; as palavras caem no campo de entrada para você ler e
  corrigir, e aí **Enter** as envia. O silêncio antes de você começar nunca o interrompe; se uma
  pausa o interromper, clique de novo para continuar.
- Ou segure **Ctrl+Alt+V** de qualquer lugar: o chat abre e o Zeca escuta enquanto você segura
  (pausas nunca o interrompem); soltar transforma tudo em texto. A tecla pode ser trocada em
  Configurações do Sistema → Atalhos.
- Em Sala ou Tela cheia, o atalho usa o chat dessa janela enquanto ela tem foco.
  As barras de volume e a prévia da fala também ficam nela. Sair do Chat, ocultar
  ou fechar a janela, ou bloquear a tela cancela uma gravação ativa.
- **Esc** enquanto ele escuta descarta a gravação. Uma gravação para sozinha depois de um minuto.
- **Language you speak** (idioma que você fala) segue o do seu sistema; escolha outro, ou *Detect
  it each time*. Um idioma fixo é muito mais confiável em frases curtas. O *Base* é fraco fora do
  inglês: em português use o *Small*, ou o *Large v3 Turbo* com uma GPU.

Só a sua fala vai para o whisper: um pequeno detector de fala (Silero VAD, menos de 1 MB) corta o
silêncio em volta dela, para o whisper não inventar palavras ali. Ele é baixado sozinho enquanto a
voz está ligada e ele falta; até lá, o microfone só para com um clique. Com uma GPU (Vulkan: AMD,
Intel ou NVIDIA) uma frase leva uma fração de segundo e uma prévia esmaecida das suas palavras
aparece enquanto você fala; no processador, escolha o *Base*.

Tudo acontece neste computador: o whisper.cpp transcreve o áudio na memória, e o áudio nunca é salvo
nem enviado para lugar nenhum. Os modelos e o detector são conferidos contra o SHA-256 conhecido
deles antes de serem usados.

<a id="what-it-may-do"></a>

## O que ele pode fazer

![O Zeca pergunta no chat antes de rodar um comando](../../assets/island-chat-permission.png)

- Ele lê arquivos daquela pasta livremente.
- Todo comando e toda edição aparecem na conversa como um card igual ao da própria ilha: o que o Zeca
  pretende fazer, o comando inteiro ou o arquivo com suas linhas **+/−**, e **Deny** e **Allow** lado
  a lado com suas teclas (**Ctrl+Alt+N** / **Ctrl+Alt+Y** respondem de qualquer lugar). Nada roda até você
  responder, e um card que ninguém responde é um não. O card informa sua pasta e o estado da permissão,
  com a operação completa em uma área rolável. **Deny** ganha destaque quando o backend não
  oferece uma regra para a chamada exata ou a conversa leu conteúdo de fora; a decisão e a
  disponibilidade de **Always in this chat** continuam com o responsável atual pelas permissões.
- Depois que a conversa lê conteúdo de fora (uma página ou busca na web, um comando que rodou
  `curl`, `git pull` ou `gh`, uma ferramenta MCP que não seja as do `vults-zeca`, o
  servidor que o Vults entrega ao Claude Code do chat), todo card seguinte avisa em vermelho: uma página pode trazer palavras
  escritas para conduzir o Zeca, então leia com cuidado o que o card pede. Só conta uma chamada que
  rodou (um fetch negado não leu nada), compilar e testar nunca contam, e o aviso vale pelo resto
  daquela conversa, também quando ela é aberta de novo pela lista. Uma conversa nova começa limpa.
- **Always in this chat** permite aquela chamada exata de novo (o comando e a entrada inteiros,
  não o texto do card) sem card, até a conversa acabar: **New**, outro motor, ou o Zeca desligado.
  Um card que não consegue nomear a chamada inteira não tem esse botão. Toda vez que a regra
  responde, o chat mostra *Allowed for this conversation* e o log de auditoria guarda. Comandos
  destrutivos sempre perguntam: nunca oferecem esse botão, e uma regra existente da conversa
  não pode respondê-los. As regras nunca são salvas.

<a id="who-answers-zecas-cards"></a>

### Quem responde os cards do Zeca

**Settings → Chat & voice → Who answers Zeca's cards** escolhe, para toda conversa, na hora:

- **Asks first** (o padrão): um card para cada chamada, como acima.
- **Your policies**: as [políticas](approvals.md#policies) que você aceitou respondem as chamadas do
  chat como respondem as do Claude Code na ilha. Um deny nega, um allow permite (chamadas do Claude
  Code, dentro da pasta da conversa), o resto pergunta.
- **Full access**: tudo roda sem card, mas um comando destrutivo (`rm`, um push forçado) ainda
  pergunta, o que uma política nega é negado e o que uma política manda perguntar ainda pergunta. Depois que a conversa lê conteúdo de
  fora (uma página web, um fetch, uma ferramenta MCP), os cards dela voltam a perguntar até ela
  acabar: as palavras de uma página podem estar conduzindo a conversa. Adicione linhas `deny` a uma
  política para o que nunca deve rodar.

O chat diz cada resposta dada assim (*Allowed by your policy*, *Denied by your policy*, *Ran with
full access*), e o log de auditoria guarda. O chip dele, ao lado do modelo (vermelho com acesso
total), mostra a escolha; dali você só pode devolver acesso. Mais acesso é dado em Settings.

<a id="files"></a>

## Arquivos

Clique no clipe do chat para escolher arquivos, ou arraste um sobre a ilha e solte na área onde o
Zeca espera de bico aberto. Nos dois casos ele engole o arquivo, um urubu o leva até o outro lado
enquanto uma barra enche, e ele entra na sua próxima mensagem: um cartão dentro do compositor com o nome
e a extensão do arquivo (**×** o retira). Nomes longos quebram linha, e uma lista longa de arquivos rola
dentro do compositor. Clique no campo para escrever sobre ele. Você pode enviá-lo sem texto para perguntar o
que ele é. Imagens, PDFs, código e texto funcionam melhor. Os arquivos são copiados para
a caixa de entrada do app (até 20 MB cada) e apagados depois de uma semana; uma pasta, ou um arquivo
com mais de 20 MB, é recusado com uma linha dizendo por quê.

<a id="your-subscription-your-login"></a>

## Sua assinatura, seu login

O chat roda o comando `claude` ou `codex` em que você já fez login, então ele usa a sua própria
assinatura e o Vults nunca lê suas credenciais. Os dois transmitem a resposta à medida que ela é
escrita: o Claude por `claude -p`, o Codex por um único `codex app-server` de longa duração.

<a id="your-setup"></a>

### Sua configuração

O chat lê a sua própria configuração do Claude Code e do Codex, como no seu terminal: seu
`CLAUDE.md`, skills, servidores MCP, hooks e regras de permissão (um ícone de pessoa junto aos controles do chat avisa).
As configurações e os servidores do `.mcp.json` de um projeto só entram numa pasta em que você já
confiou no Claude Code; o `CLAUDE.md` dele é lido de qualquer jeito. As suas regras de permissão
valem em qualquer pasta. Os cards continuam perguntando o que suas regras não liberam, seja qual for
o `defaultMode` das suas configurações, e os seus hooks nunca fazem o chat aparecer como uma sessão
de agente.

Desligue em **Settings → Chat & voice → Your Claude Code and Codex setup**, e o Claude não lê nada disso; o
Codex então só deixa de fora os seus hooks. O interruptor vale para a próxima conversa. Bom saber:

- `claude -p` pula a pergunta de confiança, então um repositório que você acabou de clonar não
  consegue trazer regras ou hooks próprios.
- Uma regra de permissão como `Bash(npm test:*)` roda o script de teste de um projeto clonado sem
  card.
- Com o interruptor desligado, o Claude não recebe servidores MCP além do próprio Vults, e nenhuma
  regra salva pula um card.
- O `config.toml` do Codex ainda traz os servidores MCP dele, e o `AGENTS.md` dele ainda é lido.
- Uma conversa já em andamento mantém o que leu.

Veja [Segurança](../safety.md) para o resto.

<a id="without-a-cli-an-api-key-or-a-local-model"></a>

## Sem uma CLI: uma chave de API, ou um modelo local

O chat também pode usar a API de um provedor com a sua própria chave, ou um modelo rodando na sua
máquina. Em **Settings → Chat & voice**, escolha o **Provider**, cole a chave dele e escolha um **Model**; o
nome do provedor então aparece no seletor de motor do chat, ao lado de Claude e Codex.

| Provedor | Chave | Observações |
|---|---|---|
| Anthropic | `sk-ant-…` | Claude Opus 5.5 até você escolher outro modelo |
| OpenAI | `sk-…` | |
| Google Gemini | `AIza…` (Google AI Studio) | |
| OpenRouter | `sk-or-…` | Uma chave para centenas de modelos |
| Groq, DeepSeek, Mistral, xAI | a de cada um | |
| Ollama | nenhuma | Precisa estar rodando em `127.0.0.1:11434` |
| LM Studio | nenhuma | O servidor dele precisa estar ligado, em `127.0.0.1:1234` |

- Os modelos são listados ao vivo pelo provedor. Cada provedor guarda o modelo que você escolheu
  para ele.
- Uma chave fica no [chaveiro do sistema](../reference/settings.md#where-things-live-linux), nunca
  é mostrada de novo, e só é enviada para o próprio provedor; **Remove** a apaga. O uso é cobrado na
  sua conta com aquele provedor.
- Este chat não pode rodar comandos, editar arquivos nem olhar seu projeto. Ele lê o que você digita
  e os arquivos que você solta (imagens, texto até 512 KB, e PDFs com a Anthropic), e chama
  ferramentas quando uma pergunta precisa: as do bando (suas sessões, os cards esperando, o uso dos
  seus planos, seu card do GitHub), `recall` e `memory_propose` com a memória ligada, e as da web com
  a web ligada. O que uma ferramenta devolve vai para o provedor (comandos com os segredos
  redigidos); ele nunca responde um card.
- Modelos de raciocínio que pensam em voz alta só mostram a resposta: a parte `<think>` fica oculta
  e nunca é enviada de volta.
- Uma conversa mantém o provedor e o modelo com que começou.
- Com a Anthropic, um pedido que o Claude recusa por motivos de segurança é tentado de novo com
  outro modelo Claude na mesma chamada; se esse também recusar, o balão avisa.

<a id="the-web"></a>

### A web

Com **Settings → Chat & voice → Web for Zeca** ligado, o chat por API pode buscar na web e ler páginas. As
buscas vão para o Brave Search ou o Tavily com a sua própria chave (guardada no chaveiro), ou para
um SearXNG que você roda. Desligado, o modelo nunca recebe nenhum dos dois.

Uma página não é confiável, então o modelo do chat nunca a vê: uma chamada separada ao seu provedor
de API, sem ferramentas e avisada de que a página é dado, a lê para a pergunta, e só essa resposta
volta. Os títulos e trechos de uma busca chegam ao modelo, marcados como não confiáveis. Páginas só
são lidas de endereços públicos. Cada busca e cada página é uma requisição em seu nome com um limite
diário ([Segurança](../safety.md#every-request-vults-makes)), e a conversa então conta como tendo
lido de fora.
