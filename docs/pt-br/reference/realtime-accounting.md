# Contabilidade realtime
<!-- source: 59afba14de3c -->

A V6.3.2 fornece um contrato offline de protocolo em `chat::realtime`. Ele não abre conexão, lê
chaves nem captura áudio. A V6.3.3 acrescenta o backend com gate descrito abaixo; voz continua desligada
por padrão. Uma mensagem preparada não
autoriza rede: o app precisa primeiro reservar seu teto pelo gate monetário.

<a id="reviewed-rates"></a>
## Tarifas revisadas

Qualificadas em **2026-10-19**, em USD por milhão de tokens:

| Modelo | Texto: entrada / cache / saída | Áudio: entrada / cache / saída |
|---|---|---|
| `gpt-realtime-2.1` | 4 / 0,40 / 24 | 32 / 0,40 / 64 |
| `gpt-realtime-2.1-mini` | 0,60 / 0,06 / 2,40 | 10 / 0,30 / 20 |

As duas páginas informam contexto de 128.000 tokens. Somente esses identificadores são aceitos;
não há correspondência por prefixo nem substituição automática. Fontes: [Realtime 2.1](https://developers.openai.com/api/docs/models/gpt-realtime-2.1)
e [Realtime 2.1 Mini](https://developers.openai.com/api/docs/models/gpt-realtime-2.1-mini).
A qualificação local expira após 30 dias, em 2026-11-18 às 00:00 UTC; datas anteriores à qualificação
também recusam cotações. Essa é uma exigência local de revisão, não uma garantia de preço do provedor.
Revalide tarifas e limites dos modelos antes de ativar a integração e ao renovar a qualificação.

<a id="reservation-bound"></a>
## Teto da reserva

A sessão define explicitamente um limite finito de saída (1–4.096 tokens) e um limite positivo em
`truncation.token_limits.post_instructions`, no máximo igual ao contexto menos a saída. O servidor
precisa confirmar a configuração efetiva antes de preparar qualquer resposta. O limite de truncamento
da API exclui as instruções; portanto, sozinho não estima a entrada total.
[Referência de eventos do cliente](https://developers.openai.com/api/reference/resources/realtime/client-events).

A reserva conservadora é:

```
contexto * tarifa_entrada_texto
  + limite_post_instructions * tarifa_entrada_audio
  + limite_saida * max(tarifa_saida_texto, tarifa_saida_audio)
```

Ela cobre deliberadamente todos os tokens do contexto como texto e mais toda a conversa limitada
como áudio. Inclui instruções, tokens de controle e turnos anteriores sem adivinhar tokens pelos
milissegundos. Pressupõe que o provedor respeite os limites confirmados de contexto/truncamento/saída
e que o áudio exista apenas na conversa limitada. Todas as instruções são texto; ferramentas e
transcrição remota de entrada ficam explicitamente desligadas. A detecção de turno no servidor é
nula: respostas são criadas manualmente, após admissão. Uma configuração efetiva alterada encerra
o responsável pela sessão.

Com limite de conversa de 8.192 tokens e saída de 1.024 tokens, os tetos são US$0,839680 e
US$0,179200, respectivamente. São valores reservados, não previsões de cobrança. O custo efetivo usa
o detalhamento informado de texto/áudio/cache, aritmética inteira verificada em nano-USD e um único
arredondamento final para cima em micro-USD. O realtime cobra novamente o contexto nos turnos
seguintes; a transcrição de entrada tem cobrança separada.
[Guia de custos](https://developers.openai.com/api/docs/guides/voice-latency-cost).

<a id="completion-and-failure"></a>
## Conclusão e falha

O responsável vincula uma sessão, uma tentativa pendente e o identificador de resposta do servidor.
Preparação simultânea, criação inesperada, deltas de áudio divergentes, conclusão antiga ou duplicada
e eventos inválidos ou grandes demais não conciliam outra tentativa. Tentativas não podem ser
reutilizadas; cada responsável aceita no máximo 1.024 tentativas para limitar a memória antirreplay.
Eventos recebidos têm limite de 256 KiB.

`response.done` produz um recibo único. Uso ausente, inconsistente ou com formato novo mantém o custo
desconhecido e encerra o responsável. Custo desconhecido precisa manter a reserva integral. Excessos
conhecidos são devolvidos integralmente e também encerram o responsável. Sucesso da requisição é
independente dos dados de cobrança: uma resposta cancelada ainda pode custar dinheiro.
[Referência de eventos do servidor](https://developers.openai.com/api/reference/resources/realtime/server-events).

A cassette sintética de duas respostas cobre contexto em cache e cancelamento com cobrança; não é
uma gravação de provedor real. Execute `cargo test -p vults-chat realtime::` offline.

<a id="runtime-transport"></a>
## Transporte em execução

A V6.3.3 acrescenta uma implementação OpenAI da interface de provedor, com origem fixa em
`wss://api.openai.com/v1/realtime`, TLS nativo, sem proxy, redirecionamentos ou tentativas automáticas.
A admissão monetária antecede a leitura do chaveiro e a conexão; cada turno seguinte reserva
novamente antes de enviar PCM. A chave fica no Rust e o cabeçalho Authorization é sensível; falhas
de transporte retornam códigos sem conteúdo. Veja o [guia oficial de WebSocket](https://developers.openai.com/api/docs/guides/voice-websockets).

A versão 41 das configurações acrescenta consentimento independente `voice_realtime`, identidade
persistente do registro, limite móvel padrão de US$1 e um modelo explicitamente revisado. Zero
desativa voz remota. Só a webview de Configurações salva essa escolha. A V6.7.1 acrescenta chamadas
tipadas no bridge para essa gravação explícita e para leitura restrita às Configurações
(`voice_realtime_settings`). A V6.7.2 apresenta essas escolhas em **Ditado > Conversar com o
Zeca**; os controles de gravação e conversa ao vivo ainda não estão disponíveis.
O campo `on` retornado é a escolha salva, não um indicador de sessão ativa ou de prontidão.
A leitura retorna apenas `on`, `model` e `limitMicrousd`, com o valor codificado como texto decimal
para preservar exatamente limites grandes no JavaScript. Ela omite a identidade privada do registro
e não acessa chaveiro, dispositivo, banco ou rede. A V6.7.2 também envia valores de gravação como
texto decimal canônico, preservando toda a faixa inteira aceita sem conversão por ponto flutuante.
O comando nativo recusa valores não canônicos ou fora da faixa antes de salvar; as verificações de
autoridade, modelo e orçamento continuam valendo. O formulário aceita USD com ponto ou vírgula
decimal e até seis casas, confirma a gravação por releitura, preserva rascunhos em falhas e bloqueia
edições antigas até uma recarga explícita quando outro responsável muda a escolha salva. O backend nativo aceita canais limitados de comandos e
saída e nunca inicia captura sozinho. Uma única concessão de voz do desktop controla a conexão.
A revogação por geração cobre mudanças de escolha, bloqueio, liberação e encerramento permanente;
apagar/trocar a chave compartilhada da OpenAI também revoga conexões existentes.
Desligar/religar ou bloquear/desbloquear não renova um responsável antigo. O gate usa uma âncora
monotônica por processo, e um registro ausente/substituído não renova o consentimento.

O socket persistente aceita PCM16 mono de 24 kHz em blocos de 100 ms. A entrada tem no máximo
30 segundos por turno, a saída 120 segundos, com prazo absoluto de 120 segundos por turno e prazos
de 10 segundos para IO/configuração/conciliação. Quadros/mensagens do servidor têm limite de 256 KiB.
Fila de saída cheia, PCM inválido, tarifas vencidas ou saldo esgotado encerram o transporte.
Cancelamento/revogação interrompe IO pendente; interrupção solicita cancelamento da resposta e
aguarda uso final dentro de um prazo limitado. Nenhum áudio adicional é enviado após falha de
admissão. A publicação de prontidão/áudio revalida o responsável após esperas assíncronas. A V6.4.2
vincula reprodução nativa e encaminhamento da transcrição abaixo; captura/interface ficam na V6.7.

Recibos finais conhecidos são conciliados uma única vez, mesmo após revogação, sem uma tentativa
concorrente com custo desconhecido. Antes de qualquer inferência poder começar, uma conexão
interrompida libera sua reserva com custo comprovadamente zero. Depois que a inferência pode ter começado,
uso ausente mantém o teto inteiro. Uma admissão desacoplada nunca entregue ao transporte concilia
custo zero e libera sua reserva por seu guardião de posse. Aborto da tarefa/falha do processo após a entrega mantém a reserva
pendente. Dentro do mesmo responsável, trabalho não conciliado continua bloqueando outra
reserva. Apagar o histórico nunca zera essa contabilidade.

<a id="orphan-recovery"></a>
## Recuperação de reservas órfãs

A V6.3.4 acrescenta recuperação somente no Linux, antes que um novo responsável nativo possa
reservar. O gate adquire uma trava exclusiva não bloqueante do kernel no próprio diretório de
dados, sem arquivo de trava removível. Ele mantém a trava durante o futuro emprestado do transporte
e cada worker desacoplado de admissão, chaveiro e conciliação. Outro responsável/processo recebe
`Busy` antes de alterar a contabilidade de reservas órfãs; cancelar a tarefa não libera a referência
restante do worker à trava. O último proprietário intencional libera a trava explicitamente, para
que um descritor herdado por um filho fork/exec não relacionado não prolongue a propriedade.
A biblioteca padrão usa `flock` no Linux; veja
[File::try_lock](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock) e o
[tempo de vida da trava no Linux](https://man7.org/linux/man-pages/man2/flock.2.html).

Só depois de adquirir a posse, uma transação vinculada à identidade do registro encerra as
reservas pendentes de `voice-realtime`. O custo efetivo desconhecido continua nulo e o teto
reservado integral permanece debitado. Sua janela móvel de 24 horas começa na conciliação, nunca
antes do início original. Reinícios repetidos não alteram a expiração de uma reserva encerrada.
Traces existentes recebem resultado de erro e marcador de conciliação sem conteúdo; traces
apagados não reaparecem. Outros recursos e recibos já conciliados não mudam. Falhas de identidade,
schema, trava ou gravação recusam a entrada; uma transação malsucedida não altera reservas nem
traces. A espera inicial é limitada a 10 segundos; qualquer worker desacoplado ainda mantém a
posse até terminar.

Isso comprova que o transporte local anterior não pode mais enviar; não declara que o provedor
terminou ao desconectar nem restitui trabalho incerto. Um processo encerrado libera sua trava no
kernel; transportes abortados descartam o socket. A recuperação não está habilitada em outras
plataformas. O backend existente não tem entrada de captura oferecida ao usuário. A V6.3.5 acrescenta
retenção/recuperação nativas abaixo; captura, reprodução, continuação visual e validação com provedor
real continuam obrigatórias antes da ativação.

<a id="native-turn-retention-and-local-text"></a>
## Retenção do turno nativo e texto local

A V6.3.5 altera o serviço nativo do app para receber valores completos `realtime::Turn`,
independentemente da admissão remota. Cada valor mantém em memória de 100 ms a 30 segundos de
PCM16 mono de 24 kHz. Um turno pendente alimenta o transporte persistente em blocos de 100 ms por
uma fila de dois comandos; uma fila de quatro eventos retransmite a saída. A emissão de áudio
cede execução para que rajadas já recebidas do socket não impeçam esse consumidor no mesmo futuro
de avançar. Só um recibo final de sucesso libera o original pendente; recibos enfileirados são
processados antes de aplicar uma falha simultânea do transporte. Uma falha posterior em espera
não recupera um turno antigo.

Admissão desligada/esgotada, chave ausente, falhas de transporte/protocolo/resposta ou saída cheia
preservam o turno que falhou para recuperação local. O transporte recebe até 11 segundos para
terminar seu caminho existente de conciliação limitada; depois, seu futuro/socket é descartado
antes da decodificação. O modelo Whisper instalado e o idioma selecionados são congelados; o
reamostrador existente converte o áudio preservado para 16 kHz. Não há download, limpeza por modelo,
outro provedor nem envio automático ao Chat. A decodificação local bem-sucedida devolve o texto
inteiro, preservando espaços, para o chamador nativo oferecer como rascunho revisável. Isso não
declara que o Zeca já respondeu em texto nem que existe uma interface para essa alternativa.

Modelo ausente, falha de decodificação, saída vazia, texto inválido/grande demais ou prazo local
de espera de 30 segundos devolvem o turno original em memória com um motivo tipado. Turnos muito
curtos, inclusive após o corte de silêncio, podem não produzir palavras locais: o mínimo existente
do Whisper é 500 ms, embora a entrada remota aceite 100 ms. Áudio e transcrição não aparecem no
Debug nem em novos traces; este serviço não grava histórico de áudio ou texto.

Cancelamento, bloqueio, encerramento, mudança da chave compartilhada ou de configurações de voz,
modelo, idioma, microfone ou Zeca descartam o trabalho em vez de oferecer recuperação antiga. A
mutação compartilhada das configurações revoga a geração sob sua trava; restaurar a escolha
anterior não revive um turno antigo. O decodificador mantém a concessão nativa até a verificação
final de publicação; um worker desacoplado mantém sua própria concessão até o trabalho nativo
terminar. A espera pode expirar antes de uma carga fria não abortável do modelo terminar; o sinal
de parada suprime saída tardia e novos responsáveis de voz recebem Busy até o worker liberar sua
concessão. Não há microfone, reprodução, comando de renderer ou caminho de permissões novo. Esses
responsáveis continuam na V6.4-V6.7.

Execute `cargo test -p vults-chat realtime::transport::` e `cargo test -p vults-app realtime` offline.
Testes de WebSocket duplex exercitam quadros reais com PCM/chaves sintéticos, dois turnos,
cancelamento, revogação, falha de conciliação, pressão na fila e uso desconhecido. Não abrem escuta
nem conexão ao provedor. Nenhuma requisição paga, microfone/reprodução reais ou validação com fala
real é declarada. Os testes do controlador/decodificador nativos usam PCM sintético e substitutos
de trabalho local; qualidade real de modelo/fala e validação de captura/reprodução continuam
separadas. Falhas do backend fornecem motivos estruturados e nunca enviam silenciosamente a outro
provedor.


<a id="playback-and-heard-text"></a>
## Reprodução e texto ouvido

A V6.4.1 acrescenta a primitiva independente `audio::playback::Playback`. Ela abre somente a saída
padrão, com um worker nativo por processo, e aceita blocos de PCM16 mono a 24 kHz de no máximo um
segundo, até 120 segundos por resposta. A preparação tem prazo de três segundos; um proprietário
vive no máximo 150 segundos. O CPAL instalado e o reamostrador com estado existente suportam saída
a 8/16/24/44,1/48/96 kHz, F32/I16/U16 e um a oito canais. Dispositivos compatíveis usam, de
preferência, períodos de callback de 10 ms. Não há adição de dependência, microfone, arquivo, rede,
Settings nem renderer.

O cursor separa frames de origem aceitos, frames entregues ao dispositivo e frames estimados como
consumidos. A estimativa mapeia o instante de reprodução previsto pelo CPAL para o relógio monotônico
e avança somente pelo áudio entregue, excluindo silêncio inserido por falta de dados. Isso não prova
a reprodução acústica: dispositivo mudo, buffers de hardware e audibilidade ainda exigem aceitação
no desktop. Relógios inválidos, latência excessiva, erros de dispositivo e falhas nos limites de fila
interrompem a reprodução. O callback usa um bloqueio de fila sem espera; contenção emite silêncio
sem avançar o conteúdo entregue. A conversão roda no produtor, fora do callback.

O `finish()` do produtor termina a conversão e entra em Draining. Somente uma fila PCM vazia junto
com o fim do áudio agendado entra em Finished; uma fila temporariamente vazia não é conclusão.
`stop()`, revogação e Drop congelam o consumo e limpam a saída pendente. Amostras já entregues ao
driver ainda podem ser audíveis depois da parada; o cursor congelado exclui deliberadamente essa
cauda incerta. A destruição do dispositivo fica no worker, que mantém a propriedade exclusiva até
sair, inclusive numa chamada nativa de preparação bloqueada. O prazo não pode abortar uma chamada
de driver que não coopera nem permitir um segundo worker.

`voice::turn::heard::split` aceita segmentos de texto finalizados, ordenados e delimitados em frames
a 24 kHz, junto com o cursor de origem congelado. Segmentos inteiros reproduzidos são mantidos; um
segmento em andamento mantém somente o piso da contagem proporcional de palavras, em limites de
palavras UTF-8. É uma estimativa, sem alinhamento exato de palavras e áudio. A própria API não
fornece alinhamento preciso da transcrição depois de uma interrupção da reprodução pelo cliente;
clientes WebSocket controlam a reprodução e o truncamento do item no servidor.
[Guia de conversa Realtime](https://developers.openai.com/api/docs/guides/realtime-conversations).

O resultado puro preserva o prefixo original e o complemento não ouvido, calcula a substituição do
contexto do modelo `heard prefix [interrupted]` e fornece a cauda não ouvida uma única vez quando
restam pelo menos três palavras separadas por espaços. Uma interrupção explícita ainda marca um
produtor parado depois da drenagem do áudio recebido. Texto vazio ou sem reprodução nunca vira uma
resposta inteira ouvida. A dica não faz requisição, não repete ferramenta nem concede permissão.
Ela não é um rótulo traduzido de interface.

A V6.4.1 fornece a primitiva independente; a V6.4.2 acrescenta seu proprietário nativo no app,
descrito abaixo. A primitiva deve continuar vinculada a esse proprietário de consentimento/revogação.
V6.5-V6.7 mantêm cancelamento de eco, ferramentas, captura e conversa visível; o microfone da
conversa continua indisponível.

Testes offline cobrem consumo/latência, falta de dados, drenagem, parada, relógios inválidos,
cancelamento, reamostragem e cálculo de palavras/histórico. Um teste ALSA nulo explícito em processo
filho exercita preparação, conversão, callback, drenagem e destruição reais do CPAL sem usar os
alto-falantes do desktop. Ele comprova somente a integração nativa; tempo de alto-falante/fone real,
interrupção, qualidade da fala e aceitação com provedor real continuam em aberto. Execute o teste
nulo somente com uma configuração ALSA privada contendo `pcm.!default { type null }`,
`ALSA_CONFIG_PATH` apontando para ela e `VULTS_PLAYBACK_NULL_TEST=1`:
`cargo test -p vults-audio native_null_output -- --ignored`.


<a id="native-playback-and-context-repair"></a>
## Reprodução nativa e reparo de contexto

A V6.4.2 conecta ao CPAL o serviço nativo existente de turnos completos. O primeiro turno completo
autorizado inicia a conexão pelo gate; somente Ready remoto confirmado abre um worker de saída
silencioso, e a entrada só é enviada depois que a saída informa Playing. Falha de dispositivo mantém
a entrada pendente para a recuperação local de texto existente. Um Busy transitório de destruição
aguarda no máximo três segundos. Cada worker de saída mantém a concessão da sessão nativa até a
destruição do dispositivo, inclusive em preparação desacoplada ou bloqueada, para que aborto/revogação
não sobreponha uma nova sessão ao trabalho nativo antigo.

Comandos nativos e sinais de fala/prova carregam a geração exata do turno. Comandos antigos são
descartados; somente uma prova confirmada correspondente congela a reprodução. Notificações das
ações Listen/Probe não concedem acesso ao microfone nem iniciam captura. A primeira saída consumida
controla a trava inicial; conclusão do produtor pede drenagem, e drenagem confirmada controla a
margem final. A entrada do app mantém interrupção por fala desligada até chegar a política de
dispositivo/AEC da V6.5. Casos com fones simulados exercitam o caminho disponível no proprietário
sem afirmar que interrupção por alto-falantes é segura.

O caminho de resposta falada aceita um item de áudio de assistente, com parte de conteúdo no índice
zero, por resposta. IDs de tentativa, resposta e item são vinculados entre PCM, transcrição, saída
final e histórico; IDs de resposta e item não podem ser reutilizados na sessão limitada. IDs
alterados, outra parte de conteúdo, mistura de ferramentas/áudio, saída somente em texto, términos duplicados de
áudio/transcrição, texto malformado e transcrições finais ausentes/divergentes encerram o proprietário.
O texto é limitado a 64 KiB e o áudio a 120 segundos. Uma transcrição finalizada cobre todo o áudio
recebido como um único segmento estimado; o provedor não fornece tempos precisos de palavras.
A conciliação de cobrança ainda usa o recibo final exato mesmo que a validação posterior da saída falhe.

Na interrupção confirmada, o cursor congela uma vez e é arredondado para baixo em milissegundos
inteiros de origem. Mais PCM correspondente pode ser contabilizado, mas não é reproduzido. Um
produtor ativo recebe `response.cancel` direcionado; depois do recibo final e da conciliação, o item
exato recebe `conversation.item.truncate`. Um produtor já concluído não é cancelado novamente.
Somente confirmação correspondente de item/índice de conteúdo/fim do áudio deixa a conexão pronta.
Confirmação errada, duplicada ou ausente encerra a conexão sem restituir uso conciliado nem admitir
outra resposta.
[Referência de eventos do cliente](https://developers.openai.com/api/reference/resources/realtime/client-events).

O proprietário nativo substitui somente seu registro de assistente correspondente em memória pelo
prefixo estimado como ouvido mais `[interrupted]`. Ele emite um registro History vinculado à
identidade após drenagem ou interrupção confirmada; não grava o banco do Chat nem persiste
áudio/transcrições. Uma cauda de pelo menos três palavras vira uma dica de reparo limitada e citada,
consumida somente no próximo `response.create` admitido de forma independente. Ela nunca gera um
turno sozinha, repete ação nem habilita ferramentas. As instruções existentes da persona continuam
presentes; a dica é dado de conversa, sem autoridade.

Se o socket falhar depois de uma resposta bem-sucedida conciliada, a saída válida em buffer ainda
pode terminar de tocar. Sinais de fala/prova continuam ativos durante essa drenagem. Se houver
interrupção nesse momento, ou se a confirmação do truncamento falhar após a conciliação, o histórico
ouvido correspondente é emitido com `server_synchronized=false`; depois, o erro terminal encerra o
proprietário. Esse registro não autoriza continuação. Cancelamento, bloqueio, escolhas alteradas e
saída fechada suprimem publicação e recuperação antigas; param a saída antes da limpeza limitada do
transporte. Uma entrada concluída nunca é recuperada nem repetida como novo turno do usuário.
Chamadores retêm qualquer captura ainda não aceita até sua notificação Generate.

A V6.5.2 acrescenta a entrada Linux separada `run_routed`, protegida antes da captura nativa pelo
consentimento existente das Configurações, cancelamento e autoridade da geração. Ela fixa os dois
streams nativos em um par AEC ou dispositivos escolhidos explicitamente, mantém sua concessão até
o fim da destruição e emite eventos Input limitados somente em memória ao chamador nativo. O
chamador deve montar e confirmar turnos completos; a captura nunca é enviada automaticamente.
A política explícita de fones/AEC fica separada da verificação temporária da rota; uma rota não
verificada limpa fala/provas retidas antes de eventos adiados da reprodução. A entrada CPAL comum
continua sem interrupção. Não há comando do renderer nem conversa de voz visível. O backend da
V6.4/V6.5 está entregue; veja [Cancelamento de eco](echo-cancellation.md) para evidências de
roteamento, relógio e grafo privado. A próxima etapa é ferramentas na V6.6, depois captura/interface
na V6.7. Tempo de dispositivo/acústica, fala real e aceitação com provedor real continuam em aberto
antes da ativação. Testes offline de socket e proprietário
nativo cobrem interrupção antes/depois do fim do produtor, confirmações exatas, gerações antigas,
admissão do reparo, falha, revogação, aborto e recuperação. A configuração ALSA nula isolada acima
também executa o driver nativo completo com um provedor sintético:
`cargo test -p vults-app native_null_conversation -- --ignored`.
Esse teste exercita CPAL real até o histórico em memória sem microfone, alto-falante do desktop,
chave nem conexão com provedor. Ele não representa aceitação de fala ou provedor real.


<a id="local-commands-before-realtime-upload"></a>
## Comandos locais antes do envio realtime

A V6.6.1 acrescenta admissão de comandos no Linux ao proprietário nativo. `Turn::with_transcript`
vincula uma transcrição local finalizada ao mesmo PCM capturado, com limite de 4 KiB e sem caracteres
de controle, exceto espaços em branco. Seu Debug informa apenas tamanhos. O proprietário da captura
nativa é responsável por essa transcrição local; captura/VAD/montagem de turnos e controles visíveis
continuam na V6.7. A ausência de transcrição falha antes de construir o provedor e pode retornar a
recuperação local de texto revisável existente. Nunca envia áudio sem classificação remotamente
nem executa texto recuperado automaticamente.

O proprietário existente do estado no runtime recebe uma solicitação cancelável, verifica o mesmo
consentimento/geração da conversa e a opção de comandos sob Configurações e reutiliza o dispatcher
da V8 com estado provisório. Cada conversa tem seu próprio contexto limitado de 90 segundos para
repetir/desfazer/esclarecer, separado do ditado independente. Alterar a opção de comandos também
revoga as gerações de conversas existentes, inclusive ao reativar. A admissão no runtime verifica
um prazo absoluto de três segundos com tempo atual após adquirir as travas; solicitações expiradas,
canceladas, consumidas ou sem destinatário não alteram o bando. O chamador nativo verifica
cancelamento/fechamento da saída e descarta a admissão pendente no prazo ou aborto.

Somente um resultado NotCommand não reconhecido pode seguir ao transporte realtime. Comandos
reconhecidos, respostas a cards recusadas, ações desativadas/indisponíveis e perguntas de
esclarecimento ficam locais. Leituras reutilizam o formatador existente do bando com cache e
remoção de segredos; não iniciam busca em conectores. Efeitos usam o executor existente e a lista
exaustiva de Intents permitidos por voz. A V6.6.3 acrescenta os recibos nativos abaixo: somente
conclusão confirmada pode informar Comando concluído; encaminhar uma abertura continua sendo
Comando solicitado. Um resultado local emite um evento Local vinculado à
identidade, termina sem Generate e aposenta a geração da entrada. Uma duplicata enfileirada não
executa novamente nem é enviada. Uma entrada tratada nunca vira fallback automático de modelo/Chat
após um erro.

Testes simulados do proprietário cobrem entrada local antes de construir o provedor, continuação
local em conexão existente, gerações antigas/duplicadas e cancelamento durante a admissão. Testes
do dispatcher cobrem recusa de permissões, desativação, ambiguidade, contexto separado, leituras
locais traduzidas, prazo vencido e mutação única. Esses testes não usam microfone, ação do desktop,
chave nem provedor. A V6.6.2 acrescenta as ferramentas do bando pelo modelo abaixo; consulta fica
na V6.6.3. A V6.6 completa, ativação do renderer e aceitação de fala/provedor real continuam em aberto.


<a id="native-command-receipts"></a>
## Recibos de comandos nativos

A etapa de conclusão da V6.6.3 usa o executor existente com um recibo por efeito. Uma mudança somente
no core termina na confirmação do estado; a persistência de projetos termina somente depois de
salvar as Configurações com sucesso. Criar o processo do editor, encaminhar ao gerenciador de
arquivos e solicitar ativação de janela continuam como Solicitado: essas APIs não comprovam que a
janela solicitada ficou visível nem que seu inicializador teve sucesso depois. Uma falha conhecida
informa que algumas alterações podem já ter ocorrido, sem alegar reversão de tudo. Lotes que misturam
leitura em cache e ação esperam todos os efeitos e preservam a resposta em cache somente quando
todos os recibos confirmam conclusão; encaminhamento, falha ou prazo vencido a substituem pelo
estado correspondente. Nenhuma resposta do modelo transforma uma ação encaminhada ou desconhecida
em Concluído.

O proprietário nativo fornece um único prazo absoluto de três segundos. Admissão/recibos reservam
uma margem de 50 ms para publicação; a espera por recibos também tem limite de dois segundos após
a admissão. Conclusão desconhecida informa Solicitado se o chamador ainda estiver ativo.
Cancelamento, bloqueio, escolhas revogadas, saída fechada ou prazo do proprietário suprimem
publicação tardia. Cada trabalhador enfileirado verifica novamente a autoridade exata de
Configurações/geração, opção de comandos, chamador ativo, destinatário do recibo e prazo antes de
iniciar. Cancelamento não desfaz trabalho externo já iniciado. O trabalhador nativo mantém a
concessão exclusiva da conversa durante a chamada à API de abertura, inclusive se ela bloquear.
Após o encaminhamento, esse trabalhador pode sair enquanto o editor aberto continua de forma
independente. Essas APIs não confirmam cancelamento; vencer o prazo nunca admite trabalhadores
nativos sobrepostos.

A captura original e todas as cópias compartilham o consumo. O proprietário do estado a marca antes
de liberar a admissão, mesmo se uma confirmação for perdida. Resultados locais, recibos consumidos
de ferramentas, respostas finalizadas e interrupções confirmadas também a consomem. Nem uma nova
geração nem um erro enviam uma captura consumida para recuperação local, execução do parser ou
envio remoto. Capturar explicitamente outro comando continua possível; essa proteção não altera os
comandos deliberados de repetir/desfazer da V8. O limite exaustivo de Intents por voz da H1 continua
igual: voz não decide cards.

Testes offline cobrem recibos concluídos/encaminhados/com falha/ausentes, lotes mistos, publicação de
prazo vencido pelo proprietário nativo, revogação/prazo/perda do chamador na fila, concessões retidas
e repetição de cópias antes/depois da construção do provedor e de interrupção. Não há ação do
desktop, microfone nem provedor real. O backend de consulta ao Claude está descrito abaixo;
a qualificação do Codex continua na V6.6.3. Montagem de captura e interface visível ficam na V6.7.


<a id="read-only-tools-in-realtime"></a>
## Ferramentas somente de leitura no realtime

A V6.6.2 conecta as cinco leituras existentes de `flock_tools`: `flock_list`, `session_detail`,
`pending_asks`, `usage` e `github_snapshot`. Elas leem somente dados em cache; o modelo não executa
Intent, responde a card, abre sessão de agente, muda configurações nem busca dados de conector.
As especificações são congeladas uma vez, confirmadas exatamente na configuração da sessão e
incluídas em cada resposta admitida. O Plan padrão continua sem ferramentas; o app nativo fornece
esse registro somente de leitura explicitamente.

O subconjunto suportado é uma resposta somente de função por etapa, seguida eventualmente por
uma resposta falada separada. A OpenAI documenta `response.done` com a chamada completa e
`function_call_output` vinculado ao ID da chamada, seguido de outro `response.create`.
[Ciclo oficial de ferramentas Realtime](https://developers.openai.com/api/docs/guides/realtime-mcp).

O Vults verifica em conjunto resposta, tentativa, item, chamada, nome, argumentos transmitidos,
conclusão do item e saída final. As chamadas precisam concluir com sucesso; respostas canceladas,
com falha ou incompletas nunca invocam ferramenta. Mistura de áudio/ferramentas, chamadas paralelas,
identidades alteradas, chaves ausentes/extras/duplicadas, valores que não são strings e nomes não
oferecidos encerram o caminho. Argumentos têm no máximo 4 KiB, valores string até 256 bytes e até
128 deltas. IDs de chamada/item continuam consumidos entre turnos; uma resposta nova não os repete.
No máximo oito leituras cabem no prazo original de 120 segundos da mesma entrada.

O recibo exato é conciliado de forma persistente antes da leitura. Uma reserva separada para a
continuação deve ser aceita antes da invocação. Só então o transporte executa a Tool existente e
envia um resultado seguido da nova resposta admitida, em ordem no WebSocket, sem reenviar áudio
nem repetir a chamada. Custo desconhecido da continuação mantém seu teto; trabalho comprovadamente
não enviado é conciliado como zero. Etapas de ferramenta não consultam a chave novamente nem
ignoram o gate.

Resultados são envelopes JSON inertes com `is_error`, `outside`, `text` e `truncated`. Envelopes de
sucesso e erro têm até 16 KiB após os escapes JSON, antes de o evento externo codificar essa string.
Prefixos UTF-8 preservam JSON válido ao encurtar texto. Erro de leitura retorna uma vez como dado,
nunca fallback do parser local. As marcações existentes de origem externa identificam conteúdo do
GitHub; as instruções mantêm todo resultado como dado citado, sem autoridade sobre permissões.

Um recibo de ferramenta é diferente da conclusão falada. O proprietário nativo publica sua
conciliação, libera a entrada original e permanece em Thinking com a mesma saída silenciosa e
geração. Não emite histórico ouvido, drenagem nem Listen por uma leitura. O áudio final tem sua
própria identidade de tentativa/item e usa o caminho existente de transcrição, consumo, truncamento
e histórico. Reutilizar uma tentativa de ferramenta é recusado antes de enfileirar áudio. Falha
após consumir uma resposta de ferramenta não devolve a captura original para recuperação ou
repetição; encerra o proprietário sem afirmar que uma ação concluiu.

Testes offline de duplex/proprietário nativo cobrem continuações com sucesso/erro, reservas
separadas, recibos conhecidos/desconhecidos, orçamento insuficiente, chamadas inválidas/duplicadas/
canceladas, revogação durante a leitura, limite com escapes, especificações exatas e áudio/histórico
finais após liberar a entrada. Usam credenciais/PCM sintéticos, sem provedor externo, microfone nem
ação do desktop. O backend de consulta ao Claude abaixo estende esse registro somente com
consentimento separado. A qualificação do Codex continua na V6.6.3, seguida da captura/interface na
V6.7. Fala física, compatibilidade com provedor e aceitação acústica continuam pendentes.


<a id="text-consultation"></a>
## Consulta textual

No Linux, o proprietário nativo da V6.6.3 acrescenta `consult_claude` somente com a opção separada
`voice_consultation`, desligada por padrão, e a identidade original do orçamento persistente.
Somente Configurações pode alterá-la; a alteração revoga a geração da voz mesmo se a escolha anterior
for restaurada antes da próxima verificação. O renderer ainda não tem entrada de conversa nem
controle de consulta. A oferta padrão continua nas cinco leituras do bando em cache. O backend
também suporta `consult_codex` com sua própria opção `voice_codex_consultation`, desligada por
padrão, e identidade original `voice_codex_consultation_budget`. Seu comando exclusivo de
Configurações invalida a mesma geração da voz quando uma escolha muda. O Codex está bloqueado
pela capacidade compartilhada `SUBSCRIPTION_AUTH_QUALIFIED=false`: ligar é recusado antes de
acessar configurações/ledger, consentimento salvo não oferece sua ferramenta nem abre o gate de
rede, e o helper textual recusa antes de acessar a CLI ou pasta temporária. Desligar uma escolha
salva continua permitido. Não há controle do renderer nem entrada de captura para nenhum provedor.

Cada consulta fornece uma pergunta não vazia de até 256 bytes, sem caracteres de controle. O caminho
do Claude reutiliza seu cliente textual com Haiku, uma nova pasta de trabalho privada e vazia,
seleção vazia de ferramentas integradas, exclusão de ferramentas MCP, configuração MCP estrita,
sem configurações de usuário/projeto, skills desativadas e sem sessão salva. Expansão de referências
a arquivos, descoberta de CLAUDE.md, memória automática e tráfego não essencial da CLI são
desativados explicitamente. Clientes anteriores à 2.1.299, versões desconhecidas e outras séries
major/minor são recusados antes de iniciar uma pergunta: clientes antigos podem ignorar esses
controles silenciosamente. O helper compartilhado também aplica essa verificação de versão aos
chamadores existentes de diário/memória/sono. Os [controles da CLI Claude](https://code.claude.com/docs/en/cli-reference)
e [controles de ambiente](https://code.claude.com/docs/en/env-vars) definem essas opções. O Vults não
usa o modo bare, que ignora a autenticação da assinatura, nem enfraquece políticas organizacionais
gerenciadas. Esse adaptador não é um sandbox do sistema operacional; políticas gerenciadas
obrigatórias ainda podem carregar hooks ou acessar seus próprios serviços. O comportamento de
clientes gerenciados precisa de qualificação separada antes da ativação.

O Vults fornece somente a pergunta, sem contexto de projeto/sessão nem ferramenta de leitura do
projeto. Ele não retoma agente ativo, delega trabalho nem herda histórico, ferramentas, políticas ou
acesso total do Chat. O gate
de cada provedor, `voice-consultation` (Claude) e `voice-codex-consultation` (Codex), reserva
de forma durável até 20 tentativas por 24 horas móveis,
incluindo falhas; reservas monetárias do realtime não pagam nem autorizam essas tentativas da CLI.
Reinício, alternância do consentimento, limpeza do histórico ou troca do ledger não renovam a
cota silenciosamente. Traces contêm recurso/provedor/modelo, tempos e resultado, nunca pergunta ou
resposta. O Vults não repete consultas com falha nem troca de provedor; a CLI instalada mantém seu
próprio comportamento de conexão e autenticação.

Os dois provedores compartilham uma vaga. Um trabalhador em segundo plano mantém a vaga ocupada e a concessão exclusiva da voz durante a
preparação e até confirmar a saída do processo. O horário original da chamada ancora o prazo de
cancelamento de 60 segundos, incluindo preparação e verificação da versão. Aos 1,5 segundo ele pode
emitir um aviso Consulting traduzido, identificado pelo pedido, no canal de saída nativo atual.
Respostas rápidas não emitem esse aviso; ele é dado de estado, não fala gerada nem histórico ouvido.
Cancelamento, perda do chamador, bloqueio, geração ou saída revogada suprimem respostas/avisos
antigos e solicitam encerramento do processo. O trabalhador espera a saída em vez de liberar a
posse após apenas solicitar o encerramento. IO local ou destruição que não possam ser interrompidos
podem mantê-lo ocupado além do prazo; outra consulta/sessão não sobrepõe esse trabalho retido.

O prazo da ferramenta realtime pode chegar explicitamente a 60 segundos; leituras em cache mantêm
seu limite de dez segundos, e o prazo original de 120 segundos da cadeia nunca é estendido.
Sucesso/erro textual usa o envelope inerte limitado existente com `outside=true`; não tem autoridade
sobre permissões. Um recibo consumido impede recuperação ou repetição da entrada mesmo se a
consulta ou continuação falhar. O aviso de 1,5 segundo ainda não tem consumidor no renderer; sua
exibição e controles de adesão ficam na V6.7.

Testes offline exercitam o tempo do aviso, prazos originais durante admissão lenta, posse durante
cancelamento e destruição, saída confirmada do processo, limite da resposta, argumentos, controles/
versão do cliente, admissão persistente independente e atualização do consentimento. Eles usam
processos filhos locais sintéticos, sem modelo, microfone nem ação do desktop. Vínculo atômico com
a assinatura e compatibilidade com cliente real/gerenciado continuam em aberto para ativação do
Codex e aceitação real. O backend protegido pode ser publicado; a engenharia da V6.7 segue com
Codex indisponível.


<a id="codex-qualification"></a>
### Qualificação do Codex

O candidato com Codex CLI 0.163.0 instalado e `gpt-6.1-sol` não cumpre esse contrato no ambiente
qualificado. Ignorar configurações/regras do usuário, escolher execução efêmera somente de leitura,
excluir documentos do projeto e desativar recursos de integração/ferramentas ainda deixa
`additional_tools` inline e um catálogo de skills do ambiente na primeira requisição ao modelo.
A ausência do campo `tools` no nível superior é insuficiente: definições de ferramentas do modo de
código podem estar em `input`. Isso é evidência contra esse candidato, não uma afirmação sobre toda
integração com o Codex. Os [controles da CLI](https://learn.chatgpt.com/docs/developer-commands?surface=cli)
e [controles de configuração](https://learn.chatgpt.com/docs/config-file/config-reference)
documentados não comprovam, por si só, o contrato de receber somente o texto fornecido.
Esse candidato original não é usado pelo adaptador nativo.

O novo helper de CLI, ainda sem ativação, qualifica um contrato mais restrito no cliente exato
0.163.0. Ele fornece um catálogo mínimo de modelo controlado pelo app, em vez de herdar metadados
mutáveis: capacidades de shell, edição, busca, modo de código e colaboração são desativadas.
Instruções fixas do chamador substituem as instruções do fornecedor; documentos do projeto,
catálogos de skills e injeção de contexto de ambiente/apps/permissões são desativados. Esses
controles foram verificados no cliente instalado; alguns são internos, por isso versões
atualizadas/desconhecidas são recusadas em vez de presumidas seguras. O helper não usa outro
provedor de API, lê credenciais nem altera a configuração do usuário.

A entrada é uma pergunta entre aspas JSON, cujos caracteres de cifrão e arroba usam escapes Unicode
de JSON. O modelo recebe instruções para responder ao valor decodificado. Isso preserva a pergunta
e impede o pré-processamento de referências explícitas a skills/arquivos pela CLI: desativar apenas
o catálogo não impede essa expansão. Perguntas ficam em stdin, nunca no catálogo nem nos argumentos
do processo. A saída reutiliza o executor limitado de processos filhos; o cancelamento espera a
saída do filho. Respostas malformadas, com falha, ferramentas ou sem conclusão não viram uma resposta.

Os testes explícitos para desenvolvedores rodam em namespace de rede contendo somente loopback,
com credenciais e perguntas sintéticas. Candidatos rejeitados não recebem resposta de modelo.
O helper isolado precisa oferecer zero ferramentas, excluir marcadores privados de projeto/arquivo/
skill nomeada e processar uma resposta textual fixa do servidor falso pela CLI real e pelo parser.
Instruções capturadas e credenciais não são salvas nem registradas, e nenhum provedor externo é
acessado. Regressões offline normais verificam os dois registros de ferramentas, a ida e volta da
entrada, restrições de versão e rejeição de respostas. Um teste de autenticação sintética, isolado
no sistema de arquivos e na rede, também comprova que a verificação de status exclusiva de ChatGPT
recusa autenticação de API sem alterá-la; a pasta pessoal/autenticação/keyring originais ficam
ocultos, e os valores de HOME/CODEX_HOME nunca são reatribuídos. O mesmo teste caracteriza a
exclusão de autenticação por exec forçado somente nessa montagem sintética descartável. Veja o
[comando de reprodução no roadmap](../../dev/plan-dictation.md#v6-talk-with-zeca-k8).

O backend nativo agora conecta esse helper por consentimento próprio, cota persistente e traces
sem conteúdo, usando a posse compartilhada da consulta, prazo, cancelamento e aviso. Consentimento
do Claude nunca autoriza Codex, e esgotar a cota de um provedor não consome a do outro. Não há
ativação visível nem chamador de captura. O helper preserva a autenticação da CLI instalada, remove
sobrescritas de chave de API/URL base/sessão herdadas do ambiente e exige sucesso na verificação
não mutante `login status` exclusiva de ChatGPT antes de iniciar exec. Isso é uma verificação prévia,
não um vínculo atômico contra uma troca externa concorrente de login. A regressão repetível com
autenticação sintética confirma que forçar o método de login em exec apaga autenticação de API
incompatível; o Vults não deve usá-lo. A [leitura de conta do App Server](https://learn.chatgpt.com/docs/app-server)
documentada não fornece identidade estável de conta/workspace para vincular um exec separado.
Garantir uso exclusivo da assinatura durante uma troca externa de login continua bloqueando a
ativação. Compatibilidade com cliente real/gerenciado também segue pendente; a ativação
do Codex permanece aberta, e a engenharia da V6.7 segue com esse recurso indisponível.


<a id="managed-policy-candidate"></a>
### Candidato com política administrada

Um candidato separado usa requisitos locais do sistema `allowed_login_methods = ["chatgpt"]`, em
vez de forçar o método de login do exec. A [configuração administrada](https://learn.chatgpt.com/docs/enterprise/managed-configuration)
aplica essas restrições antes de carregar credenciais. O [guia de segurança](https://learn.chatgpt.com/docs/enterprise/security-cookbook)
exige proteger o arquivo de política e sua pasta contra alterações pelo usuário do Codex; sessões
existentes não herdam automaticamente uma política recém-instalada. O Vults não instala nem altera
essa política do sistema.

A regressão com o cliente instalado agora isola tanto a pasta pessoal quanto `/etc`, usando uma
montagem de sistema vazia nos controles e uma política sintética nos casos administrados. Nove
cenários cobrem status comum e restrito, exec forçado destrutivo, transmissão de bearer de API sem
política, recusa de API com política sem apagar autenticação, status ChatGPT sintético, substituição
por API entre status e um exec novo, e substituição após o primeiro pedido autenticado com ChatGPT
receber um 401 sintético. O último caso precisa observar uma nova tentativa; somente o bearer
ChatGPT sintético original pode aparecer, nunca a chave de API substituta. Uma nova tentativa de
handshake WebSocket ou fallback HTTP pode satisfazer essa contagem; ela não qualifica a recarga de
credenciais. O servidor local encerra
pedidos com um 400 fixo; não há resposta de modelo, provedor externo, credencial real nem registro
de cabeçalhos/conteúdo de autenticação. Cada processo tem saída confirmada, e o servidor limitado
é encerrado antes de remover suas montagens privadas.

Essas são observações sintéticas com armazenamento em arquivo na CLI 0.163.0, não elegibilidade de
produção nem vínculo com uma conta específica. Verificação em execução da política efetiva e sua
origem protegida, comportamento do keyring, casos mais amplos de renovação/falha e compatibilidade
real continuam pendentes. A capacidade de ativação permanece falsa, inclusive em máquinas sem
política administrada; nenhum recurso nativo ou visível é habilitado.


<a id="guarded-backend-publication"></a>
### Publicação do backend protegido

A verificação consolidada permite publicar o backend com consulta ao Codex desativada; ela não
qualifica nem habilita esse adaptador opcional. Verificações isoladas de metadados agora validam
requisitos ausentes/exclusivos de ChatGPT e seleção de armazenamento em arquivo/keyring sem
renovar tokens. Keyring obrigatório com DBus indisponível recusa tanto autenticação de API armazenada quanto um
arquivo ChatGPT sintético que seria válido. O mesmo arquivo ChatGPT é aceito com armazenamento
em arquivo, portanto os casos de keyring verificam recusa sem fallback; nenhum arquivo sintético
é alterado. A máquina atual não tem arquivo protegido de requisitos do sistema,
portanto nenhuma rota administrada é habilitada.

Um servidor falso somente HTTP usa um provedor personalizado que exige a autenticação OpenAI da
CLI. Ele retorna um POST401 válido após substituir autenticação ChatGPT sintética por uma chave de
API sintética. Tanto o caso administrado quanto o sem política encerram após um POST e preservam
o arquivo substituto sem enviar sua chave. Isso cobre recusa limitada, não renovação bem-sucedida
nem recarga de credenciais. O WebSocket do provedor integrado e keyring disponível/bloqueado ainda
precisam de qualificação. Nenhuma conta real, keyring, provedor ou política do sistema é alterada.

O setter de microfone em produção agora invalida a geração da voz nativa antes de publicar um
novo dispositivo; restaurá-lo não revive o turno antigo. Escolhas idênticas e falhas ao salvar
mantêm a geração atual. Uma regressão do setter usa processo separado e configuração privada.
Os testes de corrida/IME/saída das perguntas agora rodam na CI junto aos demais checks do renderer.

A V6.7 pode montar captura e controles visíveis sobre o backend protegido. Aceitação com fala
física, provedor e interface nativa pertence a essa integração; K8 segue aberto até passar.
Consentimento, ferramentas, gate e execução do helper Codex continuam indisponíveis até concluir
sua própria qualificação. Veja o [checkpoint de publicação](../../dev/plan-dictation.md#v663-publication-checkpoint-2026-10-20).
