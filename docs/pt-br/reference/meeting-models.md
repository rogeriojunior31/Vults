# Artefatos dos modelos de reuniões
<!-- source: b0b0bb50f55c -->

O instalador opcional da N3 em **Ditado > Configurações de reuniões > Modelos de falantes para reuniões (experimental)**
prepara os artefatos de análise local de falantes para Linux x86-64 com glibc. Ele não ativa
identificação de falantes, captura ou resumos por modelo. Os downloads normais de modelos do Chat/ditado continuam
iguais. A aceitação independente de qualidade em reuniões em PT/EN e de captura real segue aberta.

<a id="sources-and-licenses"></a>
## Fontes e licenças

| Componente | Fonte e licenças declaradas | Material de redistribuição incluído |
|---|---|---|
| pyannote segmentation-3.0 | [Modelo original](https://huggingface.co/pyannote/segmentation-3.0): pesos MIT; o código do pyannote é MIT. O checkpoint original exige o fornecimento de contato; este instalador usa a reexportação ONNX pública com hash fixo abaixo, sem aceitar essa exigência. | O aviso MIT do CNRS de 2022 contido no arquivo é preservado como `SEGMENTATION-LICENSE.txt`. |
| 3D-Speaker CAM++ | O [modelo exato dos pesos](https://modelscope.cn/models/iic/speech_campplus_sv_zh-cn_16k-common) e seus [metadados](https://modelscope.cn/api/v1/models/iic/speech_campplus_sv_zh-cn_16k-common) declaram Apache License 2.0; o [código do 3D-Speaker](https://github.com/modelscope/3D-Speaker) é Apache-2.0. A [receita de conversão](https://github.com/k2-fsa/sherpa-onnx/blob/master/scripts/3dspeaker/export-onnx.py) identifica esse modelo. | `CAMPLUS-LICENSE.txt` preserva a licença Apache-2.0 original; `CAMPLUS-NOTICE.txt` registra a procedência e a conversão. O Vults não modifica o grafo nem vincula o motor que o publica. |
| ONNX Runtime 1.31.0 CPU | [Release oficial da Microsoft](https://github.com/microsoft/onnxruntime/releases/tag/v1.31.0), MIT. O runtime não contém pesos de modelos. | A licença, os avisos completos de terceiros e o documento de privacidade do arquivo exato são preservados separadamente. Cabeçalhos, links simbólicos e bibliotecas de provedores não utilizadas não são instalados. |

O backend usa `ort` sob MIT OU Apache-2.0 e `knf-rs` sob MIT, com filtros nativos Apache-2.0.
O instalador opcional usa `tar` (MIT OU Apache-2.0), `flate2` (MIT OU Apache-2.0) e `bzip2`
(MIT OU Apache-2.0). Sua dependência em Rust puro `libbz2-rs-sys` possui a
[licença permissiva do bzip2](https://github.com/trifectatechfoundation/libbzip2-rs/blob/v0.2.5/COPYING).
O arquivo `BZIP2-LICENSE.txt` é incluído sem modificações; a exceção revisada no Cargo fica restrita
à versão 0.2.5. Nenhum motor de fala vinculado a código GPL é adicionado.

<a id="download-pins"></a>
## Downloads fixados

São artefatos fixos, não uma busca pela release mais recente. Os hashes dos grafos instalados
correspondem ao experimento V5.4. O instalador segue apenas redirecionamentos HTTPS aprovados
de releases/CDNs do GitHub.

| Artefato | Tamanho do download (bytes) | SHA-256 |
|---|---:|---|
| [Arquivo de segmentação](https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2) | 6.958.444 | `24615ee884c897d9d2ba09bb4d30da6bb1b15e685065962db5b02e76e4996488` |
| [Grafo CAM++](https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/3dspeaker_speech_campplus_sv_zh-cn_16k-common.onnx) | 28.281.138 | `f682b514c05d947ee3fa91cd6ec6c5c7543479a128373fa29b1faedccd21fd11` |
| [Arquivo do ONNX Runtime Linux x64](https://github.com/microsoft/onnxruntime/releases/download/v1.31.0/onnxruntime-linux-x64-1.31.0.tgz) | 11.408.003 | `cc5c72baf5ae5c8238a6841f0897227be2d02826b9cf98eaf02fdefaeeb03a57` |

Os três downloads somam **46.647.585 bytes**, sem os cabeçalhos HTTP. O grafo de segmentação
extraído tem 5.992.913 bytes e SHA-256 `220ad67ca923bef2fa91f2390c786097bf305bceb5e261d4af67b38e938e1079`.
A biblioteca extraída do runtime tem 29.345.384 bytes e SHA-256
`469849977cb60744ea160927e1a3a0643fd4c55c0aff743251c6d293402e334e`.
O [catálogo compilado](../../../crates/voice/src/meeting/assets/catalog.rs) também fixa os arquivos
extraídos de licença, avisos e privacidade. O pacote completo tem dez arquivos; os avisos fornecidos
também participam de sua impressão digital de revisão.

<a id="installation-behavior"></a>
## Comportamento da instalação

A instalação exige a impressão digital correspondente ao catálogo revisado e um gate fornecido
pelo chamador para cada tentativa HTTP, incluindo redirecionamentos. No app, expanda a seção
experimental, escolha **Revisar instalação**, leia as fontes/licenças, marque o consentimento e
escolha **Baixar e instalar** em até um minuto. A revisão lê e verifica arquivos locais sem HTTP.
Somente a webview de Configurações pode revisar, instalar, cancelar ou consultar o estado do
instalador; Reuniões oferece um link para esse responsável pela configuração. A revisão de uso
único fica vinculada ao manifesto e ao registro local de requisições.

O gate do app reserva cada tentativa HTTP de forma persistente antes do envio, com limite de 24
em qualquer janela de 24 horas, incluindo redirecionamentos, falhas e tentativas interrompidas.
Reiniciar ou limpar o histórico não zera esse limite; um registro ausente, substituído ou ilegível
recusa a tentativa revisada. Os rastros locais contêm somente IDs dos artefatos, resultados e
tempos, nunca áudio, transcrições ou URLs. A instalação continua durante a navegação; cancele
nesta seção ou encerre o app para interromper. Sair de uma revisão não confirmada a descarta.
Não há repetição, retomada, busca por atualizações automáticas ou uso de proxy. Uma tentativa
com falha exige nova revisão. O comando de desenvolvimento separado mantém o limite de doze
requisições por execução e não substitui o gate do app.

Atualizações de progresso são aceitas apenas para o download ativo da tentativa atual. Após o
cancelamento ou a conclusão, atualizações atrasadas são ignoradas; uma nova revisão atualiza o
estado da instalação.

Um diretório temporário privado e uma trava exclusiva isolam a tentativa.
O arquivo de trava precisa ser regular, pertencer a este usuário, ter modo 0600 e um único vínculo
físico. Travas inválidas são recusadas antes da remoção dos temporários abandonados, sem substituição
ou reparo automático.

Os downloads em fluxo e os membros selecionados dos arquivos compactados precisam corresponder ao tamanho e SHA-256.
Cada artefato tem um prazo de 180 segundos para rede e corpo, compartilhado entre redirecionamentos
e conferido entre blocos e no fim do fluxo, mesmo quando o próximo resultado já está disponível.
Uma tentativa com prazo vencido não pode reservar outra requisição HTTP. Cancelamento e prazo
são conferidos novamente após a reserva da cota e o recebimento da resposta; uma tentativa já
admitida continua contabilizada e seu rastro é encerrado como cancelamento ou falha.
A descarga e sincronização do arquivo local e a extração são trabalhos separados.
Downloads já cancelados param antes de preparar o cliente ou o arquivo temporário. O cancelamento
é conferido novamente após receber um bloco, antes de informar o progresso da escrita e após a
descarga/sincronização do arquivo. Um bloco que retorna com o cancelamento já definido é descartado
sem escrita ou progresso. Escritas já iniciadas podem terminar antes da verificação do cancelamento.

A extração rejeita caminhos inseguros, membros selecionados duplicados, links no lugar de arquivos
selecionados e excesso nos limites de expansão/itens. Nada do arquivo compactado é executado.
Somente um pacote completo verificado é publicado por renomeação atômica.
Os arquivos e seu diretório temporário são sincronizados antes da publicação; após a renomeação,
o diretório da instalação também é sincronizado. O cancelamento é conferido novamente após a
sincronização do diretório temporário. Se ela falhar, a publicação não ocorre e a limpeza é tentada.
Sincronizar somente os arquivos não garante a persistência das entradas do diretório; veja [fsync no Linux](https://man7.org/linux/man-pages/man2/fsync.2.html).

A verificação de prontidão confere cada arquivo novamente, inclusive os avisos. Cancelamento ou falha tenta remover
os temporários; um cancelamento posterior à renomeação final pode encontrar a instalação já concluída.
Uma verificação de prontidão já cancelada para antes de consultar o pacote ou abrir os arquivos.
A conferência do hash verifica o cancelamento após cada leitura, inclusive no fim do arquivo,
para não informar bytes verificados quando a checagem foi cancelada.
A extração confere o cancelamento antes de criar a saída, após ler os membros e sincronizar os
arquivos, e após o marcador final do arquivo compactado. Operações bloqueantes do sistema de arquivos
precisam retornar antes da verificação; uma extração cancelada nunca torna seus temporários parciais prontos.

Os diretórios das versões incluem a impressão digital de revisão, preservando as versões
instaladas e os runtimes carregados. Uma versão existente corrompida/incompleta é informada e
preservada, sem reparo ou substituição silenciosa. Temporários abandonados são removidos sob a
trava na próxima tentativa explícita. A recuperação verifica o cancelamento entre as entradas
abandonadas. Uma remoção recursiva já iniciada pode terminar; as entradas restantes aguardam a
próxima tentativa explícita. O encerramento de uma sessão nativa libera seus contextos,
enquanto a biblioteca do runtime fica fixa naquele processo; mudar sua versão exige novo processo.

<a id="runtime-privacy"></a>
## Privacidade do runtime

O processo define `ORT_DISABLE_TELEMETRY=1` antes de criar threads. O carregamento nativo exige
isso e também desativa a opção de telemetria da API. A [documentação de privacidade original](https://github.com/microsoft/onnxruntime/blob/v1.31.0/docs/Privacy.md#disabling-telemetry)
explica por que apenas a API não basta. São definições do processo, sem editar a configuração
do shell do usuário.

<a id="validation-status"></a>
## Estado da validação

O fluxo de desenvolvimento foi conferido com novos downloads públicos, instalação completa,
revalidação do cache sem HTTP, fluxos inválidos/cancelados e falhas de arquivos compactados.
O runtime 1.31.0 instalado passa nas verificações limitadas de fala pública, cancelamento e
retomada após dar prioridade ao trabalho interativo, com a privacidade garantida na inicialização.
Essa evidência não qualifica reuniões completas nem ativa identificação automática no app.


Uma regressão com processos separados mantém a trava real da instalação em um processo filho e
confirma que outra tentativa preserva seus temporários ativos. Após SIGKILL encerrar o filho sem
executar destrutores, a próxima tentativa readquire a trava, remove os temporários abandonados e
preserva um marcador de versão anterior. A checagem usa dados sintéticos privados; a aceitação
de queda de energia e falhas de armazenamento continua separada.

As verificações separadas do app cobrem revisão/consentimento, autoridade exclusiva de
Configurações, bloqueio da cota persistente antes de HTTP e cancelamento sem download. Os layouts
nativos Black/Light em 1180 x 820 e 480 x 360 são inspecionados; as telas de progresso e arquivos
verificados usam respostas controladas sem HTTP.
A instalação sozinha mantém a análise de falantes desativada. As compilações normais não oferecem
identificação. Uma compilação de desenvolvimento com `--features meeting-speakers` oferece uma
opção adicional, desmarcada para cada gravação revisada: **Também identificar vozes nesta gravação
(experimental)**. Essa prévia usa somente os artefatos locais instalados, preserva a transcrição
comum se a análise estiver indisponível e mantém falas incertas como **Voz desconhecida**. A escolha
é reiniciada para cada gravação. Microfone e áudio do sistema mantêm relógios e identificadores de
voz separados; não são sincronizados nem combinados.

O backend nativo pode processar em blocos até 30 minutos por fonte, com um único agrupamento
global de vozes. Uma execução de diarização de 30 minutos de áudio público em CPU levou 61,556
segundos e atingiu pico de 158.916 KiB de RSS. É um experimento de recursos, não uma transcrição/
captura completa nem qualificação independente de qualidade. O app agora aceita gravação manual
de quatro horas com armazenamento PCM16 limitado, mas a análise de falantes continua limitada a
30 minutos por fonte; gravações mais longas mantêm transcrição simples sem rótulos. Os testes
nativos de cancelamento e prioridade passaram,
inclusive o cancelamento após três minutos; operações bloqueantes de IO e construção nativa
continuam cooperativas. A prévia por fonte também foi inspecionada em janelas nativas isoladas
Black/Light com respostas controladas. Esses testes não cumprem os critérios independentes de
qualidade em PT/EN nem ativam identificação automática em produção.


A [referência de avaliação de falantes](meeting-evaluation.md) descreve as verificações de relógio
por fonte e a medição de erros com anotações fornecidas. Seus exemplos sintéticos não qualificam
fala real PT/EN nem ativam análise em produção.
