# 0028. Voz: escuta local, respostas faladas opcionais
<!-- source: 4d76d0e13404 -->

**Status:** Aceita, 2026-10-12. Consolidada em 2026-10-12 a partir da 0013.

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
  uma; modelos baixados pelo Settings e conferidos por SHA-256. O áudio fica na memória e nunca é
  salvo. Sem transcrição na nuvem.
- **Sem palavra de ativação e sem microfone sempre ligado**: o microfone só abre quando o usuário
  pede.
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

<a id="history"></a>

## Histórico

- 0013, 2026-10-04: fala para texto local, nada de microfone sempre
  ligado, nada de GPL no binário, a voz nunca responde a um card. As respostas faladas locais e a
  transcrição opcional na nuvem dela foram substituídas aqui.
