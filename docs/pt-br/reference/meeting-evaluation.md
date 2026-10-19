# Avaliação de falantes em reuniões
<!-- source: 02f92da795f3 -->

Esta ferramenta offline de desenvolvimento mede os turnos de falantes anônimos em relação a
anotações fornecidas. Ela nunca ativa identificação de falantes no Vults. Não grava, lê áudio,
carrega modelos, baixa arquivos nem chama provedores. O experimento nativo opcional pode incluir
o mesmo relatório em uma análise local; a aceitação de reuniões reais em PT/EN continua aberta.

<a id="run"></a>
## Execução

```sh
cargo run -p vults-spike --bin vults-meeting-eval -- reference.json prediction.json
```

Os dois arquivos usam o schema 1 e o mesmo `clock`: identificadores anônimos `recording` e
`source`, `wavSha256` em minúsculas e `durationMs`. Durações iguais sem identidade/hash
correspondentes são recusadas. A ferramenta compara esses relógios declarados; ela não pode
verificar um áudio que nunca abre.

A referência também declara `language` (`en` ou `pt`), `annotation` (`human`, `controlled` ou
`synthetic`) e `split` (`calibration` ou `evaluation`). São declarações de procedência, não uma
verificação independente de consentimento, qualidade das anotações ou separação do corpus.
Cada turno tem `startMs`, `endMs` e um inteiro anônimo `speaker`. Na previsão, `speaker: null`
significa atividade sem atribuição; os turnos de referência precisam de identidade. Palavras
reconhecidas, nomes pessoais e caminhos de áudio não devem constar nesses arquivos. Veja os
[exemplos sintéticos](../../../crates/spike/fixtures/meeting-eval/).

Os tempos são intervalos semiabertos em milissegundos no relógio da própria fonte. Nunca concatene
anotações do microfone e do sistema nem deduza identidades comuns por rótulos numéricos iguais.
O limite é um MiB por arquivo, 4.096 turnos por lado, oito falantes conhecidos por lado e 180
segundos por fonte, como no worker nativo atual. Campos desconhecidos, limites inválidos e schemas
não suportados são recusados. No Linux, links simbólicos e entradas não regulares são recusados
sem bloquear em um FIFO.

<a id="scoring-profile"></a>
## Perfil da medição

O perfil é fixo: toda a fonte é avaliada, com **tolerância zero nas bordas** e sobreposição incluída.
Ele segue o pareamento ótimo de rótulos um a um e a decomposição de erros descritos no
[pyannote.metrics](https://pyannote.github.io/pyannote-metrics/_modules/pyannote/metrics/diarization.html),
com um solucionador exato por subconjuntos implementado de forma independente. Tempo de falantes
perdido, falso alarme e confusão são informados separadamente; DER é a soma dividida pelo tempo
de falantes de referência. Vozes sobrepostas na referência contam separadamente. Atividade
duplicada para o mesmo rótulo conta uma vez. As taxas são razões: `0.4` significa 40%; falsos
alarmes podem elevar o resultado acima de 100%.

`falseAlarmSpeakerMs` também é dividido em `falseAlarmWithoutReferenceSpeakerMs` (nenhuma voz
de referência ativa) e `excessOverlapSpeakerMs` (mais vozes previstas que vozes de referência
ativas enquanto há fala anotada). Os dois somam o total, incluindo atividade prevista sem
atribuição. Isso distingue erros de detecção de fala de sobreposição excessiva sem mudar DER
nem o pareamento de rótulos. Ausência na referência não comprova silêncio acústico: as anotações
podem omitir risadas ou outros sons vocais.

Atividade prevista sem atribuição é uma extensão explícita: ela representa um fluxo de atividade
sem pareamento, nunca uma identidade candidata. Durante a fala, é penalizada como confusão ou
sobreposição perdida; no silêncio, é falso alarme. Vários turnos nulos sobrepostos contam como um
fluxo desconhecido; não permitem determinar quantas pessoas desconhecidas falaram.
`unknownActivityMs` informa sua união. Uma referência só de silêncio tem `der: null`, com o tempo
de falso alarme ainda visível.

O relatório preserva os metadados de anotação/idioma/divisão e os hashes SHA-256 da serialização
canônica do schema 1 de cada entrada, sem considerar espaços de formatação. Não há limite de
aceitação e `productionQualified` é sempre `false`. Uma referência declarada humana ou um
resultado sintético perfeito não qualifica a produção. Congele um corpus humano PT/EN revisado
e consentido, divisões separadas e limites de aceitação antes de ajustar o modelo. Eco, ruído,
turnos curtos/baixos, sobreposição e aceitação de recursos/relógios em sessões longas continuam
como requisitos separados.

<a id="native-spike"></a>
## Experimento nativo

Compile o executável opcional de análise offline e forneça artefatos locais revisados:

```sh
cargo build -p vults-spike --features diarization --bin vults-diarization-spike
target/debug/vults-diarization-spike \
  --runtime "$DIARIZATION_RUNTIME" --runtime-sha256 "$DIARIZATION_RUNTIME_SHA256" \
  --segmentation "$DIARIZATION_SEGMENTATION" --embedding "$DIARIZATION_EMBEDDING" \
  --wav "$DIARIZATION_FIXTURE" --reference reference.json
```

Com `CARGO_TARGET_DIR` personalizado, use o executável de seu `debug/`. O hash do WAV
real precisa corresponder à anotação antes da inicialização nativa; a duração normalizada também
precisa corresponder antes da publicação do relatório. O campo `quality` resultante usa a mesma
medição. O WAV é lido para hash e analisado pelo mesmo descritor aberto, depois recebe novo
hash; conteúdo alterado descarta o resultado. Isso não cria uma cópia imutável do áudio. Sem
a opção, a saída normal do experimento mantém `quality: null`. Essa integração não
deduz a identidade da fonte nem sincronização entre dispositivos.

Os exemplos EN/PT incluídos têm linhas do tempo, rótulos de idioma e hash fictícios; nenhum idioma
falado, gravação ou anotação humana foi medido. O caso marcado como inglês confere rótulos
permutados; o marcado como português tem contagens iguais de falantes, mas 40% de confusão.
Seus resultados são regressões da medição, não evidências de precisão dos modelos em inglês
ou português.

<a id="human-calibration-on-2026-10-19"></a>
## Calibração humana em 2026-10-19

Dois recortes públicos de 120 segundos, escolhidos previamente, foram medidos com o limite fixo
de agrupamento 0.7, tolerância zero nas bordas e sobreposição incluída. O manifesto, os hashes dos
recortes/referências e o objetivo provisório de DER 0.20 foram congelados antes da inferência.
São resultados de calibração; nenhum limite foi ajustado e não há aceitação independente ou
qualificação de produção.

| Fonte | Vozes anotadas / encontradas | DER | Total nativo | Pico de RSS |
| --- | ---: | ---: | ---: | ---: |
| AMI IS1000a, primeiros 120 s | 3 / 3 | 82.12% | 3.859 ms | 130.344 KiB |
| C-ORAL-BRASIL bfamdl02, primeiros 120 s | 2 / 1 | 39.79% | 8.950 ms | 155.668 KiB |

A entrada em inglês vem do [corpus AMI](https://groups.inf.ed.ac.uk/ami/corpus/) (CC BY 4.0).
As referências derivadas de palavras vêm da [configuração BUT Speech@FIT](https://github.com/BUTSpeechFIT/AMI-diarization-setup/tree/2509d8933721023fab4def2618aabd5c28eb82e9),
com anotações manuais AMI 1.6.2. Atribuição: J. Carletta et al., *The AMI meeting corpus:
A pre-announcement* (2006); F. Landini et al., *VBx*. A gravação pertence à divisão publicada
de treino; a independência em relação ao treino do modelo não foi estabelecida.

A entrada em português brasileiro é a [amostra pública C-ORAL-BRASIL](https://www.c-oral-brasil.org/english-site/c-oral-brasil-i_example.php),
com alinhamento manual WinPitch `UNIT`. A [licença do corpus](https://www.c-oral-brasil.org/c-oral-brasil-i.php)
é CC BY-NC-SA 4.0; a amostra foi usada localmente em pesquisa não comercial. Atribuição:
Tommaso Raso e Heliana Mello (orgs.), *C-ORAL-BRASIL I*, Editora UFMG (2012).
Áudio original, transcrições e turnos das anotações não são redistribuídos aqui.

A referência em português usa intervalos de enunciados, incluindo suas pausas; a configuração em
inglês usa palavras. Portanto, os percentuais não são diretamente comparáveis entre corpora.
O recorte em inglês tem somente 28.610 ms de falantes de referência nos primeiros 120 segundos,
o que dá grande peso aos falsos alarmes. A referência em português tem 133.901 ms de falantes,
pois a sobreposição conta cada voz; suas duas vozes se fundem em uma no modelo-base. Nenhum
resultado atende ao objetivo provisório de diagnóstico. A identificação automática de falantes
continua indisponível.

A [evidência sem conteúdo](../../../crates/spike/fixtures/meeting-human-calibration-2026-10-19.json)
preserva o manifesto congelado, atribuição das fontes/licenças, hashes dos modelos, tempos e os
componentes completos dos erros. A medição independente por amostragem a cada milissegundo e
pareamento exaustivo de rótulos concorda com os relatórios Rust nativo e separado. As sessões de
CPU usaram uma thread intra/interop em Ryzen 7 5700X3D; o RSS vem do `wait4` do Linux para o
processo filho inteiro. Uma execução por recorte não representa uma distribuição de latência.
`totalMs` inclui carregamento, inferência e hash final; o tempo total do processo também inclui
o hash inicial e a admissão dos modelos.

Para reproduzir, baixe as fontes/revisões públicas exatas registradas na evidência. Extraia
somente `bfamdl02.wav` e `bfamdl02.xml` da amostra indicada. Preserve os primeiros `rate * 120`
quadros PCM sem alterar taxa, canais ou amplitude, gravando WAV PCM16 com cabeçalho canônico
de 44 bytes. AMI permanece mono a 16 kHz; C-ORAL permanece estéreo a 22.05 kHz e é normalizado
pelo worker nativo. Para AMI, use somente linhas `SPEAKER IS1000a 1` de
`only_words/rttms/train/IS1000a.rttm`; calcule o fim pela soma de início e duração antes do
arredondamento. Para C-ORAL, use `speaker`, `startTime` e `endTime` de `UNIT`, sem preservar texto
ou cabeçalhos pessoais. Arredonde as bordas para o milissegundo mais próximo, com empates para
cima; recorte em `[0,120000)`, descarte interseções vazias e atribua inteiros anônimos ordenando
os códigos de falantes da fonte completa. Defina `annotation: human`, `split: calibration` e o
relógio exato do manifesto congelado; depois use o comando nativo acima. Confira os hashes das
referências antes de comparar resultados. Os downloads das fontes, as anotações e o pacote
temporário de modelos/runtime foram removidos após a validação; áudio, perfil e cache do usuário
não foram usados.

<a id="coactivity-correction-on-2026-10-19"></a>
## Correção de atividade simultânea em 2026-10-19

O candidato de desenvolvimento impede que duas observações locais com embeddings, ativas juntas
em um quadro de segmentação pertencente à janela, recebam o mesmo rótulo global. Um par proibido
também permanece separado após fusões intermediárias entre grupos. Observações desconhecidas não
criam restrições. Os modelos, a política de embeddings e o limiar de distância cosseno de 0,7
permanecem iguais; a saída do experimento agora declara `coactivityConstraints: true`.

Os hashes originais de WAV/referência foram reproduzidos exatamente, e ambos os trechos foram
medidos novamente uma vez, sem margem de tolerância e contando sobreposição. Continuam sendo
trechos de calibração examinados durante o desenvolvimento, não um conjunto separado de avaliação.

| Fonte | Vozes encontradas, antes / depois | DER, antes / depois | Total nativo | Pico de RSS |
| --- | ---: | ---: | ---: | ---: |
| AMI IS1000a, primeiros 120 s | 3 / 3 | 82,12% / 82,12% | 3.847 ms | 130.324 KiB |
| C-ORAL-BRASIL bfamdl02, primeiros 120 s | 1 / 2 | 39,79% / 17,29% | 9.029 ms | 148.248 KiB |

O hash da predição em inglês é idêntico. Em português, os milissegundos-falante de confusão caem de
26.015 para 1.428 e os de omissão de 26.617 para 17.880, enquanto os falsos alarmes sobem de 641
para 3.847. O candidato em português atende à meta diagnóstica provisória original de 0,20; o
candidato em inglês não. Isso não qualifica para produção: a atividade simultânea prevista pode
estar errada, e uma correção desenvolvida após examinar estes trechos precisa de avaliação separada.

A [evidência da comparação](../../../crates/spike/fixtures/diarization-coactivity-2026-10-19.json)
preserva o hash da evidência original e os relatórios agregados do candidato. As pontuações do
nativo, da ferramenta independente e do cálculo separado por milissegundo com mapeamento exaustivo
concordam. Uma regressão determinística verifica fusões proibidas diretas e intermediárias. Os
testes nativos de conclusão, cancelamento tardio, interrupção ativa e prioridade interativa passam.
Áudio público temporário, anotações e arquivos de modelos/runtime foram removidos depois;
nenhuma gravação ou perfil do usuário foi usado.

A próxima etapa de qualidade precisa de material humano de avaliação com gravações/falantes
separados e limites de aceitação congelados antes de novos ajustes. Alinhamento físico de
microfone/sistema, eco, turnos baixos, ruído e recursos em sessões longas seguem abertos.
Os rótulos automáticos de falantes continuam indisponíveis.

<a id="pause-and-diagnostic-refinements-on-2026-10-19"></a>
## Refinamentos de pausas e diagnóstico em 2026-10-19

O worker de desenvolvimento agora une somente turnos do mesmo rótulo que se tocam ou sobrepõem.
A antiga tolerância de 17 ms podia preencher um quadro inteiro ausente da segmentação e fazer
um trecho de texto parecer continuamente coberto por uma voz. A regressão verifica pausas de
1, 16, 17 e 1.000 ms, observações conhecidas e desconhecidas, turnos adjacentes e a atribuição
posterior ao texto. Turnos curtos ou baixos não são filtrados. O experimento declara
`fillsSpeechGaps: false`; modelos, embeddings, restrições de atividade simultânea e limiar de
distância de 0,7 permanecem iguais.

Duas outras fontes em inglês foram escolhidas antes dos respectivos experimentos na
[lista de reuniões de teste do BUT AMI fixada](https://github.com/BUTSpeechFIT/AMI-diarization-setup/blob/2509d8933721023fab4def2618aabd5c28eb82e9/lists/test.meetings.txt).
IS1009a foi avaliada para a correção de pausas e depois explicitamente movida para calibração
quando seus erros motivaram outro experimento de agrupamento. ES2004a foi escolhida antes desse
segundo candidato. Ambas usam os primeiros 120 segundos, o mesmo espelho público/licença/crédito
do AMI e referências de palavras em `only_words/rttms/test/`. São gravações diferentes, mas não
há comprovação de separação de identidades de falantes nem do treinamento dos modelos.
Nenhuma fonte independente em português foi adicionada.

| Worker mantido, primeiros 120 s | Vozes anotadas / encontradas | DER |
| --- | ---: | ---: |
| IS1000a, calibração original em inglês | 3 / 3 | 82,12% |
| bfamdl02, calibração original em português | 2 / 2 | 17,29% |
| IS1009a, calibração ampliada em inglês | 4 / 2 | 61,09% |
| ES2004a, avaliação adicional em inglês | 3 / 1 | 31,33% |

As predições da calibração original não mudam. Preservar pausas acrescenta 17 milissegundos-falante
omitidos da referência em IS1009a (DER de 61,06% para 61,09%), portanto o critério de qualidade
congelado de ausência de regressão **falha**. A mudança fica no worker de desenvolvimento por
correção dos tempos e da cobertura contínua; não representa melhora de qualidade nem aceitação
para lançamento. Uma regra mais forte que proibia fundir todos os slots locais, mesmo sem
sobreposição, foi testada e removida: piorou IS1000a de 82,12% para 90,64%, apesar de melhorar
outros trechos individuais. Não houve busca por limiares.

A nova divisão dos falsos alarmes identifica problemas diferentes. IS1000a tem 15.329
milissegundos-falante sem voz de referência ativa e zero de sobreposição excessiva. Em português,
são 649 e 3.198, respectivamente. São resultados relativos às anotações, não causas acústicas
comprovadas. É preciso revisar escuta/anotações e avaliar fala/sobreposição de forma independente
antes de mudar as regras de detecção.

Seis variantes controladas de calibração exercitam volume menor, ruído uniforme e eco atrasado:

| Transformação | DER em inglês | DER em português |
| --- | ---: | ---: |
| Ganho 0,1 | 82,00% | 17,23% |
| Ruído uniforme, SNR de 15 dB na fonte inteira | 90,10% | 17,67% |
| Eco da mesma fonte, atraso de 200 ms, ganho 0,35 | 84,83% | 31,62% |

Elas mantêm a taxa de amostragem original, calculam a média dos canais estéreo para mono,
arredondam/limitam PCM16 e propagam intervalos anônimos marcados como `annotation: controlled`,
`split: calibration`. As referências de eco acrescentam intervalos atrasados do mesmo falante,
limitados à duração da fonte; atividade duplicada de um falante continua contando uma vez.
O ruído usa o gerador e a semente determinísticos da evidência. Não são novas anotações humanas,
comparação de idiomas nem validação física de microfone/sistema. O controle de ruído em português
também acrescenta 17 milissegundos-falante omitidos ao remover o preenchimento artificial de pausas.

A [evidência dos refinamentos](../../../crates/spike/fixtures/diarization-refinements-2026-10-19.json)
registra planos de experimento congelados, hashes exatos de fontes/referências, o candidato
rejeitado, relatórios mantidos e transformações controladas. Os relatórios nativo e independente
concordam com o cálculo separado por milissegundo/mapeamento exaustivo, incluindo os dois novos
contadores de falsos alarmes. O preenchimento antigo de pausas foi reconstruído e conferido com
três execuções nativas separadas do resultado anterior. Passam 90 testes de voz, 22 do experimento
e 3 verificações nativas explícitas. Áudio, texto e turnos das anotações originais não são
redistribuídos; downloads/modelos/runtime temporários são removidos depois e o app continua fechado.
Os rótulos automáticos de falantes seguem indisponíveis.

<a id="decoder-ties-and-reference-audit-on-2026-10-19"></a>
## Empates do decodificador e auditoria das referências em 2026-10-19

O decodificador de desenvolvimento agora mantém a primeira classe powerset com máximo exato,
seguindo o [decodificador original](https://github.com/pyannote/pyannote-audio/blob/3.0.0/pyannote/audio/utils/powerset.py)
e a [semântica de primeiro máximo do argmax](https://docs.pytorch.org/docs/2.14/generated/torch.argmax.html).
A seleção anterior do último máximo escolhia a classe final de sobreposição quando as sete
pontuações empatavam. A regressão reproduz essa falha e cobre todos os pares de máximos,
máximos únicos, zeros com sinal e quase empates separados por um passo de ponto flutuante.
Não foi adicionada tolerância de confiança nem limiar de fala/sobreposição. A saída nativa
declara `argmaxTieBreak: first`.

Os quatro hashes de predição principais e relatórios completos de qualidade correspondem à
evidência anterior dos refinamentos. É correção do contrato do decodificador, não ganho de
precisão medido nestes trechos. Hashes dos modelos, embeddings, restrições de atividade simultânea,
preservação de pausas e limiar de agrupamento de 0,7 permanecem iguais.

As mesmas predições também foram medidas contra as referências fixadas do BUT AMI em
`word_and_vocalsounds/rttms/`. Os
[autores das referências](https://github.com/BUTSpeechFIT/AMI-diarization-setup/blob/2509d8933721023fab4def2618aabd5c28eb82e9/README.md)
publicam essa alternativa, mas preferem medir somente palavras porque as anotações de sons
vocais são inconsistentes. As referências alternativas são material diagnóstico de calibração;
o perfil principal continua só com palavras, sem margem de tolerância e contando sobreposição.

| Primeiros 120 s | Milissegundos-falante de falso alarme, só palavras | Com sons vocais | Cobertos pela referência ampliada |
| --- | ---: | ---: | ---: |
| IS1000a | 15.329 | 15.329 | 0 |
| IS1009a | 10.445 | 10.445 | 0 |
| ES2004a | 3.244 | 3.121 | 123 |

As anotações ampliadas não explicam a maior parte dos falsos alarmes nesses trechos. Também
mudam o tempo de falantes de referência e o pareamento ótimo; por isso, uma DER diferente não
representa melhora do modelo: IS1000a passa a 83,22%, IS1009a permanece em 61,09% e ES2004a passa
a 35,41%. Essas comparações não comprovam causas acústicas sem revisão de escuta/anotações.
O português mantém seu relatório original; nenhuma anotação ou gravação adicional foi introduzida.

A [evidência da auditoria](../../../crates/spike/fixtures/diarization-reference-audit-2026-10-19.json)
preserva o plano anterior à inferência, o hash da evidência original, os sete hashes das
referências e hashes dos downloads, relatórios agregados e créditos/licenças das fontes.
Reproduza com o mesmo preparo de recorte acima, usando as referências fixadas em
`word_and_vocalsounds/rttms/train/IS1000a.rttm` e
`word_and_vocalsounds/rttms/test/{IS1009a,ES2004a}.rttm`. Mapeie os rótulos anônimos a partir da
anotação completa e mantenha o relógio exato da fonte principal. Meça a predição nativa sem
alterações contra cada referência. Não substitua o resultado principal pelo diagnóstico.

Sete relatórios independentes concordam com o cálculo separado por milissegundo/mapeamento
exaustivo de rótulos; os quatro relatórios nativos principais não mudam. Passam 91 testes de
voz, 22 do experimento e 3 verificações nativas explícitas. Áudio/anotações/modelos/runtime
temporários são removidos depois; o app continua fechado e os rótulos automáticos de falantes
seguem indisponíveis. O próximo experimento de detecção pode avaliar o detector de fala Silero
existente de forma independente, com perfil congelado e atenção a falas curtas/baixas omitidas;
esta auditoria não ativa esse filtro.

<a id="existing-speech-detector-evaluation-on-2026-10-19"></a>
## Avaliação do detector de fala existente em 2026-10-19

O experimento offline pode comparar sua diarização sem alterações com um candidato de máscara
de fala usando o detector Silero existente. Acrescente `--vad` após uma referência obrigatória:

```sh
target/debug/vults-diarization-spike \
  --runtime "$DIARIZATION_RUNTIME" --runtime-sha256 "$DIARIZATION_RUNTIME_SHA256" \
  --segmentation "$DIARIZATION_SEGMENTATION" --embedding "$DIARIZATION_EMBEDDING" \
  --wav "$DIARIZATION_FIXTURE" --reference reference.json --vad "$DIARIZATION_VAD"
```

Forneça um caminho absoluto para uma cópia confiável e imutável do modelo já catalogado
`ggml-silero-v6.2.0.bin`: 885.098 bytes, SHA-256
`2aa269b785eeb53a82983a20501ddf7c1d9c48e33ab63a41391ac6c9f7fb6987`.
A [conversão ggml](https://huggingface.co/ggml-org/whisper-vad) e o
[detector original](https://github.com/snakers4/silero-vad) declaram licença MIT.
O experimento verifica tamanho/hash antes da inicialização nativa; no Linux, carrega pelo
descritor mantido e recalcula os hashes do modelo e da fonte antes da publicação. Isso detecta
edições persistentes; não cria uma cópia imutável nem exclui edições transitórias. O comando não
baixa modelos.

Sem a opção, `vadDiagnostic` é nulo e somente a análise original roda. Com ela, `quality`/`turns`
principais permanecem como resultado original, e `vadDiagnostic` contém qualidade do candidato,
regiões de fala e turnos intersectados. `totalMs` continua medindo o resultado original; o tempo
do diagnóstico fica separado. O RSS do processo inclui ambos os motores e não isola o custo do VAD.
O diagnóstico usa o mesmo descritor/normalização da fonte e limite de três minutos. Carregamento,
inferência e liberação nativos compartilham um slot do escalonador, com admissão atrás do trabalho
interativo. O Silero não pode abortar dentro dessa chamada; o cancelamento descarta o resultado
ao terminar. É um experimento offline, não implementação de preempção em produção nem filtro do app.

O perfil foi congelado antes da medição: suavização/limiares existentes, confirmação de 250 ms
de fala e 400 ms de silêncio, uma thread de CPU e a margem existente de 200 ms em cada região.
A interseção dessas regiões apenas remove atividade original; preserva tempos, IDs conhecidos/
desconhecidos e sobreposição. Nunca concatena áudio nem preenche pausas originais. Uma regressão
executável confere essas propriedades, turnos de um milissegundo, divergência de duração e limite
de 4.096 turnos.

| Fonte humana, primeiros 120 s | DER original | DER com máscara | Milissegundos-falante omitidos a mais |
| --- | ---: | ---: | ---: |
| IS1000a, inglês | 82,12% | 50,73% | 5.235 |
| bfamdl02, português | 17,29% | 17,57% | 566 |
| IS1009a, inglês | 61,09% | 57,48% | 1.026 |
| ES2004a, inglês | 31,33% | 44,16% | 10.685 |
| TS3003a, nova gravação em inglês | 17,49% | 31,26% | 12.875 |

TS3003a foi escolhida na lista fixada de testes do BUT antes da saída do candidato, com o mesmo
espelho público AMI, recorte dos primeiros 120 segundos e preparo de anotações só de palavras.
É outra gravação, sem comprovação de separação de falantes/treinamento dos modelos. Os quatro
trechos humanos conhecidos e seis transformações controladas anteriores de volume/ruído/eco
continuam como calibração nesta avaliação; a declaração anterior de avaliação de ES2004a permanece
como metadado histórico. Nenhuma fonte humana independente em português foi acrescentada.

A máscara reduz os falsos alarmes de IS1000a de 15.329 para 1.368 milissegundos-falante, mas perde
um turno de referência antes coberto. ES2004a e TS3003a perdem quatro cada. Os controles de volume
baixo acrescentam 12.370 milissegundos-falante omitidos em inglês e 3.253 em português; ruído
acrescenta 9.952/911 e eco, 8.385/885. Um fragmento controlado de fala de domínio público de
200 ms perde todos os seus 170 ms de cobertura original. Esse fragmento não é uma palavra completa
anotada por humanos. Sua DER cai de 133% para 100% com o desaparecimento de falsos alarmes/confusão,
mostrando por que uma DER agregada menor não basta para qualificar um filtro.

**Rejeitar a máscara geral de fala.** Os 12 casos falham no critério congelado de não acrescentar
fala omitida; vários também falham nos critérios de DER e turnos que perdem toda a cobertura.
O diagnóstico opcional continua disponível para reprodução. O app e a diarização padrão não mudam.
Este experimento não ajusta os limiares do Chat/ditado nem justifica remover seu comportamento VAD.

O tempo omitido é relativo às anotações fornecidas. Os intervalos de enunciados do C-ORAL incluem
pausas; por isso, o tempo omitido a mais nessa fonte, isoladamente, não comprova perda de conteúdo
fonético. A validação acústica continua aberta; as regressões de cobertura das palavras em inglês
e a perda do fragmento curto controlado já impedem aceitar esse perfil.

A [evidência do VAD](../../../crates/spike/fixtures/diarization-vad-evaluation-2026-10-19.json)
preserva plano congelado, hashes de modelos/fontes/referências, receitas controladas, todos os
resultados e recursos agregados. Os dez relatórios originais conhecidos reproduzem hashes/
componentes anteriores; 24 relatórios concordam com a medição independente e um cálculo separado
por milissegundo/mapeamento exaustivo. Cada intervalo com máscara é verificado como subintervalo
de um turno original com a mesma identidade. Passam 91 testes de voz com recursos opcionais,
23 do experimento, 4 verificações nativas explícitas e 57 testes de voz padrão com as verificações
reais de fixtures Silero habilitadas. Três chamadas inválidas de VAD são recusadas antes da
admissão do runtime nativo; uma execução nativa sem a opção confirma `vadDiagnostic: null`.
Áudio original, anotações, controles e modelos/runtime temporários são removidos depois.
O app permanece fechado e os rótulos automáticos de falantes seguem indisponíveis. O próximo
trabalho de detecção específico para reuniões precisa de aceitação separada para falas curtas/
baixas e cancelamento em produção; o perfil compartilhado do ditado não deve ser reajustado
a partir deste candidato reprovado.

<a id="meeting-frame-speech-profile-on-2026-10-19"></a>
## Perfil de fala por quadros para reuniões em 2026-10-19

Um segundo candidato offline, definido separadamente antes da medição, remove a suavização e a
confirmação de 250 ms de fala do detector de ditado. `--vad-frames` substitui `--vad` no comando
acima; ambos exigem uma referência e o mesmo modelo imutável verificado pelo catálogo. Seu perfil
explícito `meeting-frames` mantém cada quadro bruto de 512 amostras com probabilidade de pelo
menos 0,5, une quadros selecionados adjacentes e aplica a margem/união original de 200 ms. A
16 kHz, cada quadro dura 32 ms; o quadro parcial final respeita o limite da fonte. Entradas menores
que um quadro continuam sem atividade. Não há busca de limiar, duração ou margem, nem novo modelo
ou dependência.

A API opcional exige um `SpeechProfile`: `Dictation` reproduz o diagnóstico anterior;
`MeetingFrames` seleciona este candidato. Chat/ditado mantêm o detector e os tempos originais.
A CLI registra o perfil selecionado em `vadDiagnostic.profile`. A análise de base, a verificação
da fonte/modelo, a preservação do relógio/identidade e a limitação de cancelamento nativo descrita
acima permanecem iguais. Nenhum perfil é um filtro do app ou um recurso de rótulos automáticos
de falantes.

Os 12 casos anteriores são todos calibração conhecida para este novo candidato, incluindo
TS3003a e os metadados de avaliação anteriores. Os primeiros 120 segundos de EN2002a foram
selecionados da mesma lista de teste BUT fixada antes dos resultados do candidato como uma
gravação adicional em inglês. Sua referência apenas de palavras usa a receita original de
recorte, arredondamento e rótulos anônimos. Isso estabelece uma gravação nova para este experimento,
sem comprovar independência de falantes/treino dos modelos; ainda falta avaliação humana
independente em português. A aceitação definida antes da medição volta a exigir nenhum aumento
de falsos alarmes, DER ou milissegundos-falante perdidos, e nenhuma fala da referência antes
coberta passando a ficar sem cobertura.

| Caso | DER de base | DER com máscara por quadros | Milissegundos-falante adicionais perdidos | Falas que ficaram sem cobertura |
| --- | ---: | ---: | ---: | ---: |
| IS1000a, inglês | 82,12% | 52,74% | 4.434 | 1 |
| IS1009a, inglês | 61,09% | 59,18% | 306 | 0 |
| ES2004a, inglês | 31,33% | 40,72% | 8.397 | 3 |
| TS3003a, inglês | 17,49% | 28,45% | 10.051 | 0 |
| EN2002a, nova gravação em inglês | 35,90% | 37,26% | 2.987 | 0 |
| bfamdl02, português | 17,29% | 17,53% | 474 | 0 |
| Controle de volume baixo em inglês | 82,00% | 62,37% | 10.536 | 1 |
| Controle de ruído em inglês | 90,10% | 64,78% | 8.221 | 1 |
| Controle de eco em inglês | 84,83% | 70,56% | 7.163 | 4 |
| Controle de volume baixo em português | 17,23% | 20,00% | 3.885 | 0 |
| Controle de ruído em português | 17,67% | 17,91% | 774 | 1 |
| Controle de eco em português | 31,62% | 31,62% | 205 | 0 |
| Fragmento controlado de 200 ms | 133,00% | 133,00% | 0 | 0 |

O controle curto preserva todos os 170 ms de cobertura da análise de base, em comparação com zero
no perfil anterior. Seu DER inalterado de 133% ainda expõe os falsos alarmes e a identidade sem
mapeamento da base; preservar um fragmento controlado não qualifica o reconhecimento de palavras
curtas. O novo perfil reduz as perdas adicionais nos quatro trechos humanos conhecidos em inglês
em comparação com o diagnóstico anterior, mas não as elimina. A melhora não é uniforme: o controle
de volume baixo em português acrescenta 3.885 milissegundos-falante perdidos, em comparação com
3.253 no perfil anterior.

**Rejeitar novamente a máscara rígida geral.** Os seis trechos humanos e seis controles de
estresse falham no critério de não acrescentar perdas de fala; somente o fragmento curto
controlado passa. EN2002a acrescenta 2.987 milissegundos-falante perdidos apesar de reduzir falsos
alarmes. A ressalva sobre enunciados/pausas do C-ORAL continua relevante; as regressões na cobertura
de palavras em inglês impedem a aceitação independentemente disso. Nem uma DER média menor nem
a melhora no controle curto anulam essas falhas. Não há busca posterior de parâmetros.

A [evidência do perfil por quadros](../../../crates/spike/fixtures/diarization-meeting-frames-2026-10-19.json)
preserva o plano definido antes da medição e a codificação de seu hash, hashes de fonte/referência/
modelo/código, receitas controladas anteriores exatas, relatórios agregados, medições de recursos
e licenças. Os 12 relatórios conhecidos de base permanecem iguais; 26 relatórios nativos concordam
com a pontuação independente pela CLI e com um oráculo independente de raster por milissegundo/
mapeamento exaustivo. Toda saída da máscara continua sendo um subconjunto da base com a mesma
identidade, e a atividade removida corresponde à redução de falsos alarmes somada às perdas
adicionais. O perfil antigo `--vad` reproduz seus relatórios anteriores de inglês/controle curto,
e uma execução comum mantém o diagnóstico nulo. Quatro entradas inválidas do candidato falham
antes do carregamento do runtime nativo.

A validação passa 92 testes de voz com recursos opcionais, 23 testes do spike, 4 verificações
nativas explícitas e 57 testes de voz padrão com amostras reais do Silero, além de Clippy com todos
os recursos, formatação e verificações do repositório. A regressão pura distingue uma rajada curta
da confirmação do ditado e cobre limiares exatos, silêncio, lacunas e bordas parciais da fonte;
as verificações nativas de entrada/cancelamento exercitam ambos os perfis. Áudio, anotações,
controles e modelos/runtime temporários são removidos depois; o app permanece fechado e os caches
existentes do usuário/compartilhados são preservados. A diarização padrão e o comportamento de
chat/ditado permanecem iguais. O próximo trabalho deve tratar os erros de detecção/sobreposição
com evidência independente de preservação da fala, em vez de continuar buscando parâmetros de
máscara rígida; relógios de fontes físicas, avaliação independente em português, cancelamento de
produção e aceitação de sessões longas continuam pendentes.

<a id="millisecond-timeline-edges"></a>
## Bordas do relógio em milissegundos

O recorte de quadros em amostras pode produzir um intervalo cujo início e fim se convertem para
o mesmo milissegundo. A análise de falantes agora descarta esse intervalo de duração zero antes
de unir os turnos; ela não estende o intervalo para fazê-lo caber. Antes, uma borda isolada que
colapsasse podia fazer a associação com o texto ou a pontuação offline rejeitar o resultado
inteiro. Turnos de duração positiva, incluindo um turno de um milissegundo, preservam seus
rótulos e limites; lacunas continuam sendo lacunas.

O leitor de WAV compartilhado exige pelo menos um milissegundo no relógio da fonte normalizada,
antes de alocar o reamostrador ou informar progresso. Essa verificação única cobre a transcrição,
a análise nativa de falantes e os dois diagnósticos opcionais de fala. Antes, a transcrição podia
publicar um resultado vazio com duração zero, enquanto somente a análise de falantes aplicava o
mínimo. Fontes válidas de um milissegundo continuam aceitas. Trata-se da representação no relógio,
não do reconhecimento de fala.

Regressões que falham antes da correção e passam depois cobrem turnos recortados conhecidos/
desconhecidos com associação ao texto, entrada da transcrição antes de progresso/decodificação,
e limites de fonte em sete taxas de amostragem (incluindo 8.001/22.050/44.100 Hz), mono/estéreo e
float32/PCM16/24/32. As amostras de cobertura de blocos e de valores não finitos usam a menor
duração válida para preservar o propósito dessas verificações. Passam 98 testes de voz com
recursos opcionais (6 verificações dependentes de modelos ignoradas), 59 testes de voz padrão
(2 ignorados), 23 testes do spike, Clippy com todos os recursos, formatação e verificações do
repositório. Este refinamento usa WAVs sintéticos sem inferência nativa, downloads de modelos/
áudio ou ativação no app; a qualidade de fala humana e a aceitação em reuniões reais continuam
pendentes.

<a id="native-operation-preparation"></a>
## Preparação de operações nativas

Antes de cada chamada nativa de segmentação/embedding, o controlador verifica novamente o
cancelamento e pedidos pendentes de voz interativa após construir as opções da chamada. Um
cancelamento observado nesse ponto retorna sem resultado e sem iniciar a inferência. Um pedido
interativo já na fila libera as opções preparadas e o espaço de execução em segundo plano; a
operação prepara novas opções depois que o trabalho interativo termina. Antes, esses pedidos
ainda podiam iniciar trabalho nativo desnecessário antes da reação do observador de interrupção.
Pedidos que chegam durante a inferência mantêm o fluxo existente de interrupção/descarte/nova
tentativa. Os limites de cancelamento da construção de sessões e da chamada completa do Silero
acima continuam valendo.

Duas regressões determinísticas falham antes da correção e passam depois, cobrindo cancelamento
e chegada do primeiro plano durante a criação das opções. Passam 96 testes de voz com recursos
opcionais (5 ignorados), 23 testes do spike e 3 verificações nativas explícitas de conclusão do
fluxo/cancelamento tardio, interrupção ativa e nova tentativa com prioridade para o primeiro plano,
além de Clippy com todos os recursos, formatação e verificações do repositório. As verificações
nativas usam o pacote existente de modelos/runtime com hashes fixados, a amostra de fala do
repositório e entrada sintética. Modelos/runtime e logs temporários são removidos depois; não há
uso de gravação/perfil pessoal nem ativação no app. Isso valida o agendamento das operações, não
a qualidade de fala humana; os rótulos automáticos de falantes e os critérios de aceitação em
reuniões reais continuam pendentes.

<a id="shared-speech-detector-ownership"></a>
## Controle compartilhado do detector de fala

A construção do Silero, a detecção interativa e a liberação do contexto agora usam o agendador
compartilhado de trabalho nativo. Isso abrange o observador de fim de fala do chat, o recorte final
da fala e a verificação de presença de fala para nova tentativa no ditado. Antes, a construção
era serializada, enquanto detecção/liberação podiam ignorar outros trabalhos nativos de voz.
A detecção interativa agora entra na mesma fila de prioridade do Whisper. Não há ajuste de
limiares do detector, histerese, margens ou tempos da fonte.

O diagnóstico offline de fala mantém a entrada em segundo plano durante a construção do contexto,
a inferência e a liberação. Ele compartilha o decodificador por funções que executam sob um espaço
já reservado, sem tentar adquirir recursivamente um espaço interativo. Sua limitação de
cancelamento/interrupção da chamada completa continua igual; isso não ativa as máscaras de fala
reprovadas no app.

Uma nova regressão nativa explícita mantém um espaço ocioso do agendador ocupado e comprova que
a detecção e a liberação entram na fila antes de concluir. Ela falha antes da correção, mostrando
que ambas ignoravam o controle, e passa depois; não executa dois motores nativos ao mesmo tempo
para provocar uma falha. A verificação nativa existente do diagnóstico de fala também passa para
os dois perfis sem bloqueio. Passam 96 testes de voz com recursos opcionais (6 ignorados), 57
testes de voz padrão (2 ignorados), 23 testes do spike, os dois testes nativos explícitos e Clippy
com todos os recursos, formatação e verificações do repositório. As suítes normais de voz executam
com o modelo Silero real fixado por hash, mantendo as verificações de amostras de fala/pausa/
silêncio/ruído. Somente o modelo existente é baixado em um diretório descartável; não há uso de
áudio/perfil pessoal. O app permanece fechado e os modelos/logs temporários são removidos.
Isso estabelece o controle de acesso nativo; a qualidade de fala humana e a aceitação em reuniões
reais continuam pendentes.

<a id="buffered-wav-input"></a>
## Entrada WAV com buffer

O leitor de WAV compartilhado usa o buffer de entrada limitado da biblioteca padrão, para que
a interpretação amostra por amostra não exija uma leitura do arquivo para cada amostra. Isso
abrange a transcrição, a análise nativa de falantes e os diagnósticos offline de fala. Não há
nova dependência ou configuração; os limites de blocos PCM, os relógios das fontes, a validação
e o comportamento de cancelamento permanecem iguais.

Uma regressão com leitor que conta chamadas registra 96.017 leituras antes da correção para um
segundo de entrada float32 estéreo a 48 kHz. O caminho com buffer passa no limite de 407 leituras,
produzindo exatamente o PCM mono/reamostrado de referência e o relógio completo de 16.000 amostras.
Essa é uma verificação de chamadas de entrada/saída, não uma qualificação de latência total ou
de desempenho em dispositivo real. As verificações existentes de arquivos malformados,
truncados, cancelamento e limites entre blocos também passam.

Passam 99 testes de voz com recursos opcionais (6 ignorados), 60 testes de voz padrão
(2 ignorados), 23 testes do spike, Clippy com todos os recursos, formatação e verificações do
repositório. Somente entradas sintéticas; não são necessários inferência nativa, downloads de
áudio/modelos ou inicialização do app para essa verificação. Os logs temporários são removidos
e os caches pessoais/compartilhados são preservados. A qualidade de fala humana, os relógios
de fontes físicas e a aceitação de sessões longas continuam pendentes.
