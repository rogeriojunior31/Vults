# Arquitetura
<!-- source: be269b258c5a -->

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
Isso mantém barata uma superfície nova (ela desenha `State::view` e envia intents, [ADR 0021](adr/0021-four-sizes-one-nest.md))
e mantém o Zeca opcional ([ADR 0018](adr/0018-zeca-as-an-agent.md)): ele usa Chat, Voice e
Connect, e nenhum deles sabe que ele existe. O corpo dele fica em Experience; o cérebro é o
crate `zeca` em Connect, que lê o bando mas nunca responde a uma permissão
([ADR 0018](adr/0018-zeca-as-an-agent.md)).

| Crate | Papel | Não pode usar |
|---|---|---|
| `brand` | O nome, o slug e o bundle id do app; gera `ui/src/brand.ts` | nada |
| `protocol` | Mensagens versionadas hook ↔ app, limites, nomes de socket e pipe | tokio, Tauri |
| `peer` | Verificações de mesmo usuário nas duas pontas da conexão | tokio, Tauri |
| `hook` | Lê o JSON de hook do agente, o encaminha, imprime a decisão no formato do agente | tokio, HTTP |
| `ipc` | Servidor assíncrono: limites de conexão, confirmar-e-depois-decidir, roteamento para o app | Tauri |
| `core` | Domínio puro: sessões, aprovações (cada card uma oferta no ledger: presa ao pedido exato, respondida uma vez, só no prazo), classes de ação, as linhas de auditoria, alertas, a view, cujos tipos TypeScript ele gera em `ui/src/view.gen.ts` (um teste verifica que está atualizado); relógio injetado | IO, async, Tauri |
| `agents` | O registro de todos os agentes (id, nome, forma no fio, instalação, como roda, o que consegue fazer; `ui/src/agents.gen.ts` é o gêmeo dele) e, por agente (Claude Code, Codex, Antigravity, OpenCode, Qwen Code, Copilot CLI, Cursor, Factory Droid, Kimi Code, qualquer outro): nomes de eventos, ferramenta → atividade, entradas de instalação, confiança do Codex; o trait `AgentDriver` para um agente que o próprio app roda (sem runtime) | Tauri, tokio |
| `agent-config` | Edições seguras das configurações dos agentes (JSON, TOML com os comentários mantidos, arquivos de plugin inteiros): leitura estrita, diff, fingerprint, backup, escrita atômica | Tauri |
| `store` | O banco local: um `vults.sqlite` (SQLite embutido), migrações numeradas, o histórico de turnos, o log de auditoria só de acréscimo, os consentimentos (nunca apagados), traces, as conversas do Zeca (com o que cada uma leu de fora e o seu contexto congelado), os telefones pareados e o maior contador com que cada um assinou, o diário do Zeca e os resumos diários com a busca de texto e por vetor, as propostas de memória esperando um sim, as noites dele, o que os agentes custaram por dia e por roost e o histórico textual de ditado com totais independentes por dia/aplicativo (migração 21), gravado conforme o prazo escolhido, além de dicionário/substituições/trechos do usuário (migração 22), independentes do prazo do Histórico | Tauri, async, a rede |
| `chat` | Chat pelas CLIs `claude` e `codex`, com pedidos de permissão, ou por uma API: Anthropic ou uma nuvem compatível com OpenAI com a chave do usuário, ou um Ollama / LM Studio local | Tauri |
| `zeca` | O cérebro do Zeca (ADR 0018): uma visão só de leitura do bando, a memória dele como um repositório git de pequenos arquivos Markdown de fatos, cada um com um limite rígido (cada escrita um commit) | `core` (então nada nele consegue construir um `Intent`), Tauri |
| `link` | O elo com um telefone pareado (ADR 0004): pareamento por QR, envelopes selados com X25519 e XChaCha20-Poly1305 (um relay só vê texto cifrado), respostas assinadas pelas chaves do Keystore do telefone e verificadas antes que o ledger do `core` as resgate | IO, a rede, Tauri |
| `relay` | O relay do telefone (ADR 0004): um servidor axum que guarda e repassa envelopes selados por caixa, com limites e descartados depois de 10 minutos, e o cliente que as duas pontas usam, cada pedido pelo gate de rede do app; depende só do `link` | Tauri, `core` |
| `evals` | Evals com cassettes (só para desenvolvimento, nunca no app): classes de política, redação, os prompts enviados aos modelos, streams gravados repetidos para injeção | ser ligado ao app |
| `spike` | O kit de spike (só para desenvolvimento, nunca no app): um modelo falso, um app falso no socket do hook e um terminal para as TUIs, para gravar os hooks de um agente de verdade sem conta; o `scripts/spike.sh` roda o kit para cada agente | ser ligado ao app |
| `secrets` | O chaveiro do sistema (Secret Service, Credential Manager), indexado pelo bundle id | Tauri, arquivos |
| `connectors` | Vults Connect: o trait `Connector`, o runtime de polling, GitHub | Tauri, core |
| `audio` | Captura do microfone em sua própria thread (cpal), dispositivo por nome com alternativa padrão e monitores ignorados, mono a 16 kHz limitado na memória (rubato), nível escalar e nove faixas de fala (rustfft); nenhuma transcrição ou áudio gravado em disco | Tauri, core, transcrição, rede |
| `voice` | Push-to-talk do Chat sobre `audio`, limite de um minuto por gravação, fim de fala pelo Silero e whisper.cpp/prévia ao vivo neste computador, downloads de modelos conferidos por SHA-256 | Tauri, core |
| `dictation` | Gestos puros com tempo monotônico injetado (segurar, toque duplo, travar com Espaço, cancelar e prazo de 600 ms de áudio adicional), comparação de cancelamento falado e pontuação de código falado em contexto de terminal | IO, async, Tauri, core, a rede |
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

<a id="local-dictation"></a>

## Ditado local

O ditado independente tem seu próprio worker cancelável no `app`; não passa amostras do microfone
nem transcrições pelo loop de eventos dos agentes. O roteador de atalhos liga cada soltura de tecla
a quem recebeu o pressionamento. O Chat do Vults em foco com o Zeca ligado usa sua entrada de voz
no rascunho; aplicativos externos e superfícies com o Zeca desligado usam ditado independente.
Configurações não inicia ditado; seu diagnóstico explícito de microfone tem uma captura separada e limitada.

```text
desktop talk shortcut -> app::shortcut_route -> app::dictation worker (generation)
                                                  |
                              audio::Capture (16 kHz mono, <= 10 min, memory only)
                                  |                               |
                          voice::Preview                     finish + 600 ms tail
                          (supported GPU)                         |
                                  |                    Silero + whisper + text filter
                                  |                               |
                                  |                    optional installed-Base retry
                                  |                               |
                                  |                    spoken cancel -> discard
                                  |                               |
                                  |                    local words + context-bound code
                                  |                               |
                                  |                    guarded delivery or clipboard
                                  |                               |
                                  +--> ordered feedback <-- confirmed outcome
                                      island / Nest               |
                                                         dictation history + totals
                                                          (store, retention choice)
```

Cada turno captura suas escolhas de microfone e processamento. Cancelar invalida trabalhos
atrasados de captura, prévia, decodificação e entrega; reutilizar usa o texto já entregue e nunca
repete conversão ou inserção. Áudio, texto anterior da área de transferência e títulos de janelas
ficam na memória. O texto ouvido/escrito retido e os metadados reconhecidos de aplicativo/sessão
só entram no banco após uma entrega bem-sucedida. Apagar transcrições preserva os totais medidos
por dia/aplicativo.

A migração 23 acrescenta tempos opcionais por etapa, em milissegundos, aos ditados entregues e
retidos. O worker mede com relógio monotônico o total do pedido de término até a entrega (incluindo
a cauda do microfone e a fila), STT com carregamento/nova tentativa do modelo, processamento local
do texto e entrega confirmada. A limpeza permanece sem medição até ser implementada.
`store::dictations` resume cada etapa conhecida com p50/p95 por ordem de classificação mais próxima
entre as 50 entregas retidas mais recentes, sem carregar texto. O comando de histórico expõe
entradas e resumo; cancelamentos, falhas/resultados vazios e reutilização não criam amostras de
tempo. Os tempos seguem a retenção/exclusão das transcrições; os totais independentes de uso por
dia/aplicativo não mudam.

`app::doctor::dictation` expõe diagnóstico exclusivo de Configurações usando os responsáveis existentes
por atalhos/teclado, o formato da entrada de áudio, sondagens locais de GPU/VAD e uma leitura apenas
dos tempos no banco. A decodificação final informa repetições removidas sem contar prévia/Chat;
contadores na memória também registram novas tentativas com Base e resultados Copiado. O teste separado
de seis segundos mede o ruído RMS dos dois segundos iniciais em dBFS e nove bandas, sem transcrição nem
acesso a disco/rede/clipboard. Identificadores de teste rejeitam consultas/paradas antigas, o fechamento
da captura serializa reinícios e a ausência de contato de Configurações por dois segundos cancela o teste.
Mudanças nativas de janela/webview, bloqueio de tela e início de voz também o cancelam. Sondagens
desconhecidas e falhas de leitura dos tempos não viram zeros de sucesso. A interface preserva o responsável
entre redesenhos de Configurações, consulta só enquanto visível e atualiza as bandas sem recriar a ação
Parar que está com foco.

A migração 24 acrescenta `dictation_app_days` à transação de uso existente, incluindo aplicativos
desconhecidos. As estatísticas leem totais diários e aplicativos num único snapshot do SQLite,
independentemente das transcrições retidas. Os totais antigos de todo o período continuam exatos;
períodos limitados indicam quando a atribuição diária está incompleta. Atividade > Ditado no Nest
é dona dos números, filtros de período, barras diárias e aplicativos mais usados; a área de Ditado
aponta para lá. Atividade > Semana inclui palavras e semanas com apenas uso de ditado, mesmo com
o histórico de agentes desligado. O período é estado apenas de apresentação, segue o armazenamento
local dos painéis do Nest e sobrevive à recriação da janela.

A inserção começa desligada. X11 nativo e Plasma Wayland capturam capacidades de foco separadas;
o segundo também exige autorização humana de teclado pelo portal, com tokens de restauração no
chaveiro do sistema. Reservas da área de transferência preservam cópias mais novas. A entrega
tmux exata também consulta o runtime sobre a prontidão atual da sessão: permissões/perguntas e
cartões adiados pendentes bloqueiam a entrega. Texto nunca vira Enter, comando shell ou decisão
de permissão.

Enquanto há captura, seu guard opcional `platform::music::Listening` diminui ou pausa players
MPRIS e restaura os não alterados ao terminar/descartar a captura. Esse controle local não lê
metadados das faixas e é separado do observador Tocando agora de `media`. O Chat e a futura
captura de reuniões não entram nesse guard. A transcrição comum fica neste computador; a limpeza
e as reuniões futuras mantêm seus próprios interruptores e contratos do gate de rede.

A V2.1 acrescenta `store::words`: palavras do dicionário, regras de substituição ordenadas e
trechos na migração 22. Chaves canônicas de caixa Unicode tornam atômica a verificação de
duplicatas entre conexões do banco. Atualizar uma substituição preserva seu id/ordem; palavras
e gatilhos de trechos repetidos são recusados. Os comandos exclusivos do Nest `dictation_words`
/ `dictation_words_change` leem uma visão consistente ou aplicam uma edição tipada num worker
bloqueante, com SQL parametrizado e erros de banco sem dados do usuário. O conteúdo de trechos e
substituições fica literal. Limpar o Histórico preserva essas preferências; a exclusão individual
compacta as linhas removidas. A V2.2 captura essas regras por gravação e executa o fluxo puro
`dictation::text::Rules` após o cancelamento falado no texto original: marcadores opacos de
trechos, substituições literais ordenadas, grafia elegível do dicionário, código falado conforme
o contexto e restauração literal dos trechos. A comparação Unicode de caixa/palavras mantém
controles como barreiras. Segmentos anotados guardam a regra e o texto correspondente para as
prévias/destaques de frases; o Histórico guarda o texto original e final com um estilo preciso de
processamento. O fluxo roda fora do lock de controle e reconfere a geração antes de registrar
o resultado. Falha na leitura das regras ou no limite de 256 KiB de entrada/saída intermediária
impede a entrega. Reutilização e Chat não passam por ele; a V2.5 oferece as páginas de trabalho
Dicionário/Trechos no Nest e o comando somente de leitura
`dictation_try`. O comando verifica o chamador Nest, captura regras salvas num worker bloqueante
e executa o mesmo fluxo `Rules::apply`, respeitando o cancelamento falado salvo e o contexto
opcional de terminal/código. Seu resultado limitado inclui segmentos inertes e anotações de
regras; não captura áudio, chama modelo, escreve na área de transferência/histórico nem concede
capacidade de inserção. Campos estáveis mantêm rascunhos, cursor e rolagem durante leituras e
trocas de idioma; gravações serializadas bloqueiam ações duplicadas, exigem confirmação de exclusão
e releem dados salvos. Falhas de leitura mantêm uma visão antiga com nova tentativa; mudanças
na frase descartam respostas antigas da prévia assíncrona. Configurações mantém os dois controles
opcionais de vocabulário; as páginas de trabalho só levam até lá.

A V2.3 acrescenta contexto opcional do dicionário no esquema 34 de Configurações (desligado por
padrão; controle em Configurações > Ditado > Suas palavras). Cada gravação captura a opção e o dicionário uma vez. Prévia/final
passam só as palavras do dicionário para `voice`; uma nova tentativa com Base reconfere seu modelo
real. O tokenizador do modelo limita o contexto de programação mais entradas inteiras a 224
tokens; ignora palavras vazias, com controles ou repetidas. Os tokens pertencem a uma decodificação,
nunca ao modelo em cache, e o Chat mantém seu fluxo original de prompts. A restrição por
modelo/idioma permite atualmente só Turbo/inglês fixo: apenas essa combinação passou nos
testes locais de 20 frases por idioma mais silêncio/ruído. Base/Small e português continuam
bloqueados mesmo onde o WER das frases melhorou. A origem e os resultados dos testes ficam em
`crates/voice/tests/fixtures/README.md`; WER sintético é separado da aceitação humana no desktop.

A V2.4 acrescenta a preferência independente `dictation_project_vocabulary` (esquema 35, desligada
por padrão, controle em Configurações > Ditado > Suas palavras). O leitor existente de repositório, fora do loop, atualiza o
cache opcional em memória no início da sessão e na atualização comum do repositório. Cada árvore
de trabalho Git tem sua própria entrada (máximo de 128), para não misturar branches e membros
entre worktrees de um mesmo pouso. O extrator lê nomes e dependências Cargo/npm, membros do
workspace literais ou terminados em `/*`, nomes próximos de módulos em `src`, nomes de arquivos
da raiz e branch. As leituras têm limites: manifests de 64 KiB, 64 diretórios membros, 512 entradas
por diretório e 256 nomes únicos com até 128 bytes. Ignora links simbólicos no arquivo final do
manifest, diretórios de workspace/fontes ligados, caminhos membros absolutos ou com pai, arquivos
inválidos e nomes ocultos/com controles. Arquivos de código nunca são lidos e scripts nunca são
executados. Metadados do repositório continuam sendo contexto não confiável, sem autoridade
para permissões.

O leitor aproximado de primeiro plano no pressionamento da tecla seleciona nomes em cache pela
pasta da sessão fixada/em foco ou por um título sem ambiguidade de terminal/editor. Contexto
desconhecido ou tardio não acrescenta nomes do projeto. Um `OnceLock` ligado à geração fixa o
resultado antes da primeira decodificação real; prévia e final o compartilham, e cancelamento/
reinício recusam observações antigas. Palavras do dicionário pessoal precedem nomes do projeto
sob o limite combinado de tokens e a restrição modelo/idioma da V2.3. As regras puras de texto
final, Chat, capacidades de inserção e passagem de rede não mudam.

O renderizador cuida do Histórico e das indicações temporárias, não da inserção nativa nem da
alteração de configurações. Comandos exclusivos de Configurações verificam o label do webview
chamador antes de salvar. O [guia de ditado](guide/dictation.md#supported-destinations) lista os
destinos realmente compatíveis; a [ADR 0032](adr/0032-dictation-into-other-apps.md) registra os
limites de capacidade e permissão.

<a id="meaning-in-core-look-in-the-surface"></a>

## Significado no core, aparência na superfície

A view diz o que as coisas significam, para que todas as superfícies concordem ([ADR 0021](adr/0021-four-sizes-one-nest.md)).
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
cabeçalho (ADR 0021). Os comandos conferem o label do webview que chamou: as chamadas que só as
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
respondido; nenhum modo de presença pode escondê-lo depois de confirmado ([ADR 0021](adr/0021-four-sizes-one-nest.md)).

<a id="why-only-core-produces-a-decision"></a>

## Por que só o `core` produz uma decisão

`core::reduce` transforma um evento `PermissionRequested` em estado pendente, e só o clique de um humano
transforma estado pendente num effect `RespondPermission`: `Intent::Decide` (Allow, Deny) ou
`Intent::DecideAlways`, que também salva uma regra. Uma regra salva só responde a um pedido que bate com
ela exatamente: o mesmo agente, pasta, ferramenta e alvo ([ADR 0004](adr/0004-a-human-answers-permissions.md)). Uma pergunta
(`QuestionAsked`) espera do mesmo jeito, e só um `Intent::Answer` que se encaixe nas perguntas dela a transforma
num effect `AnswerQuestion`. Um teste alimenta
todas as outras entradas em todas as ordens e verifica que nenhuma delas responde.

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
opcional, regras antes de modelos) está nos [registros de decisão](adr/README.md).

<a id="documentation"></a>

## Documentação

`docs/` é a fonte da documentação no site
([rogeriojunior31.github.io/en/docs/vults](https://rogeriojunior31.github.io/en/docs/vults/)),
publicada a cada tag de release; `docs/site.json` guarda o nome e o resumo que o site mostra. As páginas começam com um
`# H1`, usam links relativos e guardam as imagens em `docs/assets/`. `docs/adr/` guarda os registros de decisão. `docs/dev/` é interno e não é
publicado; `docs/pt-br/` guarda a tradução em português do Brasil (veja o CONTRIBUTING).
