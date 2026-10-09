# 0013. A voz é local primeiro; nuvem só quando escolhida
<!-- source: df70847d9fdc -->

**Status:** Aceita, 2026-10-04 (o apertar para falar saiu no #15).

<a id="context"></a>

## Contexto

A voz é como muitos usuários querem falar com o Zeca e, mais tarde, ouvi-lo. Áudio é pessoal, e os
melhores motores ou são serviços na nuvem ou são modelos locais grandes.

<a id="decision"></a>

## Decisão

- A conversão de fala em texto roda neste computador: whisper.cpp via `whisper-rs`, na GPU via
  Vulkan quando houver uma; modelos baixados pelo Settings e verificados por SHA-256. O áudio fica
  na memória e nunca é salvo.
- A transcrição na nuvem é opcional (opt-in), com a chave no chaveiro do sistema e um aviso claro de
  que o áudio sai da máquina.
- Respostas faladas são opcionais e locais, com um motor e vozes de licença permissiva. Nada de
  código GPL no nosso binário: o Piper (GPL-3.0) está fora, e também o build padrão do crate
  `sherpa-onnx`, que faz link com o `espeak-ng` (GPL-3.0). Uma ferramenta GPL que o usuário instalou
  pode rodar como processo separado.
- Sem palavra de ativação e sem microfone sempre ligado: o microfone só abre quando o usuário pede.
- A voz nunca responde a um card de permissão ([0004](0004-a-human-answers-permissions.md)).

<a id="consequences"></a>

## Consequências

- Downloads maiores e um caminho de GPU para manter funcionando.
- Cada novo motor ou voz tem a licença verificada (código e modelo) antes de entrar.
