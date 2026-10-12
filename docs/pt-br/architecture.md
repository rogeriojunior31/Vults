# Arquitetura
<!-- source: 5bbde522b206 -->

Tudo flui num só sentido, por um único loop:

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
| **Connect** | `connectors`, `chat`, `voice`, `media`, `zeca`, `link`, `relay`, `evals`, `spike` | O que vai além dos agentes: GitHub, as CLIs e APIs do chat, o microfone, o que está tocando, um telefone pareado |
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
| `store` | O banco local: um `vults.sqlite` (SQLite embutido), migrações numeradas, o histórico de turnos, o log de auditoria só de acréscimo, os consentimentos (nunca apagados), traces, as conversas do Zeca (com o que cada uma leu de fora e o seu contexto congelado), os telefones pareados e o maior contador com que cada um assinou, o diário do Zeca e os resumos diários com a busca de texto e por vetor, as propostas de memória esperando um sim, as noites dele, as tarefas do usuário e o que os agentes custaram por dia e por roost | Tauri, async, a rede |
| `chat` | Chat pelas CLIs `claude` e `codex`, com pedidos de permissão, ou por uma API: Anthropic ou uma nuvem compatível com OpenAI com a chave do usuário, ou um Ollama / LM Studio local | Tauri |
| `zeca` | O cérebro do Zeca (ADR 0018): uma visão só de leitura do bando, a memória dele como um repositório git de pequenos arquivos Markdown de fatos, cada um com um limite rígido (cada escrita um commit) | `core` (então nada nele consegue construir um `Intent`), Tauri |
| `link` | O elo com um telefone pareado (ADR 0004): pareamento por QR, envelopes selados com X25519 e XChaCha20-Poly1305 (um relay só vê texto cifrado), respostas assinadas pelas chaves do Keystore do telefone e verificadas antes que o ledger do `core` as resgate | IO, a rede, Tauri |
| `relay` | O relay do telefone (ADR 0004): um servidor axum que guarda e repassa envelopes selados por caixa, com limites e descartados depois de 10 minutos, e o cliente que as duas pontas usam, cada pedido pelo gate de rede do app; depende só do `link` | Tauri, `core` |
| `evals` | Evals com cassettes (só para desenvolvimento, nunca no app): classes de política, redação, os prompts enviados aos modelos, streams gravados repetidos para injeção | ser ligado ao app |
| `spike` | O kit de spike (só para desenvolvimento, nunca no app): um modelo falso, um app falso no socket do hook e um terminal para as TUIs, para gravar os hooks de um agente de verdade sem conta; o `scripts/spike.sh` roda o kit para cada agente | ser ligado ao app |
| `secrets` | O chaveiro do sistema (Secret Service, Credential Manager), indexado pelo bundle id | Tauri, arquivos |
| `connectors` | Vults Connect: o trait `Connector`, o runtime de polling, GitHub | Tauri, core |
| `voice` | Apertar para falar: o microfone para a memória (cpal), whisper.cpp neste computador, downloads de modelos verificados por SHA-256 | Tauri, core |
| `media` | O que está tocando (MPRIS pelo session bus, pelos sinais dele) e tocar/pausar/pular | Tauri, core |
| `platform` | Posicionamento das superfícies no Linux (layer-shell, uma região de entrada por janela), o item da bandeja (um StatusNotifierItem), atalhos globais (o portal do desktop) e pular para o terminal | Tauri, core |
| `app` | O shell Tauri: o loop do runtime, effects, comandos, bandeja, configurações | — |

A UI (`ui/`) é TypeScript sem framework. `src/bridge.ts` é o único arquivo que fala com o Tauri;
`src/island/` renderiza a ilha a partir da view, `src/character/` desenha as aves a partir dos dados de sprite,
e `src/surfaces/` guarda todas as outras superfícies, uma pasta cada (`widget/`, `settings/`, `nest/`) com a sua
entrada `main.ts` e a sua folha de estilo. O código usado por mais de uma superfície fica na raiz de `src/`;
as exceções são o painel do chat (`src/island/chat.ts`, com o Markdown e os ícones dele) e o card do
GitHub (`src/island/board.ts`), que o Nest também desenha, e o visual das páginas das configurações
(`surfaces/settings/pages.css`), que a Week do Nest usa dentro de uma shadow root.

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
`--delay-ms` ou `--agent codex` para mudá-la. `ui/lab/` (`npm run dev`, depois `/lab/`) mostra todos os clipes
e a ilha com estados inventados, sem o app.

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
publicado; `docs/pt-br/` guarda a tradução em português do Brasil (veja o CONTRIBUTING). `node tests/visual/docs-shots.mjs` refaz as
imagens da ilha do README e dos guias a partir do lab; rode antes de cada release.
