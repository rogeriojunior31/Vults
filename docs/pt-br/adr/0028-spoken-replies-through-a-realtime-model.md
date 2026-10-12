# 0028. Respostas faladas por um modelo em tempo real, opcional
<!-- source: 6307efbb76d2 -->

**Status:** Aceita, 2026-10-12. Substitui as respostas faladas e a transcrição na nuvem da
[0013](0013-voice-local-first.md); o resto da 0013 continua valendo.

<a id="context"></a>

## Contexto

A [0013](0013-voice-local-first.md) previa respostas faladas por um motor local de licença
permissiva, e uma transcrição na nuvem opcional. A voz local que saiu (Kokoro, com fonemas do
`espeak-ng`) foi removida em 2026-10-07: soava ruim demais para ficar, sobretudo em português. O
`crates/voice` não tem síntese de fala hoje. A transcrição na nuvem nunca foi construída, e o
usuário a tirou do plano em 2026-10-12.

<a id="decision"></a>

## Decisão

- A fala em texto continua local: whisper.cpp, como diz a [0013](0013-voice-local-first.md).
- As respostas faladas vêm de um modelo em tempo real que o usuário liga (`docs/dev/plan-zeca.md`,
  K8). É uma requisição em nome do usuário ([0019](0019-requests-on-the-users-behalf.md)):
  desligada por padrão, com sua própria chave, orçamento e registro, pelo portão de rede do `app`.
  A chave de API fica só no chaveiro do sistema.
- Ligá-lo avisa que o áudio sai da máquina e para onde vai.
- Nada de transcrição na nuvem.
- A voz nunca responde a um card ([0004](0004-a-human-answers-permissions.md)), diga o modelo o que
  disser.

<a id="consequences"></a>

## Consequências

- Com o recurso desligado, o Zeca não fala; nada sobre voz sai da máquina.
- Nenhum motor de fala ou voz vai no nosso binário, então as verificações de licença da 0013 valem
  só para o que ainda roda localmente (o whisper.cpp e seus modelos).
