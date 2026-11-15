# Arquitetura
<!-- source: 0c18485a5b3a -->

O acompanhamento de agentes flui num só sentido, por um único loop de execução:

```
agent ──hook JSON──▶ vults-hook ──protocol line──▶ ipc ──┐
GitHub (gh) ─────────────▶ connectors runtime ─────────────────┤
island clicks (Allow, open, jump…) ────────────────────────────┤
                                                                ▼
                                        core::reduce(state, input, now) ──▶ effects
                                                                │            (answer the hook,
                                                    State::view ▼             open a URL, jump)
                                                         island renders
```

<a id="layers"></a>

## Camadas

Os crates formam três camadas sobre uma pequena base. Um crate depende só da própria camada ou de uma
mais baixa; `scripts/check-layers.sh` verifica isso na CI (os testes podem ir além).

| Camada | Crates | Do que ela cuida |
|---|---|---|
| **Experience** | `app`, `platform`, `ui/` | Todas as superfícies: a ilha, o widget, as configurações, a bandeja, o Zeca e o bando |
| **Connect** | `connectors`, `chat`, `audio`, `voice`, `dictation`, `media`, `zeca`, `link`, `relay`, `evals`, `spike` | O que vai além dos agentes: GitHub, as CLIs e APIs do chat, o microfone, o que está tocando, um telefone pareado |
| **Core** | `core`, `protocol`, `peer`, `ipc`, `hook`, `agents`, `agent-config`, `store` | Sessões, eventos, aprovações e decisões: o que os agentes estão fazendo e o que o humano disse |
| base | `brand`, `secrets` | O nome, o chaveiro do sistema |

Só o `app` liga as camadas entre si: Connect nunca chama Experience, e Core não conhece nenhuma das duas.
Isso mantém barata uma superfície nova (ela desenha `State::view` e envia intents, [ADR 0006](../adr/0006-surfaces.md))
e mantém o Zeca opcional ([ADR 0005](../adr/0005-zeca.md)): ele usa Chat, Voice e
Connect, e nenhum deles sabe que ele existe. O corpo dele fica em Experience; o cérebro é o
crate `zeca` em Connect, que lê o bando mas nunca responde a uma permissão
([ADR 0005](../adr/0005-zeca.md)).

| Crate | Papel | Não pode usar |
|---|---|---|
| `brand` | O nome, o slug e o bundle id do app; gera `ui/src/brand.ts` | nada |
| `protocol` | Mensagens versionadas hook ↔ app, limites, nomes de socket e pipe | tokio, Tauri |
| `peer` | Verificações de mesmo usuário nas duas pontas da conexão | tokio, Tauri |
| `hook` | Lê o JSON de hook do agente, o encaminha, imprime a decisão no formato do agente | tokio, HTTP |
| `ipc` | Servidor assíncrono: limites de conexão, confirmar-e-depois-decidir, roteamento para o app | Tauri |
| `core` | Domínio puro: sessões, aprovações (cada card uma oferta no ledger: presa ao pedido exato, respondida uma vez, só no prazo), classes de ação, as linhas de auditoria, alertas, a view, cujos tipos TypeScript ele gera em `ui/src/view.gen.ts` (um teste verifica que está atualizado); relógio injetado | IO, async, Tauri |
| `agents` | O registro de todos os agentes (id, nome, forma no fio, instalação, como roda, o que consegue fazer; `ui/src/agents.gen.ts` é o gêmeo dele) e, por agente (Claude Code, Codex, Antigravity, OpenCode, Hermes Agent, Qwen Code, Copilot CLI, Cursor, Factory Droid, Kimi Code, qualquer outro): nomes de eventos, ferramenta → atividade, entradas de instalação, confiança do Codex; o trait `AgentDriver` para um agente que o próprio app roda (sem runtime) | Tauri, tokio |
| `agent-config` | Edições seguras das configurações dos agentes (JSON, TOML com os comentários mantidos, arquivos de plugin inteiros): leitura estrita, diff, fingerprint, backup, escrita atômica | Tauri |
| `store` | O banco local: um `vults.sqlite` (SQLite embutido), migrações numeradas, o histórico de turnos, o log de auditoria só de acréscimo, os consentimentos (nunca apagados), traces, as conversas do Zeca (com o que cada uma leu de fora e o seu contexto congelado), os telefones pareados e o maior contador com que cada um assinou, o diário do Zeca e os resumos diários com a busca de texto e por vetor, as propostas de memória esperando um sim, as noites dele, o que os agentes custaram por dia e por roost e o histórico textual de ditado com totais independentes por dia/aplicativo (migração 21), gravado conforme o prazo escolhido, além de dicionário/substituições/trechos do usuário (migração 22), independentes do prazo do Histórico | Tauri, async, a rede |
| `chat` | Chat pelas CLIs `claude` e `codex`, com pedidos de permissão, ou por uma API: Anthropic ou uma nuvem compatível com OpenAI com a chave do usuário, ou um Ollama / LM Studio local | Tauri |
| `zeca` | O cérebro do Zeca (ADR 0005): uma visão só de leitura do bando, a memória dele como um repositório git de pequenos arquivos Markdown de fatos, cada um com um limite rígido (cada escrita um commit) | `core` (então nada nele consegue construir um `Intent`), Tauri |
| `link` | O elo com um telefone pareado (ADR 0002): pareamento por QR, envelopes selados com X25519 e XChaCha20-Poly1305 (um relay só vê texto cifrado), respostas assinadas pelas chaves do Keystore do telefone e verificadas antes que o ledger do `core` as resgate | IO, a rede, Tauri |
| `relay` | O relay do telefone (ADR 0002): um servidor axum que guarda e repassa envelopes selados por caixa, com limites e descartados depois de 10 minutos, e o cliente que as duas pontas usam, cada pedido pelo gate de rede do app; depende só do `link` | Tauri, `core` |
| `evals` | Evals com cassettes (só para desenvolvimento, nunca no app): classes de política, redação, os prompts enviados aos modelos, streams gravados repetidos para injeção | ser ligado ao app |
| `spike` | O kit de spike (só para desenvolvimento, nunca no app): um modelo falso, um app falso no socket do hook e um terminal para as TUIs, para gravar os hooks de um agente de verdade sem conta; o `scripts/spike.sh` roda o kit para cada agente | ser ligado ao app |
| `secrets` | O chaveiro do sistema (Secret Service, Credential Manager), indexado pelo bundle id | Tauri, arquivos |
| `connectors` | Vults Connect: o trait `Connector`, o runtime de polling, GitHub | Tauri, core |
| `audio` | Captura do microfone em sua própria thread (cpal), dispositivo por nome com alternativa padrão e monitores ignorados, mono a 16 kHz limitado na memória (rubato), nível escalar e nove faixas de fala (rustfft); a captura opcional de reuniões no Linux mantém WAVs temporários privados; nenhuma transcrição | Tauri, core, transcrição, rede |
| `voice` | Push-to-talk do Chat sobre `audio`, limite de um minuto por gravação, fim de fala pelo Silero e whisper.cpp/prévia ao vivo neste computador, downloads de modelos conferidos por SHA-256; transcrição local de WAVs de reuniões em blocos limitados, com horários por segmento, cancelamento e prioridade para a fala interativa | Tauri, core |
| `dictation` | Gestos puros com tempo monotônico injetado (segurar, toque duplo, travar com Espaço, cancelar e prazo de 600 ms de áudio adicional), comparação de cancelamento falado, pontuação de código falado em contexto de terminal, gramática de comandos de voz e resolução de nomes de projetos/agentes | IO, async, Tauri, core, a rede |
| `media` | O que está tocando (MPRIS pelo session bus, pelos sinais dele) e tocar/pausar/pular | Tauri, core |
| `platform` | Superfícies/regiões de entrada no Linux, bandeja, atalhos globais, pular para o terminal, capacidades de foco, reservas da área de transferência, inserção protegida por XTest/portal/tmux e mudanças temporárias de volume/reprodução MPRIS | Tauri, core |
| `app` | O shell Tauri: o loop do runtime, effects, comandos, bandeja, configurações | — |

A UI (`ui/`) é TypeScript sem framework. `src/bridge.ts` é o único arquivo que fala com o Tauri;
`src/island/` renderiza a ilha a partir da view, `src/character/` desenha as aves a partir dos dados de sprite,
e `src/surfaces/` guarda todas as outras superfícies, uma pasta cada (`widget/`, `settings/`, `nest/`) com a sua
entrada `main.ts` e a sua folha de estilo. O código usado por mais de uma superfície fica na raiz de `src/`;
as exceções são o painel do chat (`src/island/chat.ts`, com o Markdown e os ícones dele) e o card do
GitHub (`src/island/board.ts`), que o Nest também desenha, e o visual das páginas das configurações
(`surfaces/settings/pages.css`), que a Week do Nest usa dentro de uma shadow root.

<a id="voice-and-dictation"></a>

## Voz e ditado

Todo recurso de voz compartilha um único caminho local: o `audio` captura na memória, o `voice`
transcreve com whisper.cpp, e o crate puro `dictation` transforma texto em texto ou num comando
tipado. O `platform` cuida de tudo que toca o desktop (atalhos, a janela em foco, a área de
transferência, a inserção, mídia, saída de áudio do sistema, abrir aplicativos); o `app` orquestra e
é dono do gate de rede. O comportamento para o usuário está em [Ditado](../guide/dictation.md),
[Voz](../guide/voice.md) e [Reuniões](../guide/meetings.md).

**Um ditado** começa no aperto da tecla com um ticket que congela o modo (texto ou comando), o
destino capturado e um retrato das configurações e das regras de Suas palavras. Depois da
transcrição local, o texto passa, nesta ordem, por: cancelamento falado, Suas palavras
(`dictation::text`), código falado, o pedido opcional de limpeza (`dictation::cleanup`, pelo gate) e
a entrega. A entrega confere o destino de novo logo antes de agir (identidade da janela e época do
foco no X11 ou por um script somente leitura do KWin, painel e processo no tmux) e pergunta ao
`core` o estado atual da sessão: uma sessão que espera um card nunca recebe texto. O que não puder ser
provado cai para a área de transferência. O histórico guarda só texto, dentro da retenção escolhida.

**Um comando** usa a mesma captura e transcrição, e depois o `dictation::command`: uma gramática
limitada (4 KiB, oito comandos) em inglês, português e espanhol, e um resolvedor de nomes sobre um
retrato de projetos, agentes e apelidos. O `app::voice_commands` planeja o lote sobre um clone do
estado; só as intenções de `Intent::voice_command_allowed` podem sair dele, o que exclui toda resposta
a card, liberação, revisão e merge. Ações nativas (mídia, saída do sistema, abrir aplicativo, foco de
terminal) são conferidas juntas antes, revalidadas uma a uma, e informam só os passos confirmados:
sem nova tentativa, sem desfazer.

**Uma conversa por voz** mantém a captura e o Whisper locais e acrescenta três responsáveis. O
`voice::turn` é uma máquina de turnos pura (fim da fala, interrupção, contagem do texto ouvido). O
`chat::realtime` é o provedor e o preço dele. O `store::money` reserva um valor em micro-USD antes de
tocar qualquer chave ou socket e o liquida uma vez; uma reserva nunca se devolve sozinha. As
transcrições finais locais passam pelo despachante de comandos antes de qualquer envio, e o realtime
só recebe ferramentas somente leitura do bando em cache.

**Uma reunião** grava dois WAVs temporários privados (`meeting-audio/`, 0700 e 0600), os transcreve
depois do Parar em blocos que cedem a vez ao ditado interativo, e os apaga em qualquer resultado e na
abertura do app. Análise de quem fala e resumos são opções à parte.

Invariantes que toda mudança mantém, cada uma com um teste:

- Nenhum caminho de voz chega a `Intent::Decide` ou `DecideAlways` (ADR 0002).
- O áudio fica na memória, exceto os arquivos temporários de uma reunião; sem palavra de ativação,
  sem microfone sempre ligado (ADR 0004).
- Toda chamada de modelo sobre áudio ou texto passa pelo gate, com chave, orçamento e rastro próprios.
- Um ticket atrasado ou substituído nunca executa nem entrega nada.
- Texto falado nunca vira texto de comando de shell ou de tmux, e Enter nunca é enviado.

Os registros de engenharia por trás desses responsáveis (preços e tetos, as rotas de cancelamento de
eco, o modelo semântico de fim de turno, os artefatos dos modelos de reunião e a avaliação de quem
fala) ficam em `docs/dev/internals/`.

<a id="meaning-in-core-look-in-the-surface"></a>

## Significado no core, aparência na superfície

A view diz o que as coisas significam, para que todas as superfícies concordem ([ADR 0006](../adr/0006-surfaces.md)).
Cada sessão carrega a sua `attention`, um nível ordenado (`quiet` < `info` < `done` < `failed` <
`needs-you`), e a view carrega o maior deles; `card` marca a sessão cujo card é o primeiro
da fila e ainda espera por ele. A ilha só escolhe a aparência: que som cada nível faz, e
quanto tempo um estado precisa durar (1,5 s, 3 s para `done`) antes de virar novidade.

O runtime transmite a view, quando ela muda, para todas as janelas de uma vez (`publish_view`), e
guarda a última para uma página que carregue depois (`current_view`). Eventos destinados só à ilha
(ponteiro, atalhos, chat) são enviados para o label dela. Um `listen` comum ouve todos os alvos, então uma
superfície nova assina esses eventos pelos helpers com escopo de janela em `bridge.ts`
(`getCurrentWebviewWindow().listen`) para ficar fora deles.

A janela do Nest tem dois webviews, o do Nest e o das Configurações, um visível por vez sob o mesmo
cabeçalho (ADR 0006). Os comandos conferem o label do webview que chamou: as chamadas que só as
Configurações podem fazer são recusadas vindas do webview do Nest, que mostra texto de agente.

`ended` lista os cards que saíram da fila mais recentemente, cada um com o seu `outcome`: respondido aqui
(`allowed`, `denied`, `answered`), `released` (enviado daqui para o terminal), `terminal` (o
agente seguiu em frente), `expired` ou `rule` (um Always num card idêntico). A ilha lê isso para dizer o que aconteceu com o card que
ela mostrou. Um outcome é só informação: ele é registrado onde um card sai da fila e nunca
responde a um.

`front` é a sessão em primeiro plano, por uma única regra: a sessão cujo card espera, senão a que o usuário
pôs em primeiro plano (`focus`, definida por `Intent::Focus` a partir de um clique na linha dela, esquecida quando a sessão
sai), senão a primeira. O trabalho sozinho nunca a muda. As sessões vêm na ordem em que chegaram, a
ordem em que todas as superfícies as desenham, e a que `Intent::FocusNext` e `FocusPrevious` (atalhos
globais) percorrem, dando a volta. O chat trabalha na pasta da sessão em primeiro plano.

<a id="why-the-hook-waits-for-an-acknowledgement"></a>

## Por que o hook espera uma confirmação

Um pedido de permissão mantém a conexão aberta. O servidor só espera por um humano depois que o loop
do app pegou o pedido e o `core` enfileirou o card dele (`Effect::AckPermission`). Se o app estiver
travado ou não estiver escutando, o agente recebe a resposta (silêncio, então ele pergunta no terminal) em
800 ms em vez de dois minutos. A ilha abre num card enfileirado e fica aberta até ele ser
respondido; nenhum modo de presença pode escondê-lo depois de confirmado ([ADR 0006](../adr/0006-surfaces.md)).

<a id="why-only-core-produces-a-decision"></a>

## Por que só o `core` produz uma decisão

`core::reduce` transforma um evento `PermissionRequested` em estado pendente, e só o clique de um humano
transforma estado pendente num effect `RespondPermission`: `Intent::Decide` (Allow, Deny) ou
`Intent::DecideAlways`, que também salva uma regra. Uma regra salva só responde a um pedido que bate com
ela exatamente: o mesmo agente, pasta, ferramenta e alvo ([ADR 0002](../adr/0002-permissions.md)). Uma pergunta
(`QuestionAsked`) espera do mesmo jeito, e só um `Intent::Answer` que se encaixe nas perguntas dela a transforma
num effect `AnswerQuestion`. Um teste alimenta
todas as outras entradas em todas as ordens e verifica que nenhuma delas responde.

A interface Linux de perguntas de texto `ask_user` do Zeca é separada desse caminho de respostas do
hook. Um socket privado do mesmo usuário conecta o relay stdio ao runtime do app. O PID do par obtido
do kernel, a identidade do executável, o início dos processos e a ancestralidade retida vinculam o
pedido a um cliente ativo verificado; cabeçalhos fornecidos pelo cliente não comprovam a identidade.
Clientes iniciados pelo app usam o servidor de chat existente. Clientes externos usam
`--questions-mcp`, que expõe somente ferramentas de perguntas, e precisam do consentimento separado
em Ditado > Agentes, desligado por padrão. A geração do consentimento e o horário de recebimento do
transporte recusam pedidos antigos enfileirados após uma nova concessão. As travas de configurações
e consentimento permanecem presas durante as transições do runtime, incluindo a entrega da resposta,
impedindo uma revogação entre a leitura do acesso e a entrega. O runtime possui o `core::questions::Slot`, seu nonce novo por conexão, prazo
absoluto, concessão da superfície e confirmação única do rascunho. Ele adia a entrega até a auditoria
sem conteúdo ser persistida, enquanto continua processando invalidações. O relay confirma após
descarregar a resposta MCP no fluxo; falha/perda de confirmação nunca reutiliza o texto consumido.
Cards de permissão têm prioridade. Só o webview dono da ilha/Nest pode editar ou confirmar; bloquear,
navegar, sair, expirar e desconectar invalidam o trabalho pendente e conclusões tardias.
A regra do rascunho de pergunta da [ADR 0002](../adr/0002-permissions.md) (um rascunho ditado só é enviado por um clique) também
é implementada por uma gravação local iniciada somente pela tecla física, vinculada ao token da
pergunta ativa e ao identificador da gravação. `vults_dictate` é um alias da mesma troca tipada de
`ask_user`. A captura usa o microfone e o transcritor local em cache existentes, com verificações de
admissão e duração e sem passar por provedor, download, área de transferência ou histórico. O resultado
é acrescentado ao editor atual; uma confirmação de aplicação correspondente precede o Enviar humano
separado. Cancelar e cada transição do ditado avançam a revisão da tela, impedindo que snapshots
tardios restaurem palavras canceladas. Captura/transcrição nativas nunca bloqueiam o loop do runtime. A admissão do worker comum e o registro
do gravador do Chat compartilham a trava de roteamento; concessões de microfone revogadas não voltam
a valer após fechar uma pergunta. Um Status correspondente do renderer precisa confirmar a exibição
da pergunta antes que a captura possa selecioná-la.
Nenhum texto de pergunta vira intent de permissão, regra ou política. Somente Configurações pode
mostrar/aplicar a configuração MCP exclusiva de perguntas do Claude Code/Codex. Um plano em cache
vincula agente, executável, escolha de instalação/remoção, diff exato e fingerprint. `agent-config`
fornece leitura estrita, backup datado e gravação atômica; entradas antigas/estrangeiras são recusadas.
A entrada gerenciada pode acompanhar a mudança de caminho do executável, preservando os demais
servidores, hooks e a confiança. Gerações de leitura da UI, versões de eventos de consentimento e
gerações de saída impedem a volta de respostas antigas de configuração, estado ou prévia.

<a id="the-island-on-wayland"></a>

## A ilha no Wayland

A ilha é uma superfície layer-shell mapeada uma vez, num tamanho fixo, e nunca redimensionada nem escondida: o KWin
deixa de mostrar uma layer surface redimensionada a partir do webview. A UI desenha a ilha dentro dela e informa
o retângulo dela, que vira a única parte que recebe o mouse (a região de entrada); todo o resto
passa para as janelas abaixo. O WebKit pausa `requestAnimationFrame` enquanto acha que a página está
escondida, então a UI mede o DOM de forma síncrona e anima com timers.

O código de plataforma é por janela: cada superfície tem um `LayerSpec` (namespace, tamanho fixo, as bordas
de que ela pende, margem, teclado) e a própria região de entrada, guardados pelo label da janela. Os pedidos de `layout`
e de teclado de uma página agem na janela que os enviou, e só se ela for uma superfície
(`SURFACES` em `app/src/lib.rs`). Uma superfície nova ganha um spec, o seu label em `windows` de `app/capabilities/default.json` e, para
a própria página, uma entrada no Vite.

<a id="trying-the-island-without-an-agent"></a>

## Testando a ilha sem um agente

`cargo run -p vults-hook --example replay` envia uma sessão gravada pelo hook de verdade para
o app em execução: leitura, busca, a web, uma edição, uma permissão (ela espera a sua resposta, como um
agente faria), um comando e o fim. Passe o seu próprio arquivo JSONL (um JSON de hook por linha) e
`--delay-ms` ou `--agent codex` para mudá-la. O próprio aplicativo é a superfície de validação da UI.

<a id="decisions"></a>

## Decisões

Por que o app é construído assim (só Rust, as regras de segurança, um core para todas as superfícies, Zeca
opcional, regras antes de modelos) está nos [registros de decisão](../adr/README.md).

<a id="documentation"></a>

## Documentação

`docs/` é a fonte da documentação no site
([rogeriojunior31.github.io/en/docs/vults](https://rogeriojunior31.github.io/en/docs/vults/)),
publicada a cada tag de release; `docs/site.json` guarda o nome e o resumo que o site mostra. As páginas começam com um
`# H1`, usam links relativos e guardam as imagens em `docs/assets/`. `docs/adr/` guarda os registros de decisão. `docs/dev/` é interno e não é
publicado; `docs/pt-br/` guarda a tradução em português do Brasil (veja o CONTRIBUTING).
