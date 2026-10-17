# Segurança
<!-- source: 59f9d0d700e6 -->

O que o Vults promete:

- **Ele nunca bloqueia seu agente.** Se o app estiver fechado, lento ou quebrado, o hook sai em
  milissegundos sem imprimir nada, e seu agente pergunta no terminal como sempre.
- **Só você responde a uma permissão.** Nenhum timer ou padrão responde a uma. Além do seu clique,
  só estes respondem: uma regra exata que você criou com **Always**, para aquele comando ou arquivo;
  uma [política](guide/approvals.md#policies) que você escreveu, viu como diff e aceitou com um
  clique (um deny vence um allow); um telefone que você pareou, quando a resposta dele é assinada pela chave dele para aquele
  card exato, no prazo, e nunca vista antes (aprovar pede a sua biometria no telefone, negar não;
  veja [O telefone](guide/phone.md)); e, só no chat do Zeca, o acesso que você escolheu para os cards
  dele ([Quem responde os cards do Zeca](guide/chat.md#who-answers-zecas-cards)); no acesso total,
  a negação ou o perguntar de uma política continuam valendo, e conteúdo de fora traz os cards de
  volta. Uma chamada
  destrutiva (`rm -rf`, um push forçado) sempre pergunta: o card dela não oferece **Always**, e
  nenhuma regra, política ou acesso total a responde. Uma pergunta do Claude Code só é respondida com
  a escolha ou as palavras que você deu no card dela.
- **Só a sua conta de usuário fala com ele.** O socket fica na sua pasta de runtime privada, com modo
  `0600`, e os dois lados verificam se o outro processo roda como você (no Windows, pelo SID).
- **Olhar a pasta de uma sessão só a lê.** Para ver os arquivos que os comandos de uma sessão
  escreveram, no início e no fim de cada turno o app lista o repositório da pasta dela com
  `git ls-files` e ele mesmo compara o tamanho e a data de modificação de cada arquivo. O git não lê
  o conteúdo de nenhum arquivo e não escreve nada; o fsmonitor e os hooks ficam desligados, então
  nenhum filtro, hook ou monitor da configuração do repositório roda.
- **Ele nunca edita a configuração de um agente pelas suas costas.** Toda mudança ganha um backup
  datado e um diff que você aprova; os hooks de outras ferramentas são mantidos, uma status line do
  Claude Code que seja sua continua rodando ([como](reference/settings.md#agent-configs-vults-edits)),
  e o `trusted_hash` do Codex nunca é escrito (você mesmo confia nos nossos hooks, no `/hooks` do
  Codex).
- **O uso do plano vem só das CLIs.** A entrada da status line do Claude Code é reduzida no hook
  aos `rate_limits` dela e ao custo da sessão antes de qualquer coisa sair dele; o Codex é consultado com uma chamada só de leitura,
  só enquanto **Settings → General → Codex usage** estiver ligado.
- **O chat usa as CLIs em que você fez login** (`claude`, `codex`), e o Vults nunca lê as
  credenciais delas; todo comando e toda edição que suas próprias regras não liberam esperam o seu
  Allow (veja [Sua configuração](guide/chat.md#your-setup)).
- **As chaves de API ficam no chaveiro do sistema.** Se você der ao chat a chave de API de um provedor, ela é gravada
  só no chaveiro (Secret Service no Linux), nunca em um arquivo ou log, nunca devolvida a uma
  janela, e só é enviada para aquele provedor. Modelos locais (Ollama, LM Studio) são acessados só em
  `127.0.0.1`. O chat pela API não executa comandos nem edita arquivos. As ferramentas dele leem o
  que a ilha mostra (sessões, cards esperando, uso, seu card do GitHub); com a memória ligada, o
  `recall` lê o diário e o `memory_propose` só escreve uma proposta; com a web ligada, ele recebe as
  ferramentas da web. O que elas devolvem vai para aquele provedor junto com a conversa, comandos
  com os segredos redigidos.
- **Cada mensagem do chat diz o que a ilha mostra.** Por uma CLI ou por uma API, uma mensagem ao
  Zeca leva a data e a hora e uma linha para cada sessão (agente, projeto, estado, o passo dela com
  os segredos redigidos), nunca o comando de um card. Com a memória ligada, o que o Zeca lembra
  entra quando uma conversa começa, e até 3 trechos lembrados vão com cada mensagem.
- **O microfone só escuta enquanto você segura uma gravação.** A voz fica desligada até você baixar um
  modelo; depois o mic grava só na memória enquanto o botão dele está vermelho (no máximo um minuto), e
  o whisper.cpp transcreve neste computador. O áudio nunca é salvo nem enviado para lugar nenhum.
- **O Now playing fica desligado até você ligar.** Aí o app lê seus players de mídia pelo
  session bus (MPRIS no Linux) para mostrar a música; isso nunca é guardado nem enviado para lugar nenhum.
- **O que o app pede por conta própria passa por um portão só.** Uma requisição feita sem um
  clique naquele momento (a verificação de um conector, a leitura de uso do Codex, os cards e as
  respostas de um telefone pareado, as ferramentas da web, o resumo do diário, as sugestões de
  memória e o sono do Zeca) pergunta antes ao portão de rede do app: só um recurso que você ligou,
  cada um com o próprio limite diário (veja a tabela), cada requisição guardada como uma linha no
  `vults.sqlite` (quando, qual recurso, se respondeu; 30 dias). Um conector que passa do limite
  espera e diz isso em Settings → Connectors.

O que ele guarda fica neste computador, nunca é enviado para lugar nenhum e está listado em
[Configurações e arquivos](reference/settings.md#where-things-live-linux); o
[Clear history](guide/activity.md#the-history) apaga quase tudo:

- **As edições que ele mostra ficam na memória**: as linhas alteradas de uma edição concluída (no
  máximo 400), das últimas 32 edições da sessão, e os caminhos dos arquivos que ela alterou com
  suas contagens, nunca gravados em um arquivo ou log.
- **Os conectores guardam só a última resposta**, para que um reinício não repita notícias antigas
  ([Conectores](guide/connectors.md#the-github-card)).
- **O histórico de atividade é só contagem**: nunca um prompt, uma resposta, um comando, um nome de
  arquivo ou um caminho ([Atividade](guide/activity.md#what-a-turn-is)).
- **Os turnos do Zeca ficam registrados** por 30 dias: motor, modelo, tokens, custo e o alvo
  redigido de cada ferramenta, nunca um prompt ou uma resposta ([O Nest](guide/nest.md#activity)).
- **As conversas do Zeca ficam guardadas**, as 200 mais recentes, nunca a resposta a um card nem uma
  regra *Always in this chat* ([Conversas](guide/chat.md#conversations)).
- **A memória do Zeca fica desligada até você ligá-la**, e só você a escreve: o que o Zeca, um
  modelo ou o servidor MCP dele adicionariam espera o seu sim, e o `CLAUDE.md` de um projeto só muda
  sobre o arquivo que você viu, depois de um backup datado ([A memória do Zeca](guide/memory.md)).
- **Toda resposta fica registrada.** Um log de auditoria guarda, por 90 dias, cada resposta a um
  card, do bando e do chat do Zeca: quem a deu (você, o seu telefone pareado, uma regra *Always*, o *Always in this chat* do
  chat, uma [política](guide/approvals.md#policies) com o consentimento dela, o
  [acesso total](guide/chat.md#who-answers-zecas-cards) do chat, ou o app quando o tempo de um card
  acabou ou o *Paused* o mandou para o terminal), o que ela fez (permitir, negar, sempre permitir,
  permitir nesta conversa, responder, mandar para o terminal, ou o tempo acabou), o agente, o nome da pasta do
  projeto, a ferramenta e o comando ou caminho exato, com tokens e valores `NOME=segredo` redigidos
  antes. Uma resposta dada no próprio terminal do agente não entra. Cada configuração de agente que
  o instalador escreve também fica. Uma linha nunca é alterada e nenhuma sai antes de 90 dias,
  exceto pelo **Clear history**, que deixa uma linha: que você o limpou, e quando. Fica sempre
  ligado.
- **O que você aceita fica guardado para sempre.** Cada política que você aceita é uma linha nos
  consentimentos do `vults.sqlite`: quando, sobre o que é, as palavras que você aceitou e um SHA-256
  do diff que você viu. Uma linha nunca é alterada nem apagada, e o **Clear history** não mexe nela:
  uma política nunca pode perder a prova de que você disse sim. Revogar desliga a política na hora;
  a linha fica, marcada como revogada. As palavras da linha são a própria política, e uma cujo diff
  não termina nessas palavras nunca responde.
- **Os segredos ficam no chaveiro do sistema**, e não há telemetria.

<a id="every-request-vults-makes"></a>

## Toda requisição que o Vults faz

Nada mais chega à rede. O CI confere: o `scripts/check-network.sh` falha num crate que depende de um
cliente HTTP, ou num arquivo que nomeia um ou inicia um comando de rede, fora do
`scripts/network-allowlist.txt`, que nomeia cada um e o motivo; e num webview que possa se conectar a
qualquer lugar além do próprio app.

| Requisição | Quem inicia | O que a liga | Onde |
|---|---|---|---|
| Um turno do chat por uma API (Anthropic, OpenAI, Gemini, OpenRouter, Groq, DeepSeek, Mistral, xAI; Ollama e LM Studio em `127.0.0.1`) | Você, a cada mensagem | O provedor e a chave que você escolheu; enviado só a esse provedor | `crates/chat/src/api.rs`, `openai.rs`, `providers.rs` |
| A lista de modelos de um provedor | Você, ao abrir **Settings → Chat & voice** com uma chave salva (ou um Ollama ou LM Studio local), ou com **Try again** | O provedor e a chave que você escolheu | `crates/chat/src/providers.rs` |
| Uma busca na web (a web do Zeca, **Settings → Chat & voice**, desligada até você ligar) | O modelo do chat por API, num turno que você mandou; pelo portão de rede (200 por dia, com as leituras de página) | Brave Search ou Tavily com a sua chave, ou o seu SearXNG | `crates/chat/src/web.rs`, `app/src/web_tools.rs` |
| A leitura de uma página (o mesmo interruptor) | O modelo do chat por API, num turno que você mandou; pelo portão de rede (os mesmos 200) | O site da página, só endereços públicos; o texto dela então vai para o seu provedor de API numa chamada própria, sem ferramentas | `crates/chat/src/web.rs`, `app/src/web_tools.rs` |
| Os cards e as respostas de um telefone pareado (**Settings → Phone**, desligado até você ligar) | O app: cada card que ele manda, e uma leitura da caixa de cada telefone a cada 25 s; pelo portão de rede (10.000 por dia) | O relay que você escolheu, que só vê mensagens seladas; em **Direto**, este computador escuta no endereço que você deu | `crates/relay/src/client.rs`, `app/src/phone.rs` |
| A lista de modelos do Codex | Você, abrindo o seletor de modelo do chat com o Codex (uma vez por execução) | O `codex` em que você fez login, pelo `model/list` do `codex app-server` | `crates/chat/src/codex_server.rs` |
| Um turno do chat pelo `claude` ou pelo `codex` | Você, a cada mensagem | O motor que você escolheu; a CLI fala com o próprio serviço no seu login | `crates/chat/src/claude.rs`, `codex.rs`, `codex_server.rs` |
| Uma sessão retomada (o `claude` ou o `codex app-server` na pasta existente dela) | Você, ao digitar `/continue <palavras>` no Chat; `/continue stop` para apenas essa execução | A CLI fala com o próprio serviço no seu login, no sandbox dela, cada comando perguntando antes | `app/src/continuation.rs`, `crates/chat/src/claude.rs`, `codex_server.rs` |
| A leitura de uso do Codex (`account/rateLimits/read`, pelo `codex app-server`) | O app, a cada 5 minutos e meio minuto depois que uma janela reinicia, nunca com a tela bloqueada | **Settings → General → Codex usage**, pelo portão de rede (400 por dia) | `app/src/usage.rs` |
| Um dia passado do diário do Zeca resumido (**Settings → Chat & voice → Memory → A summary of each day**, desligado até você ligar) | O app, numa passada de hora em hora: cada dia passado sem resumo desde que você ligou, até 3 por passada; pelo portão de rede (8 por dia) | O `claude` em que você entrou, o menor modelo dele, sem ferramentas e sem nenhuma das suas configurações | `crates/chat/src/summary.rs`, `app/src/journal.rs` |
| Um embedding de um resumo do diário ou da sua mensagem, para lembrar (**Recall by meaning**, desligado até você ligar) | O app, na passada de hora em hora do diário para cada resumo sem um, e antes de cada mensagem do chat | Um Ollama em `127.0.0.1` com o `nomic-embed-text`, nunca por um proxy; nada sai do computador | `crates/chat/src/embed.rs`, `app/src/recall.rs` |
| Sugestões de memória de uma conversa que você deixou (**Suggest what to remember**, desligado até você ligar) | O app, quando você começa uma conversa nova ou abre outra; pelo portão de rede (20 por dia), só com pelo menos 2 mensagens suas, nunca para uma que leu conteúdo de fora | O `claude` em que você entrou, o menor modelo dele, sem ferramentas e sem nenhuma das suas configurações; o que ele propõe espera o seu sim | `crates/chat/src/summary.rs`, `app/src/memory.rs` |
| O sono do Zeca (**Let Zeca sleep on it**, desligado até você ligar): o que ele lembra sobre você e entre projetos, e os resumos dos últimos 7 dias | O app, uma vez por dia a partir das 2h enquanto nada trabalha; pelo portão de rede (4 por dia) | O `claude` em que você entrou, o menor modelo dele, sem ferramentas e sem nenhuma das suas configurações; as junções dele esperam o seu sim | `app/src/sleep.rs`, `crates/chat/src/summary.rs` |
| A verificação de um conector (GitHub, pelo `gh`) | O app, no seu próprio ritmo | O interruptor de cada conector em **Settings → Connectors**, pelo portão de rede (3.000 por dia) | `crates/connectors/src/github.rs` |
| O seu calendário de contribuições do GitHub (pelo `gh`) | Você, ao abrir a aba GitHub de **Settings → Activity** | O conector do GitHub ligado | `crates/connectors/src/github.rs` |
| Um squash merge do seu pull request (pelo `gh`), com uma linha no rastro local | Você, ao clicar em **Squash and merge** na linha dele nas Connections da ilha, e depois em **Merge** | O conector do GitHub ligado; só o seu próprio pull request, aprovado, com os checks passando | `crates/connectors/src/github.rs`, `app/src/connectors.rs` |
| O download de um modelo de voz | Você, com um clique em **Settings → Chat & voice → Voice** | Nada até você clicar; do Hugging Face (`ggerganov/whisper.cpp`), conferido pelo SHA-256 | `crates/voice/src/models.rs` |
| O download do detector de fala (Silero VAD, menos de 1 MB) | O app, sozinho, fora do portão, enquanto a voz está ligada e ele falta | Um modelo de voz que você escolheu; do Hugging Face (`ggml-org/whisper-vad`), conferido pelo SHA-256 | `crates/voice/src/models.rs`, `app/src/voice.rs` |
| O uso do plano do Claude Code | O Claude Code, que o entrega à nossa status line | Nenhuma requisição nossa | o hook |
| A checagem de atualização | Ainda não existe (ADR 0019) | Vai ficar desligada até você ligar, pelo portão | — |
