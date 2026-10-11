# Segurança
<!-- source: cd826289d9ce -->

O que o Vults promete:

- **Ele nunca bloqueia seu agente.** Se o app estiver fechado, lento ou quebrado, o hook sai em
  milissegundos sem imprimir nada, e seu agente pergunta no terminal como sempre.
- **Só um clique aprova uma chamada de ferramenta.** Nenhum timer ou padrão responde a um pedido de permissão; a
  única exceção é uma regra que você mesmo criou com **Always**, para aquele comando ou arquivo exato.
  Uma pergunta do Claude Code só é respondida com a escolha ou as palavras que você deu no card dela.
- **Só a sua conta de usuário fala com ele.** O socket fica na sua pasta de runtime privada, com modo
  `0600`, e os dois lados verificam se o outro processo roda como você (no Windows, pelo SID).
- **Ele nunca edita a configuração de um agente pelas suas costas.** Toda mudança ganha um backup datado e um diff que você
  aprova; os hooks de outras ferramentas são mantidos, e uma status line do Claude Code que seja sua é guardada ao lado do hook, continua rodando e volta como estava quando você remove a nossa; o `trusted_hash` do Codex nunca é escrito (você mesmo confia nos nossos hooks, no `/hooks` do Codex).
- **O uso do plano vem só das CLIs.** A entrada da status line do Claude Code é reduzida no hook
  aos `rate_limits` dela antes de qualquer coisa sair dele; o Codex é consultado com uma chamada só de leitura,
  só enquanto **Settings → General → Codex usage** estiver ligado.
- **O chat usa as CLIs em que você fez login** (`claude`, `codex`); o Vults nunca lê as
  credenciais delas. Elas leem a sua própria configuração (seu `CLAUDE.md`, skills, servidores MCP,
  hooks e regras de permissão, que valem em qualquer pasta, como no seu terminal), e as
  configurações e os servidores do `.mcp.json` de um projeto só numa pasta em que você já confiou no
  Claude Code. No chat, todo comando e toda edição que suas regras não liberam esperam o
  seu Allow, seja qual for o `defaultMode` das suas configurações; um card que ninguém responde é um
  não. Arquivos que você solta nele são copiados para uma caixa de entrada e apagados depois de uma semana.
- **As chaves de API ficam no chaveiro do sistema.** Se você der ao chat a chave de API de um provedor, ela é gravada
  só no chaveiro (Secret Service no Linux), nunca em um arquivo ou log, nunca devolvida a uma
  janela, e só é enviada para aquele provedor. Modelos locais (Ollama, LM Studio) são acessados só em
  `127.0.0.1`. O chat pela API não tem ferramentas: ele não executa comandos nem edita arquivos.
- **O microfone só escuta enquanto você segura uma gravação.** A voz fica desligada até você baixar um
  modelo; depois o mic grava só na memória enquanto o botão dele está vermelho (no máximo um minuto), e
  o whisper.cpp transcreve neste computador. O áudio nunca é salvo nem enviado para lugar nenhum.
- **O Now playing fica desligado até você ligar.** Aí o app lê seus players de mídia pelo
  session bus (MPRIS no Linux) para mostrar a música; isso nunca é guardado nem enviado para lugar nenhum.
- **As edições que ele mostra ficam na memória.** Para o diff da ilha, o hook repassa as linhas
  alteradas de uma edição concluída (no máximo 400); o app guarda só as dos últimos passos da sessão, e nunca
  as grava em um arquivo ou log.
- **O que o app pede por conta própria passa por um portão só.** Uma requisição feita sem um
  clique naquele momento (a verificação de um conector, como a do GitHub, e a leitura de uso do
  Codex) pergunta antes ao portão de rede do app: só um recurso que você ligou, no máximo 3.000
  verificações em quaisquer 24 horas para um conector e 400 para a leitura de uso, cada uma
  guardada como uma linha no `vults.sqlite` (quando, qual recurso, se respondeu; 30 dias). Um
  conector que passa disso espera e diz isso em Settings → Connectors.
- **Os conectores guardam só a última resposta.** A última verificação do GitHub (títulos, links, estados de checks e
  reviews dos seus pull requests abertos e repositórios recentes) é salva na pasta de dados do app,
  `connectors/github.json`, para que um reinício não repita notícias antigas. O seu calendário de
  contribuições, para Settings → Activity, só é pedido quando a aba dele abre e fica uma hora na
  memória. Nada mais é guardado, e isso nunca é enviado para lugar nenhum.
- **O histórico de atividade é só contagem, neste computador.** Para Settings → Activity, cada turno
  terminado de um agente fica na pasta de dados (`vults.sqlite`: cada turno por 12 semanas, os totais
  de cada dia por um ano): quando terminou, quanto durou, o agente, o nome da pasta do projeto e
  contagens (passos, comandos, arquivos e linhas mudados, as suas respostas). Nunca um prompt, uma
  resposta, um comando, um nome de arquivo ou um caminho. Nunca é enviado para lugar nenhum; desligue
  ou limpe em Settings → Activity.
- **Os turnos do Zeca ficam registrados, neste computador.** Cada turno do chat e cada ferramenta
  que ele pediu para rodar é um span no `vults.sqlite`, guardado por 30 dias: o motor e o modelo,
  quando começou e terminou, os tokens de entrada e saída, o cache do prompt, o custo quando o motor
  informa, e o nome e o alvo da ferramenta com segredos redigidos. Nunca um prompt ou uma resposta.
  Nada disso sai do computador; o **Clear history** também apaga.
- **Toda resposta fica registrada, neste computador.** Um log de auditoria no mesmo `vults.sqlite`
  guarda, por 90 dias, cada resposta a um card, do bando e do chat do Zeca: quem a deu (você, uma
  das suas regras *Always* ou o *Sempre nesta conversa* do chat, ou o app quando o tempo de um card
  acabou ou o *Paused* o mandou para o terminal), o que ela fez (permitir, negar, permitir sempre,
  permitir nesta conversa, responder, mandar para o terminal), o agente, o nome da pasta do
  projeto, a ferramenta e o comando ou caminho exato de que se tratava,
  com tokens e valores `NOME=segredo` redigidos antes (nunca o que vem depois deles). Uma resposta
  dada no próprio terminal do agente não é do Vults, e não entra. Cada configuração de agente que o
  instalador escreve também fica. Uma linha nunca é alterada, e nenhuma sai antes de 90
  dias, exceto pelo **Clear history** em Settings → Activity, que a apaga junto com o histórico e
  deixa uma linha: que você o limpou, e quando. Fica sempre ligado, e nunca é enviado para lugar
  nenhum.
- **O que você aceita fica guardado para sempre, neste computador.** Cada política que você aceita
  (ainda não dá para criar nenhuma; elas chegam com o motor de políticas) é uma linha nos
  consentimentos do `vults.sqlite`: quando, sobre o que é, as palavras que você aceitou e um SHA-256
  do diff que você viu. Uma linha nunca é alterada nem apagada, nada a poda, e o **Clear history**
  não mexe nela: uma política nunca pode perder a prova de que você disse sim. Revogar é o único
  jeito de desfazer, e desliga a política na hora; a linha fica, marcada como revogada. Nunca é
  enviado para lugar nenhum.
- **Os segredos ficam no chaveiro do sistema**, e não há telemetria.

<a id="every-request-vults-makes"></a>

## Toda requisição que o Vults faz

Nada mais chega à rede. O CI confere: o `scripts/check-network.sh` falha num crate que depende de um
cliente HTTP, ou num arquivo que nomeia um ou inicia um comando de rede, fora do
`scripts/network-allowlist.txt`, que nomeia cada um e o motivo; e num webview que possa se conectar a
qualquer lugar além do próprio app.

| Requisição | Quem inicia | O que a liga | Onde |
|---|---|---|---|
| Um turno do chat por uma API (Anthropic, OpenAI, Gemini, OpenRouter, Groq, DeepSeek, Mistral, xAI; Ollama e LM Studio em `127.0.0.1`) | Você, a cada mensagem | O provedor e a chave que você escolheu; enviado só a esse provedor | `crates/chat/src/providers.rs` |
| A lista de modelos de um provedor | Você, ao abrir **Settings → Chat** com uma chave salva (ou um Ollama ou LM Studio local), ou com **Try again** | O provedor e a chave que você escolheu | `crates/chat/src/providers.rs` |
| A lista de modelos do Codex | Você, abrindo o seletor de modelo do chat com o Codex (uma vez por execução) | O `codex` em que você fez login, pelo `model/list` do `codex app-server` | `crates/chat/src/codex_server.rs` |
| Um turno do chat pelo `claude` ou pelo `codex` | Você, a cada mensagem | O motor que você escolheu; a CLI fala com o próprio serviço no seu login | `crates/chat/src/claude.rs`, `codex.rs`, `codex_server.rs` |
| A leitura de uso do Codex (`account/rateLimits/read`, pelo `codex app-server`) | O app, a cada 5 minutos, nunca com a tela bloqueada | **Settings → General → Codex usage**, pelo portão de rede (400 por dia) | `app/src/usage.rs` |
| A verificação de um conector (GitHub, pelo `gh`) | O app, no seu próprio ritmo | A chave de cada conector em **Settings → Connectors**, pelo portão de rede (3.000 por dia) | `crates/connectors/src/github.rs` |
| O seu calendário de contribuições do GitHub (pelo `gh`) | Você, ao abrir a aba GitHub de **Settings → Activity** | O conector do GitHub ligado | `crates/connectors/src/github.rs` |
| O download de um modelo de voz | Você, com um clique em **Settings → Chat** | Nada até você clicar; o arquivo é conferido pelo SHA-256 | `crates/voice/src/models.rs` |
| O uso do plano do Claude Code | O Claude Code, que o entrega à nossa status line | Nenhuma requisição nossa | o hook |
| A checagem de atualização | Ainda não existe (ADR 0016) | Vai ficar desligada até você ligar, pelo portão | — |
