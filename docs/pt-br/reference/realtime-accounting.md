# Contabilidade realtime
<!-- source: bf2ab127083e -->

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
admissão. A publicação de prontidão/áudio revalida o responsável após esperas assíncronas. Consumo
pelo dispositivo/reprodução e roteamento da transcrição continuam na V6.4/V6.7.

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
restante do worker à trava. A biblioteca padrão usa `flock` no Linux; veja
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
plataformas. O backend existente não tem entrada de captura oferecida ao usuário; retenção nativa
do turno local/alternativa em texto e validação com provedor real continuam obrigatórias antes da
ativação.

Execute `cargo test -p vults-chat realtime::transport::` e `cargo test -p vults-app realtime` offline.
Testes de WebSocket duplex exercitam quadros reais com PCM/chaves sintéticos, dois turnos,
cancelamento, revogação, falha de conciliação, pressão na fila e uso desconhecido. Não abrem escuta
nem conexão ao provedor. Nenhuma requisição paga, microfone/reprodução reais ou validação com fala
real é declarada. O futuro chamador nativo precisa manter seu turno local até a conclusão para a
alternativa em texto; falhas do backend fornecem motivos estruturados e nunca enviam silenciosamente
a outro provedor.
