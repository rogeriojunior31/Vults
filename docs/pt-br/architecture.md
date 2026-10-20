# Arquitetura
<!-- source: 8b29ba702e56 -->

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

<a id="voice-conversation-turns"></a>
## Turnos da conversa por voz

A V6.1 acrescenta a máquina pura `voice::turn`, ainda sem chamadas pelo app. Tempo monotônico
injetado e tokens opacos de turno/tentativa vinculam cada resultado assíncrono. A primeira saída
consumida inicia um bloqueio de 500 ms com fones ou 1 s com cancelamento de eco; a interrupção com
alto-falantes vem desligada. Um início de fala suprimido que continua ativo pode pedir uma análise
separada com 260 ms de áudio anterior após o bloqueio. Só a confirmação da tentativa correspondente
cancela a saída e avança o turno; rejeitar ou atingir o prazo de 1,5 s mantém a resposta em andamento.

O fim da produção pede o escoamento da reprodução; uma fila vazia transitória anterior não significa
conclusão. A confirmação desse escoamento inicia uma tolerância de cauda de 450 ms no desktop. Fala
nesse intervalo é um novo turno, sem reaproveitar a cauda da saída anterior; expirar termina sem
abrir captura. Um início pendente no escoamento vira um novo turno com seu áudio anterior, em vez
de cancelar uma resposta já silenciosa. Cancelamento/falha limpa a saída e recusa callbacks
atrasados. Adaptadores devem manter os tokens de cada ação, confirmar o consumo/escoamento real da
saída e ser donos do buffer circular do microfone em memória; trocar o token de um resultado antigo
pelo atual viola o contrato. A máquina não faz IO de áudio, provedor, permissão ou rede, e seus
efeitos não concedem consentimento para o microfone. Captura/reaproveitamento no runtime,
orçamento/gate realtime, reprodução e integração da interface continuam nas próximas etapas da V6.

A V6.2.1 acrescenta a política pura de conversa `voice::turn::end`: 1,6 s de silêncio, ou 2,6 s após
uma palavra de continuação em inglês/português. Exige fim de fala do VAD e palavras não vazias;
silêncio antes da fala ou voz em andamento nunca concluem o turno. Só crescer além do maior número
de palavras anterior da transcrição parcial reinicia seu timer de texto. Reescritas e eventos
repetidos de fim do VAD não ficam reiniciando a pausa. Outros idiomas usam o prazo base.

O agendador semântico opcional, só de áudio, emite no máximo um pedido pendente, vinculado a
geração/token, a cada 200 ms. Uma probabilidade de pelo menos 0,5 só pode concluir após a política
atual de silêncio; valores não finitos/fora do intervalo ou uma falha de inferência o desativam
pelo resto da conversa. A espera adicional absoluta é de 1,2 s desde quando o silêncio seria
elegível, não desde uma consulta atrasada; um pedido travado não a estende. Retomar a fala cancela
o token e a espera pendentes; cancelamento/novos turnos recusam callbacks antigos. A conclusão é
emitida uma vez. Limites de duração da captura e entrada vazia ficam com o adaptador. Os gravadores
existentes do Chat/ditado mantêm suas próprias regras de parada.

A V6.2.2 acrescenta o backend opcional `semantic-turn`, com grafo smart-turn v3.0 fixado e runtime
CPU Linux x64 qualificado de forma independente. Corte/preenchimento à esquerda de oito segundos,
normalização e features Whisper/Slaney correspondem às matrizes NumPy da implementação de referência;
uma execução nativa explícita com artefatos conferidos corresponde às probabilidades de referência.
A admissão compartilhada `model_file` mantém os bytes exatos, e `onnx::Runtime` passa a ser o único
responsável nativo por diarização e turnos semânticos. Admissão interativa cancelável, prazos por
quadro no pré-processamento e encerramento via RunOptions respeitam o orçamento restante do token
da política. A construção do modelo/sessão nativa não promete aborto imediato. Não há chamada no app,
download, opção em Configurações ou captura nova. Veja [Modelo semântico de fim de turno](reference/semantic-turn-model.md)
para origem BSD-2-Clause, pins exatos, comandos de qualificação e critérios pendentes de fala/latência/plataforma.

A V6.3.1 acrescenta `store::money` (migração 28): reservas em micro-USD inteiros e conciliação única
do uso final, vinculadas à identidade existente do registro de requisições. Transações imediatas
gravam a reserva e seu trace inicial juntas, impedindo operações simultâneas de gastar o mesmo saldo.
Cotações ausentes/nulas, limites zerados, registros indisponíveis e estouro aritmético recusam a
admissão. Reservas abertas nunca expiram, inclusive após falha do processo; custo final desconhecido
permanece nulo e debita a reserva inteira. Só um transporte encerrado pode conciliar o uso. Débitos
concluídos expiram 24 horas após a conclusão; horários futuros continuam debitados se o relógio recuar.
Apagar o histórico preserva reservas/débitos e uma conciliação tardia não recria traces apagados.
Excessos cobrados pelo provedor são debitados integralmente, sem truncamento.
O sucesso/falha da requisição fica no trace independentemente da disponibilidade dos dados finais de cobrança.
O app recusa explicitamente `voice-realtime` pelas configurações de conectores e pelo gate de
contagem. As seções posteriores acrescentam admissão, transporte e recuperação sobre esta base de
persistência. Nenhum reembolso ou recuperação automática libera um orçamento novo.

A V6.3.2 acrescenta o responsável offline `chat::realtime`: tarifas inteiras revisadas, tetos
conservadores por resposta, limites finitos explícitos de sessão e confirmação da configuração
efetiva, uma tentativa vinculada ao identificador da resposta e recibos únicos de uso. Uso
desconhecido, eventos antigos/grandes demais e configuração alterada não reabrem um responsável nem
reembolsam sua reserva. Texto/áudio em cache têm preços separados; excessos conhecidos ficam visíveis
integralmente. A cassette sintética é offline; não adiciona chamada ao provedor, microfone nem
autoridade de permissão. Veja [Contabilidade realtime](reference/realtime-accounting.md) para fontes
das tarifas, demonstração do teto, expiração da qualificação e validação em execução ainda obrigatória.

A V6.3.3 acrescenta provedor WebSocket TLS de origem fixa, canais limitados de PCM em streaming e gate
monetário controlado pelo app. Configurações 41 mantém consentimento independente desligado por
padrão, limite/modelo e vínculo com o registro. Uma concessão revogável de voz verifica escolha/
bloqueio/cancelamento durante configuração, IO e publicação; encerrar o app fecha permanentemente
a admissão. Reservas antecedem chaveiro/conexão/áudio; a conciliação dos recibos continua após
revogação. Guardiões de entrega não reclamada restituem apenas trabalho comprovadamente não enviado;
reservas pendentes bloqueiam novas reservas dentro desse responsável. A V6.3.4 acrescenta trava do
kernel no diretório de dados Linux, mantida pelo transporte e cada worker desacoplado. Só um novo
responsável exclusivo concilia reservas órfãs, mantendo atomicamente o custo desconhecido integral
por 24 horas desde a conciliação. Reiniciar não restitui nem adia um débito já conciliado, e traces
apagados continuam apagados. Outras plataformas recusam a entrada. Não há entrada nativa de
captura/reprodução. Testes duplex e de processos/falhas são offline; validação com provedor/
dispositivos reais e continuação oferecida ao usuário continuam pendentes antes da ativação.
O inventário de rede lista o transporte concreto e seus testes.

A V6.3.5 altera o serviço nativo do app para controlar um turno completo limitado, independentemente
da admissão remota. A entrada do transporte persistente continua em blocos; sucesso libera o
original e falha encerra o transporte antes da recuperação local pelo Whisper. Um texto completo
revisável ou o áudio original com falha local tipada volta ao chamador nativo, sem troca de provedor,
envio automático ao Chat ou gravação de histórico. Cancelamento/revogação suprimem resultados antigos;
mudanças compartilhadas revogam gerações sob a trava das Configurações e workers de decodificação
mantêm a concessão nativa após o prazo. A espera local é limitada, mas a carga fria nativa continua
cooperativa. Captura/reprodução e continuação visual ficam na V6.4-V6.7. Veja
[Contabilidade realtime](reference/realtime-accounting.md#native-turn-retention-and-local-text).

A V6.4.1 acrescenta a saída independente `audio::playback` e a contabilidade pura
`voice::turn::heard`. Um worker CPAL limitado reamostra PCM16 a 24 kHz, distingue frames na fila,
entregues e estimados como consumidos, e separa a conclusão do produtor da drenagem do dispositivo.
Parada/revogação congela o cursor e limpa a fila; a destruição nativa mantém a propriedade exclusiva.
O separador puro estima palavras inteiras ouvidas e calcula contexto interrompido do modelo mais
uma dica única de texto não ouvido. Ele não escreve histórico nem envia nada. A V6.4.2 acrescenta o
proprietário do serviço nativo: turnos completos/sinais vinculados à geração, identidades de
provedor/item/transcrição pelo gate, consumo/drenagem CPAL e truncamento exato confirmado no servidor.
Ele emite registros correspondentes de histórico ouvido em memória e admite uma dica de reparo
citada uma vez na próxima resposta autorizada. Reprodução terminal ainda pode terminar ou ser
interrompida; histórico sem confirmação do servidor é explicitamente não sincronizado e não pode
retomar o proprietário. A saída nativa desacoplada mantém a concessão da sessão. Não há microfone,
entrada do renderer nem gravação persistente de histórico do Chat; interrupção por fala continua
desligada na entrada do app até a V6.5. Veja
[Reprodução nativa e reparo de contexto](reference/realtime-accounting.md#native-playback-and-context-repair)
para limites, premissas de relógio, testes ALSA nulos isolados e a aceitação física/real pendente.

A V6.5.1 acrescenta o inventário somente de leitura `audio::echo` para pares de módulos WebRTC do
PipeWire configurados explicitamente. Snapshots opacos vinculam módulo/grupo de fonte e destino e
identidades de servidor/nós; verificá-los novamente não concede uma rota. Um experimento nativo
privado qualifica apenas eco sintético/preservação de tons. A V6.5.2 acrescenta um proprietário
nativo duplex separado, vinculado ao consentimento, com confirmação exata dos destinos e streams
`DONT_MOVE`. Ele observa conexões internas do módulo AEC e destinos físicos, revoga rotas alteradas
e mantém exclusividade nativa durante a destruição. Fones exigem escolha explícita; rotas de
alto-falantes sem qualificação mantêm a interrupção desligada. Intervalos de conteúdo, latência do
destino e confirmação separada de drenagem preservam a contabilidade da saída consumida. Invalidar
a rota também limpa evidências retidas de interrupção. A captura limitada retorna somente ao
chamador nativo; não é enviada nem salva automaticamente. O caminho CPAL padrão continua sem
interrupção. Não há carregador de módulos de produção, entrada do renderer nem ativação visível
de captura. O backend da V6.5 está entregue; ferramentas na V6.6 e captura/interface na V6.7 vêm
a seguir. Veja [Cancelamento de eco](reference/echo-cancellation.md) para o teste privado de
roteamento nativo e os limites restantes de aceitação física/real.

A V6.6.1 encaminha transcrições locais finalizadas ao dispatcher de comandos da V8 antes do envio
realtime. O proprietário do estado verifica Configurações/gerações e admissão cancelável de uso
único, reutiliza leituras locais com remoção de segredos e separa o contexto do ditado independente.
Comandos tratados/recusados/esclarecidos aposentam a geração sem Generate nem fallback de modelo.
A V6.6.3 distingue recibos de efeitos concluídos, encaminhados e com falha; lotes que misturam
leitura e ação também esperam. Salvar projetos com sucesso pode concluir, enquanto encaminhar
aberturas continua como Solicitado. O consumo compartilhado da captura bloqueia recuperação e
repetição de cópias com nova geração após confirmação, conciliação ou interrupção. Trabalho nativo
enfileirado verifica novamente chamador/autoridade/prazo e mantém a concessão exclusiva até sair. O Linux
exige transcrição limitada da mesma captura; texto ausente falha antes de construir o provedor e
oferece apenas recuperação revisável. Não há ativação de renderer/montagem de captura, ferramenta
do modelo nem consulta. Veja
[Admissão de comandos locais](reference/realtime-accounting.md#local-commands-before-realtime-upload).

A V6.6.2 fornece ao realtime as cinco leituras existentes do bando em cache por especificações
congeladas e confirmadas. Respostas de função em sequência têm argumentos limitados, verificação
exata de identidades transmitidas/finais e controle de repetição. A conciliação precede a reserva
de cada continuação e leitura; envelopes inertes de sucesso/erro têm limite após escapes JSON.
Recibos de ferramentas liberam a entrada original sem concluir a reprodução; a fala final mantém
sua própria identidade/histórico. Chamadas consumidas não viram recuperação nem repetição automática.
Não há ferramenta de ação/permissão nem ativação visível. Veja
[Ferramentas realtime somente de leitura](reference/realtime-accounting.md#read-only-tools-in-realtime).

A V6.6.3 pode acrescentar perguntas textuais `consult_claude` e `consult_codex`, com consentimentos
separados, a esse registro nativo. Cada provedor tem opção própria desligada por padrão e cota
persistente de 20 tentativas por 24 horas móveis; ambos compartilham uma vaga até confirmar a saída
do processo. O Codex usa metadados fixos de modelo controlados pelo app e recusa versões não
qualificadas. A verificação prévia de assinatura não é atômica com exec; trocas externas de login
ainda bloqueiam a ativação. Não há entrada de renderer/captura. Suas opções, orçamentos persistentes
e traces locais de metadados são independentes dos custos realtime e do Chat. O cliente existente sem
ferramentas desativa expansão de arquivos e carregamento de memória e exige controles suportados
pela CLI; não herda sessão ativa de agente nem permissões do Chat. Um prazo original cobre
preparação e saída do processo, com aviso nativo traduzido após 1,5 segundo; a posse da vaga/voz
sobrevive a trabalho que não pode ser interrompido. Respostas são dados externos inertes. Políticas
gerenciadas da CLI continuam valendo. Vínculo com a assinatura, qualificação de cliente real/gerenciado e controles visíveis seguem em aberto.
Veja [Consulta textual](reference/realtime-accounting.md#text-consultation).

<a id="local-dictation"></a>

## Ditado local

`dictation::command` fornece a gramática da V8.1 sem um chamador em execução. A normalização Unicode
de caixa/acentos/pontuação produz chaves lexicais; a remoção inicial de cortesia/muletas e as formas
de verbos iniciais pertencem somente às cláusulas de comando. Tabelas em inglês, português e espanhol
retornam descrições com nomes de destino ainda não resolvidos, nunca intents do core. Conectores
explícitos de sequência produzem um lote só se todas as cláusulas forem reconhecidas; frases que
respondem a cards recusam toda a fala. A análise é limitada a 4 KiB e oito comandos, sem IO, microfone,
modelo, área de transferência ou histórico.

A V8.2 acrescenta `command::names`, uma cópia independente das identidades de projetos/agentes e das
substituições salvas. Aliases elegíveis apontam para nomes conhecidos completos; a resolução prioriza
aliases, nomes exatos, palavras inteiras isoladas e correspondências por distância Levenshtein Unicode
limitada. Consultas aproximadas de uma palavra também comparam as palavras de cada nome; assim,
`apy` mantém tanto `api` quanto `api-gateway`. Empates na menor distância continuam ambíguos;
identidades distintas nunca são agrupadas pelo rótulo. Um relógio injetado limita um esclarecimento
às opções originais por 90 segundos, sem renovar o prazo quando uma resposta não resolve a dúvida.
Uma opção removida ou renomeada invalida a pergunta; uma resposta bem-sucedida a consome. Nenhuma
resolução executa ações.

O `app::voice_commands::Commands` da V8.3 aceita uma transcrição somente do responsável pelo estado
no runtime e prepara um lote sobre uma cópia desse estado. Só `Focus`, `FocusNext`, `FocusPrevious`,
`Jump`, `OpenFolder` e `SetProjectPref` passam pelo limite exaustivo de `Intent::voice_command_allowed`;
respostas a cards, liberação/adiamento/revisão e merges ficam excluídos. Lotes não resolvidos,
recusados ou indisponíveis descartam a cópia e todos os efeitos. Um lote bem-sucedido efetiva a cópia
e devolve os efeitos ao executor existente. Identidades qualificadas de sessões distinguem rótulos
duplicados; preferências são agrupadas pela pasta exata. Lotes pendentes preservam as identidades
dos destinos anteriores e as revalidam quando a pergunta é resolvida. Repetir e desfazer guardam
90 segundos de contexto com tempo injetado; desfazer uma preferência restaura o valor anterior só
enquanto a pasta da sessão e o valor esperado ainda corresponderem. Desfazer o foco verifica o foco
atual e a sessão anterior ainda ativa. Bloqueio/cancelamento limpa o contexto e a pergunta.

Consultas reutilizam `flock_tools` sobre a visão local atual e o uso em cache, incluindo
`session_detail_exact` com chave exata e o mesmo formatador de detalhes com segredos ocultos já
existente. Cards pendentes, estado/etapas de sessões, uso e linhas do GitHub viram frases completas
nos quatro idiomas. Títulos externos continuam como texto inerte; essas consultas não usam modelo,
busca na rede, escrita na área de transferência nem resposta falada a cards.

A V8.4 fixa o modo de ditado/comando em cada ticket do worker antes da captura. O atalho separado
`talk-command` do portal sempre entra nesse worker, inclusive com o Zeca desligado ou o Chat em foco.
A captura de comandos usa gestos de apenas segurar e a transcrição local compartilhada, sem
transformações de texto, observação de foco/painel nem área de transferência/inserção. O worker lê
os aliases salvos fora do loop do core e enfileira a transcrição com seu ticket. O runtime a admite
sob a trava curta de cancelamento do ticket, chama o dispatcher síncrono de dados em cache e executa
os efeitos pelo executor existente. Não há await nem IO nessa admissão. Um ticket substituído ou
cancelado não pode executar trabalho enfileirado.

A linha/legenda existente de ditado, com um único responsável por exibição, apresenta a captura e os
resultados dos comandos. A expiração absoluta impede que trocas de superfície ou respostas sem
resolução renovem os 90 segundos originais de esclarecimento; a legenda do que foi ouvido continua
expirando em até dois segundos. Cards e layouts restritos mantêm prioridade. O Histórico registra
comandos com um resultado distinto e a mesma ordenação de prazo/exclusão, sem agregados de uso/tempo
de entrega do ditado. Não há repetição na UI, e o backend rejeita repetir linhas de comandos.
Copiar novamente mantém somente ditado comum.

A V8.5 acrescenta o controle de comandos em Settings (ligado por padrão) e o envio separado de falas
desconhecidas ao Chat, desligado por padrão. Alterar qualquer escolha cancela tickets e contexto de
comandos sem interromper o ditado comum. A página de comandos do Nest filtra o histórico retido e
compartilha o guia de frases com Settings. Somente uma fala desconhecida, sem recusa nem esclarecimento
ativo, pode pedir envio ao Chat. Reserva o Chat atual livre, recusa texto/arquivos/referências por
enviar e reconfere configurações, bloqueio, ticket e pasta/provedor selecionados após a preparação
assíncrona. Conversas novas usam essa pasta, sem mudar o destino de uma conversa já iniciada. Nenhum turno espera
na fila do Chat. A mensagem admitida usa o envio normal da conversa atual, suas ferramentas, portão,
orçamento, rastros e aprovações; a resposta aparece no Chat. Não herda o rascunho do compositor nem
altera a autoridade sobre aprovações. Após a admissão, o Chat é responsável por Parar.

A V3.1 introduz `vults-dictation::cleanup` como contrato puro de prompt/resposta.
Envolve os dados escapados de transcrição/aplicativo/grafia em turnos reais de mensagens, demonstrando
perguntas e imperativos como texto. Edições locais tipadas são substituídas por marcas opacas sem
colisão; seu conteúdo fica fora do prompt do modelo e só é restaurado literalmente após validar a
sequência. Limites de entrada/saída, controles, pontuação explícita de perguntas e plausibilidade de
palavras/caracteres rejeitam respostas inadequadas, mantendo o original local completo. Essas
verificações não provam fidelidade semântica. A V3.2 conecta a preferência separada `dictation_cleanup`,
desligada por padrão (esquema 37 de Settings), a uma chamada sem ferramentas pelo provedor de API
selecionado em Chat & voice, incluindo Ollama/LM Studio em IP literal de loopback. A escolha capturada
ao pressionar a tecla não herda consentimento posterior; mudar a preferência, o provedor/modelo ou a
chave invalida a limpeza em andamento. O worker dedicado preserva a transcrição local e as verificações
finais de inserção/cards/destino. Falha ou prazo de quatro segundos mantém o texto local completo;
tradução tem prazo de oito segundos e Prompt tem doze.

O portão admite até 2.000 tentativas em 24 horas móveis, incluindo chamadas locais. A migração 25
acrescenta um registro de admissão sem conteúdo e o rastro inicial, gravados juntos antes do HTTP.
Reservas sobrevivem a reinício, falha/interrupção e Clear history. Settings vincula o consentimento à
identidade do registro; registro ausente, ilegível ou recriado impede requisições, sem renovar a cota. A conclusão atualiza apenas rastros preservados com provedor/modelo/local/tempo/resultado
e tokens conhecidos, nunca transcrição/áudio/área de transferência/chave nem uso inventado. O cliente
HTTP desativa proxies, redirecionamentos e novas tentativas. A V3.3 fixa aplicativo/estilo/idioma de
saída no callback da tecla. Terminais/editores usam Como falado, e-mail usa Polido, chats usam Natural
e sessões reconhecidas do bando usam Prompt. Uma escolha explícita por aplicativo precede o padrão
da categoria; sessões mantêm Prompt. Outros aplicativos identificados usam o padrão global (Natural).
Foco ausente ou preferências ilegíveis mantém o texto local sem modelo. Apenas aliases exatos e
revisados do desktop compartilham escolhas; títulos de janela/URLs nunca selecionam estilo.
O esquema 38 de Settings acrescenta estilo global, idioma de saída opcional e inglês nas sessões,
desligado por padrão. Inglês nas sessões precede o idioma global apenas em sessões reconhecidas;
Como falado nunca traduz nem chama modelo.

A migração 26 acrescenta `dictation_styles(app, style)`, preservada por Clear history, e
`output_language` opcional no histórico, separado do idioma ouvido. O worker registra estilo/idioma
aplicados apenas após sucesso da limpeza; falhas mantêm o estilo do processamento local. Mudanças de
estilo/idioma valem na próxima gravação sem alterar a escolha já capturada. X11 lê a identidade em
foco no callback da tecla; Plasma Wayland reutiliza o observador KWin verificado com script de vida
separada, preparado ao ativar a limpeza, sem permissão de teclado pelo portal. Perda do observador ou
ambiguidade mantém o texto local; contexto assíncrono da interface não muda uma escolha capturada.
A V3.4 acrescenta nas Configurações o consentimento separado da limpeza, a divulgação do provedor,
os cartões de estilo, as escolhas de idioma/inglês nas sessões e as opções por aplicativo recolhidas.
As leituras usam apenas configurações e metadados locais do provedor; abrir Estilos não testa modelos
nem busca sua lista. Escritas pendentes, eventos recentes e leituras antigas preservam sua ordem;
falhas mantêm a opção de recarregar, e é possível desligar um recurso já ativo mesmo com falha nas
leituras auxiliares. A aba Ditado apresenta um único editor de Estilos pelo destino privilegiado
`dictationstyles` de Configurações, junto às páginas Voz, Atalhos de teclado, Configurações dos
comandos, Vocabulário, Colagem, Privacidade e Diagnóstico. Links de páginas de trabalho voltam à
webview de transcrições do Nest; configurar não concede poderes ao Nest. Voz reúne os controles de
transcrição compartilhados, com um link em Chat e voz, e o Histórico nomeia apenas estilos/idiomas
aplicados conhecidos. Textos e cartões
compartilhados usam o tema de produção e os quatro idiomas da interface. Esse caminho não acrescenta
autoridade de aprovação nem execução. O processo guarda uma seção de configuração validada do
Ditado separada da aba de trabalho do Nest. Só Configurações pode lembrar essa seção; uma rota
explícita de trabalho a remove, e voltar pelo cabeçalho/teclado do Ditado usa o responsável nativo
para restaurar a webview correta. O armazenamento compartilhado da origem não concede autoridade
de configuração.

O ditado independente tem seu próprio worker cancelável no `app`; não passa amostras do microfone
nem transcrições pelo loop de eventos dos agentes. O atalho de fala do desktop vai diretamente para
esse worker ao pressionar e soltar, independentemente do app em foco ou do Zeca. Não pode abrir o
Chat nem iniciar sua voz. Somente o botão de microfone do Chat preenche diretamente seu rascunho.
As páginas de configuração mantêm seu diagnóstico explícito, separado e limitado de microfone.

```text
desktop talk shortcut -> app::dictation worker (generation)
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
                              passive dictation capsule           |
                                                         dictation history + totals
                                                          (store, retention choice)
```

No Linux, `app::dictation_overlay` mapeia uma superfície passiva de camada de 480 x 160, centralizada
embaixo e acima dos painéis do desktop. Sua região de entrada vazia e teclado desativado preservam
o foco do destino. Reutiliza o responsável de status/tema de produção e eventos locais, sem ser
dona de microfone ou rede. Uma consulta de disponibilidade somente de leitura oculta indicadores
internos duplicados; falha de construção mantém a alternativa da ilha/Nest. Cards, bloqueio,
presença pausada, expiração e legendas opcionais mantêm suas regras. A ilha continua sendo a única
responsável pelo som de início.

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
padrão; controle em Ditado > Vocabulário). Cada gravação captura a opção e o dicionário uma vez. Prévia/final
passam só as palavras do dicionário para `voice`; uma nova tentativa com Base reconfere seu modelo
real. O tokenizador do modelo limita o contexto de programação mais entradas inteiras a 224
tokens; ignora palavras vazias, com controles ou repetidas. Os tokens pertencem a uma decodificação,
nunca ao modelo em cache, e o Chat mantém seu fluxo original de prompts. A restrição por
modelo/idioma permite atualmente só Turbo/inglês fixo: apenas essa combinação passou nos
testes locais de 20 frases por idioma mais silêncio/ruído. Base/Small e português continuam
bloqueados mesmo onde o WER das frases melhorou. A origem e os resultados dos testes ficam em
`crates/voice/tests/fixtures/README.md`; WER sintético é separado da aceitação humana no desktop.

A V2.4 acrescenta a preferência independente `dictation_project_vocabulary` (esquema 35, desligada
por padrão, controle em Ditado > Vocabulário). O leitor existente de repositório, fora do loop, atualiza o
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

A interface Linux de perguntas de texto `ask_user` da M7 é separada desse caminho de respostas do
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
A [ADR 0028](adr/0028-spoken-replies-through-a-realtime-model.md#human-confirmed-question-drafts) também
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
opcional, regras antes de modelos) está nos [registros de decisão](adr/README.md).

<a id="documentation"></a>

## Documentação

`docs/` é a fonte da documentação no site
([rogeriojunior31.github.io/en/docs/vults](https://rogeriojunior31.github.io/en/docs/vults/)),
publicada a cada tag de release; `docs/site.json` guarda o nome e o resumo que o site mostra. As páginas começam com um
`# H1`, usam links relativos e guardam as imagens em `docs/assets/`. `docs/adr/` guarda os registros de decisão. `docs/dev/` é interno e não é
publicado; `docs/pt-br/` guarda a tradução em português do Brasil (veja o CONTRIBUTING).
