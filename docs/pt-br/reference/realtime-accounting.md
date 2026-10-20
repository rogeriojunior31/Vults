# Contabilidade realtime
<!-- source: 56e2fdba12e2 -->

A V6.3.2 fornece um contrato offline de protocolo em `chat::realtime`. Ele não abre conexão, lê
chaves nem captura áudio. `voice-realtime` continua desativado no app. Uma mensagem preparada não
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

A V6.3.3 continua responsável por chaveiro, consentimento independente, gate monetário persistente,
relógios monotônicos, transporte PCM limitado e roteamento da reprodução, cancelamento de streaming,
recuperação de reservas desconhecidas e alternativa em texto. Só a integração em execução pode
comprovar que o trabalho cobrável parou antes de conciliar o registro persistente. Nenhuma requisição
paga ou validação com fala real é declarada nesta etapa.
