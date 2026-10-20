# Modelo semântico de fim de turno
<!-- source: 8432a58f1af2 -->

A V6.2.2 fornece um **backend offline para desenvolvedores**, pela feature `semantic-turn` de
`vults-voice`. Builds normais do app não a ativam. Esse backend não tem download, controle de
microfone, opção em Configurações ou requisição de rede. V6.3-V6.7 mantêm a integração da conversa/runtime.
Esta é uma qualificação numérica no runtime CPU Linux x64 conferido, não uma aceitação de qualidade
de fim de turno, fala pt-BR, meta de latência ou plataforma.

<a id="sources-and-licenses"></a>
## Fontes e licenças

A [documentação do código e modelo smart-turn](https://github.com/pipecat-ai/smart-turn/tree/4786657e242dfe77dd138699ac564ee074a2a543)
e o [card fixado dos pesos](https://huggingface.co/pipecat-ai/smart-turn-v3/blob/f766f81d3cfdf7737ac64aad813d91bbfd56bf93/README.md)
declaram **BSD-2-Clause**. A descrição anterior de Apache-2.0 no plano para esses pesos estava incorreta.
O grafo é um export int8 v3.0 sem modificações; exports v3.1/v3.2 posteriores não o substituem automaticamente.
Sua [licença upstream](https://github.com/pipecat-ai/smart-turn/blob/4786657e242dfe77dd138699ac564ee074a2a543/LICENSE)
fica preservada em [SMART-TURN-LICENSE.txt](../../../crates/voice/assets/semantic-turn/SMART-TURN-LICENSE.txt).
Nenhum peso de modelo vem incluído no Vults.

O extrator de features Rust é implementado de forma independente e testado contra o
[WhisperFeatureExtractor do Transformers 4.57.1](https://github.com/huggingface/transformers/blob/v4.57.1/src/transformers/models/whisper/feature_extraction_whisper.py)
e seus [utilitários de áudio](https://github.com/huggingface/transformers/blob/v4.57.1/src/transformers/audio_utils.py).
O código de referência é Apache-2.0; sua licença completa fica preservada em
[TRANSFORMERS-LICENSE.txt](../../../crates/voice/assets/semantic-turn/TRANSFORMERS-LICENSE.txt).
As matrizes geradas não contêm pesos nem áudio novo gravado do usuário.

<a id="pinned-artifacts"></a>
## Artefatos fixados

| Artefato | Bytes | SHA-256 |
|---|---:|---|
| [smart-turn-v3.0.onnx](https://huggingface.co/pipecat-ai/smart-turn-v3/resolve/f766f81d3cfdf7737ac64aad813d91bbfd56bf93/smart-turn-v3.0.onnx) | 8.757.193 | `07a133aba31e2d0b523f17f8c2e4e65efe6d8f685efd12ca4fe21ebf4e798991` |
| Biblioteca CPU ONNX Runtime 1.31.0 Linux x64 | 29.345.384 | `469849977cb60744ea160927e1a3a0643fd4c55c0aff743251c6d293402e334e` |
| Licença smart-turn preservada | 1.301 | `0d66364067f678c08586ebb60a16a2aed4fa081ec11057df35585759ce0e774f` |
| Licença Transformers preservada | 11.418 | `77fd4710def9ec3c0f6225800e0235f15a425abd4a8b03559127fcd782612049` |

O arquivo exato do runtime, licença, avisos de terceiros e documento de privacidade estão em
[Artefatos dos modelos de reuniões](meeting-models.md#download-pins). O responsável compartilhado
`voice::onnx::Runtime` exige uma biblioteca confiável e imutável em caminho absoluto, fingerprint
independente e `ORT_DISABLE_TELEMETRY=1` no início do processo; também desativa a API de telemetria
nativa. Um processo não pode trocar seu runtime admitido. O caminho Runtime existente da diarização
continua sendo uma reexportação.

`Detector::load` exige também esse fingerprint qualificado do runtime e os bytes exatos do modelo.
O leitor compartilhado recusa symlinks, arquivos não regulares, divergências de tamanho/hash e leituras
canceladas; o carregamento usa os bytes verificados em memória em vez de reabrir o caminho. Sessões
usam execução CPU, uma thread intra/inter-op e otimização de grafo `All`. Um runtime Python 1.24.4
anterior produziu outra probabilidade para o mesmo tensor de entrada curta; essa execução não é a
referência. Alterar runtime, opções de grafo, modelo ou CPU exige nova qualificação em vez de afrouxar testes.

<a id="feature-contract"></a>
## Contrato das features

A entrada é PCM f32 mono a 16 kHz. Mantém as últimas 128.000 amostras; completa entradas menores com
zeros à esquerda antes de normalizar para média zero/variância unitária (epsilon de variância `1e-7`).
Recusa amostras não finitas na janela retida e estouro na normalização. O backend não grava áudio em disco.

As features usam janela Hann periódica de 400 amostras, preenchimento central refletido, FFT de 400
pontos e passo de 160 amostras. A referência arredonda a FFT para complex64 antes da potência em
float64 e redução para 80 mels com normalização Slaney. Aplica piso `1e-10`, log10, descarta o último
quadro STFT, limita ao máximo global retido menos oito e então `(x + 4) / 4`. O modelo recebe
`input_features` f32 de formato `[1, 80, 800]`; `logits` deve ser uma probabilidade finita em `[0, 1]`,
com formato `[1, 1]`. Apesar do nome da saída, nenhuma sigmoid adicional é aplicada.

`voice::turn::end` mantém a exigência de prontidão de VAD/texto e o limiar 0,5. Uma pontuação semântica
sozinha não é detecção de fala nem autoridade de permissão: até silêncio pode receber probabilidade
alta de conclusão. O agendador vincula cada pedido a seu turno/token. Use `semantic_deadline(ticket)`
para calcular o orçamento restante do worker; não reinicie a espera de 1,2 s ao despachar com atraso.
O worker confere cancelamento/prazo no pré-processamento, admissão na fila interativa e inferência
nativa. RunOptions pede encerramento cooperativo; inicialização/kernels nativos já em execução não
prometem aborto instantâneo. Resultados cancelados são descartados; erro/timeout passa pela alternativa
da política que desativa o recurso após uma falha na conversa. O adaptador mantém a responsabilidade
e recusa resultados antigos antes da publicação. Destruir sessões compartilha a trava de trabalho
de fala com as outras operações nativas.

<a id="validation-and-remaining-gates"></a>
## Validação e critérios pendentes

As matrizes completas de referência foram geradas com NumPy 2.2.6 e o caminho NumPy CPU do
Transformers 4.57.1, seguindo a receita fixada de corte/preenchimento upstream e `do_normalize=True`.
Cobrem uma onda periódica curta, ruído determinístico maior que oito segundos e o WAV de fala
existente. Silêncio tem uma referência exata separada de `-1.5`. Os metadados incluem cada SHA-256
das features e probabilidades do Python ONNX Runtime 1.31.0, CPU, uma thread e otimização de grafo `All`.

Testes Rust comparam as 64.000 features de cada matriz com erro absoluto máximo inferior a `1e-4`,
e um teste nativo explícito compara as quatro probabilidades com tolerância `2e-4`. Esse teste nativo
é ignorado no CI comum: exige o grafo/runtime conferidos e fornecidos explicitamente, sem baixá-los.
Testes puros cobrem cancelamento, pré-processamento expirado, saída da fila, encerramento de trabalho
nativo travado, limpeza após panic, prazos antigos e modelo corrompido sem carregar biblioteca nativa.

```sh
cargo test -p vults-voice --features semantic-turn
cargo clippy -p vults-voice --all-targets --features semantic-turn,diarization -- -D warnings
# After explicitly supplying the checked graph and trusted runtime:
ORT_DISABLE_TELEMETRY=1 \
VULTS_SEMANTIC_TEST_RUNTIME=/absolute/path/libonnxruntime.so.1.31.0 \
VULTS_SEMANTIC_TEST_RUNTIME_SHA256=469849977cb60744ea160927e1a3a0643fd4c55c0aff743251c6d293402e334e \
VULTS_SEMANTIC_TEST_MODEL=/absolute/path/smart-turn-v3.0.onnx \
cargo test -p vults-voice --features semantic-turn \
  native_probabilities_match_the_reference_on_checked_artifacts -- --ignored --nocapture
```

Rótulos reais de fim de turno em conversa, qualidade pt-BR, aceitação de dispositivo/desempenho e
integração nativa da interface continuam pendentes. Esta etapa não aprova oferta de download no app
nem escuta automática.
