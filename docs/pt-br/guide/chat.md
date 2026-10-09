# Conversando com o Zeca
<!-- source: a26dec646f5a -->

Abra a aba **Chat**, escolha **Chat…** no ícone da bandeja ou solte um arquivo na ilha. O Zeca fica
à esquerda da conversa: ele pensa enquanto a resposta vem, inclina a cabeça quando pergunta algo a
você e engole os arquivos que você dá a ele. Escolha **Claude** ou **Codex** no topo (ou o provedor
de API que você configurou, veja abaixo). **New** começa uma conversa nova; quando há uma conversa a
perder, ele pergunta antes, e o mesmo vale para trocar de provedor. **Esc** fecha o chat de qualquer
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
  responder, e um card que ninguém responde é um não. Aqui não há **Always**: cada chat pergunta
  toda vez.

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

Os turnos do chat ignoram seus hooks, configurações e servidores MCP, inclusive o
`.claude/settings.json` do próprio projeto, então um chat nunca aparece na ilha como uma sessão de
agente, e nenhuma regra de permissão salva deixa um comando pular o card.

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
- Este chat só conversa: ele não tem ferramentas, então não pode rodar comandos, editar arquivos nem
  olhar seu projeto. Ele lê o que você digita e os arquivos que você solta: imagens como estão,
  arquivos de texto inline (até 512 KB), e PDFs também com a Anthropic.
- Modelos de raciocínio que pensam em voz alta (DeepSeek-R1, Qwen3 e outros no Ollama ou no LM Studio)
  só mostram a resposta: a parte `<think>` fica oculta, e também não é enviada de volta na próxima
  mensagem.
- Escolher outro provedor começa uma conversa nova: a nova não tem nada da antiga.
- Com a Anthropic, se o Claude recusar um pedido por motivos de segurança, a API tenta de novo com
  outro modelo Claude na mesma chamada (o fallback do lado do servidor da Anthropic). Se esse modelo
  também recusar, o balão avisa.
