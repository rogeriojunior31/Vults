# Contabilidade realtime
<!-- source: 2d0f2e31cf55 -->

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
desativa voz remota. Só a webview de Configurações salva essa escolha; ainda não há controle visual,
chamada no bridge ou entrada de microfone. O backend nativo aceita canais limitados de comandos e
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

O protocolo sem ferramentas aceita um item de áudio de assistente, com parte de conteúdo no índice
zero, por resposta. IDs de tentativa, resposta e item são vinculados entre PCM, transcrição, saída
final e histórico; IDs de resposta e item não podem ser reutilizados na sessão limitada. IDs
alterados, outra parte de conteúdo, ferramentas, saída somente em texto, términos duplicados de
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
