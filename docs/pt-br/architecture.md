# Arquitetura
<!-- source: 869b5565a2ee -->

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
| **Connect** | `connectors`, `chat`, `voice`, `media` | O que vai além dos agentes: GitHub, as CLIs e APIs do chat, o microfone, o que está tocando |
| **Core** | `core`, `protocol`, `peer`, `ipc`, `hook`, `agents`, `agent-config` | Sessões, eventos, aprovações e decisões: o que os agentes estão fazendo e o que o humano disse |
| base | `brand`, `secrets` | O nome, o chaveiro do sistema |

Só o `app` liga as camadas entre si: Connect nunca chama Experience, e Core não conhece nenhuma das duas.
Isso mantém barata uma superfície nova (ela desenha `State::view` e envia intents, [ADR 0008](adr/0008-one-core-many-surfaces.md))
e mantém o Zeca opcional ([ADR 0010](adr/0010-zeca-is-optional.md)): ele usa Chat, Voice e
Connect, e nenhum deles sabe que ele existe.

| Crate | Papel | Não pode usar |
|---|---|---|
| `brand` | O nome, o slug e o bundle id do app; gera `ui/src/brand.ts` | nada |
| `protocol` | Mensagens versionadas hook ↔ app, limites, nomes de socket e pipe | tokio, Tauri |
| `peer` | Verificações de mesmo usuário nas duas pontas da conexão | tokio, Tauri |
| `hook` | Lê o JSON de hook do agente, o encaminha, imprime a decisão no formato do agente | tokio, HTTP |
| `ipc` | Servidor assíncrono: limites de conexão, confirmar-e-depois-decidir, roteamento para o app | Tauri |
| `core` | Domínio puro: sessões, aprovações, alertas, a view, cujos tipos TypeScript ele gera em `ui/src/view.gen.ts` (um teste verifica que está atualizado); relógio injetado | IO, async, Tauri |
| `agents` | Por agente: nomes de eventos, ferramenta → atividade, entradas de instalação, confiança do Codex | Tauri |
| `agent-config` | Edições seguras das configurações dos agentes: leitura estrita, diff, fingerprint, backup, escrita atômica | Tauri |
| `chat` | Chat pelas CLIs `claude` e `codex`, com pedidos de permissão, ou por uma API: Anthropic ou uma nuvem compatível com OpenAI com a chave do usuário, ou um Ollama / LM Studio local | Tauri |
| `secrets` | O chaveiro do sistema (Secret Service, Credential Manager), indexado pelo bundle id | Tauri, arquivos |
| `connectors` | Vults Connect: o trait `Connector`, o runtime de polling, GitHub | Tauri, core |
| `voice` | Apertar para falar: o microfone para a memória (cpal), whisper.cpp neste computador, downloads de modelos verificados por SHA-256 | Tauri, core |
| `media` | O que está tocando (MPRIS pelo session bus, pelos sinais dele) e tocar/pausar/pular | Tauri, core |
| `platform` | Posicionamento das superfícies no Linux (layer-shell, uma região de entrada por janela), o item da bandeja (um StatusNotifierItem), atalhos globais (o portal do desktop) e pular para o terminal | Tauri, core |
| `app` | O shell Tauri: o loop do runtime, effects, comandos, bandeja, configurações | — |

A UI (`ui/`) é TypeScript sem framework. `src/bridge.ts` é o único arquivo que fala com o Tauri;
`src/island/` renderiza a ilha a partir da view, `src/character/` desenha as aves a partir dos dados de sprite,
e `src/surfaces/` guarda todas as outras superfícies, uma pasta cada (`widget/`, `settings/`) com a sua
entrada `main.ts` e a sua folha de estilo. O código usado por mais de uma superfície fica na raiz de `src/`.

<a id="meaning-in-core-look-in-the-surface"></a>

## Significado no core, aparência na superfície

A view diz o que as coisas significam, para que todas as superfícies concordem ([ADR 0008](adr/0008-one-core-many-surfaces.md)).
Cada sessão carrega a sua `attention`, um nível ordenado (`quiet` < `info` < `done` < `failed` <
`needs-you`), e a view carrega o maior deles; `card` marca a sessão cujo card é o primeiro
da fila e ainda espera por ele. A ilha só escolhe a aparência: que som cada nível faz, e
quanto tempo um estado precisa durar (1,5 s, 3 s para `done`) antes de virar novidade.

O runtime transmite a view, quando ela muda, para todas as janelas de uma vez (`publish_view`), e
guarda a última para uma página que carregue depois (`current_view`). Eventos destinados só à ilha
(ponteiro, atalhos, chat) são enviados para o label dela. Um `listen` comum ouve todos os alvos, então uma
superfície nova assina esses eventos pelos helpers com escopo de janela em `bridge.ts`
(`getCurrentWebviewWindow().listen`) para ficar fora deles.

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
respondido; nenhum modo de presença pode escondê-lo depois de confirmado ([ADR 0009](adr/0009-presence-never-hides-a-card.md)).

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
publicado; `docs/pt-br/` vai guardar a tradução. `node tests/visual/docs-shots.mjs` refaz as
imagens da ilha do README e dos guias a partir do lab; rode antes de cada release.
