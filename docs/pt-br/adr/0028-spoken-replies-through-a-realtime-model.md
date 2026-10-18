# 0028. Voz: escuta local, respostas faladas opcionais
<!-- source: 8f1209c5c71a -->

**Status:** Aceita, 2026-10-12. Consolidada em 2026-10-12 a partir da 0013.
Alterada em 2026-10-17 para ditado, escuta limitada de respostas e áudio temporário de reuniões.
A V0.1 foi aceita pelo mantenedor; a implementação da N3 continua em etapas.

<a id="context"></a>

## Contexto

A voz é como muitos usuários querem falar com o Zeca, e ouvi-lo. Áudio é pessoal, e os melhores
motores são serviços na nuvem ou modelos locais grandes. O push-to-talk saiu no #15. A voz local que
saiu para as respostas faladas (Kokoro, com fonemas do `espeak-ng`) foi removida em 2026-10-07:
soava ruim demais para ficar, sobretudo em português. A transcrição na nuvem nunca foi feita, e o
usuário a tirou do plano em 2026-10-12.

<a id="decision"></a>

## Decisão

- **A fala vira texto neste computador**: whisper.cpp via `whisper-rs`, na GPU via Vulkan quando há
  uma; modelos baixados pelo Settings e conferidos por SHA-256. O áudio do ditado e do chat fica na memória; reuniões têm a exceção temporária descrita
  abaixo. Sem transcrição na nuvem.
- **Sem palavra de ativação e sem microfone sempre ligado**: o microfone só abre quando o usuário
  pede, incluindo a opção explícita e limitada de resposta descrita abaixo.
- **As respostas faladas vêm de um modelo em tempo real que o usuário liga.** É uma requisição em
  nome do usuário ([0019](0019-requests-on-the-users-behalf.md)): desligada por padrão, com
  interruptor, orçamento e registro próprios, pelo portão de rede do `app`. A chave dela vive só no
  chaveiro do sistema. Ligá-la diz que o áudio sai da máquina e para onde vai.
- **Nenhum código GPL no nosso binário**: o Piper (GPL-3.0) fica de fora, e também o build padrão
  do crate `sherpa-onnx`, que liga o `espeak-ng` (GPL-3.0). Uma ferramenta GPL que o usuário
  instalou pode rodar como um processo separado. Cada motor ou modelo que roda localmente tem a
  licença conferida (código e modelo) antes de entrar.
- **A voz nunca responde a um card** ([0004](0004-a-human-answers-permissions.md)), diga o modelo o
  que disser.

<a id="consequences"></a>

## Consequências

- Com o recurso desligado, o Zeca não fala; nada sobre voz sai da máquina.
- Nenhum motor de fala nem voz vai no nosso binário, então as checagens de licença valem para o
  whisper.cpp e os modelos dele.
- Downloads maiores e um caminho de GPU para manter funcionando.

<a id="dictation-and-meetings"></a>

## Ditado e reuniões

Estas cláusulas foram aceitas na V0.1 do [plano de Ditado](../../dev/plan-dictation.md#v0-decisions-and-the-prototype).
Elas definem o contrato da N3; o aceite não significa que os recursos já foram implementados:

- **Palavras locais, transcrição local (R1).** Um dicionário local, substituições e trechos
  prontos podem corrigir a transcrição; o dicionário e o vocabulário do projeto também podem
  orientar o whisper pelo `initial_prompt`. Essas etapas não fazem requisições de rede. A
  transcrição na nuvem continua de fora. A limpeza opcional por modelo é uma requisição
  separada, desligada por padrão, sob a [0019](0019-requests-on-the-users-behalf.md), com o texto
  ouvido preservado e retorno ao texto bruto em caso de falha; não faz parte da transcrição local.
- **Escuta explícita e limitada (R3).** Uma tecla ou um clique inicia a captura. Um duplo toque
  pode travar o ditado em mãos livres, com estado de escuta visível e um caminho explícito para
  parar/cancelar. Não se adicionam palavra de ativação nem escuta em segundo plano. Uma opção
  separada, *Responder sem a tecla*, fica desligada por padrão: depois que o usuário a ativa,
  Zeca pode reabrir o microfone por no máximo oito segundos após sua própria pergunta falada
  numa conversa de voz ativa. A ilha mostra o microfone aberto e o usuário pode cancelar.
  Expiração ou cancelamento o fecham; notificações, resultados de ferramentas e cards de
  permissão não o abrem nem prolongam.
- **Reuniões são a única exceção para áudio em disco (R5).** Cada gravação começa por uma ação
  do usuário e uma linha de consentimento explicando quais fontes são capturadas e que o áudio
  é salvo temporariamente para transcrição local. Os WAVs temporários são privados do usuário
  e apagados quando a transcrição termina, falha ou é cancelada; a inicialização remove sobras
  de uma execução interrompida. Não há arquivo permanente de áudio nem gravação automática.
  O ditado comum e a captura do chat continuam na memória. Só ficam a transcrição e as notas
  solicitadas explicitamente, sob sua configuração de retenção. O resumo por modelo é uma
  requisição separada, desligada por padrão, pelo portão de rede.
- **Conferir código e pesos antes de habilitar um download.** A regra de licença existente vale
  para cada modelo adicionado, incluindo smart-turn v3, pyannote segmentation-3.0 e CAM++:
  registrar origem, licença do código, licença dos pesos, condições de redistribuição e SHA-256
  do download na referência de modelos antes de oferecê-lo. Um modelo proposto não está
  aprovado por aparecer no plano. As dependências continuam passando pelo `cargo deny`;
  nenhum motor ligado a GPL entra no binário.
- **A voz continua sem responder a um card.** Nenhuma destas cláusulas altera a
  [0004](0004-a-human-answers-permissions.md). Texto falado nunca é interpretado como aprovação
  ou negação. Um futuro campo de composição em cards de pergunta exige sua própria decisão
  e uma confirmação humana separada para enviar; esta emenda não o habilita.

A resposta de oito segundos e os arquivos temporários de reunião são as exceções explícitas
às regras de escuta e armazenamento. O ditado em outros aplicativos segue a
[0032](0032-dictation-into-other-apps.md).

<a id="history"></a>

## Histórico

- 2026-10-17 (aceita pelo mantenedor): a N3 V0.1 define vocabulário local, escuta opcional
  limitada para respostas, áudio temporário de reuniões e a comprovação de licença exigida
  para novos modelos.
- 0013, 2026-10-04: fala para texto local, nada de microfone sempre
  ligado, nada de GPL no binário, a voz nunca responde a um card. As respostas faladas locais e a
  transcrição opcional na nuvem dela foram substituídas aqui.
