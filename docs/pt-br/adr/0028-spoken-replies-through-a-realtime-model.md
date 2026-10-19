# 0028. Voz: escuta local, respostas faladas opcionais
<!-- source: 6271effb320a -->

**Status:** Aceita, 2026-10-12. Consolidada em 2026-10-12 a partir da 0013.
Alterada em 2026-10-17 para ditado, escuta limitada de respostas e áudio temporário de reuniões.
Alterada em 2026-10-19 para rascunhos de perguntas com confirmação humana (V7.1). A V7.2.2 entrega
perguntas digitadas da M7; ditado local no editor delas e oferta a agentes externos continuam em etapas separadas.
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
- **A voz nunca responde a uma permissão** ([0004](0004-a-human-answers-permissions.md)), diga o
  modelo o que disser. O ditado só pode editar um rascunho de pergunta elegível pelo contrato de
  confirmação separado abaixo; a fala por si só nunca envia a resposta de um card.

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
- **A autoridade sobre permissões não muda.** Nenhuma destas cláusulas altera a
  [0004](0004-a-human-answers-permissions.md). Texto falado nunca é interpretado como aprovação
  ou negação. O contrato de rascunhos de perguntas abaixo permite entrada de texto, com uma ação
  humana separada para enviar.

A resposta de oito segundos e os arquivos temporários de reunião são as exceções explícitas
às regras de escuta e armazenamento. O ditado em outros aplicativos segue a
[0032](0032-dictation-into-other-apps.md).

<a id="human-confirmed-question-drafts"></a>
## Rascunhos de perguntas com confirmação humana

A V7.1 define este contrato para o compositor de perguntas `ask_user` da M7 e
`vults_dictate(question)`. A V7.2.2 implementa o `ask_user` digitado da M7 para clientes verificados
iniciados pelo app no Linux. A V7.2.3 conecta ditado local no rascunho pela tecla física e o alias
`vults_dictate` para esses clientes. A V7.3 adiciona o consentimento externo desligado por padrão,
um servidor exclusivo de perguntas e a configuração revisada do Claude Code/Codex. Esta decisão
por si só não adiciona controle de microfone nem instalação.

- **Uma pergunta de texto é um tipo de pedido distinto.** Somente o compositor de perguntas
  tipado e sem autoridade de permissão do app é elegível. Um rótulo do cliente, o texto da pergunta
  ou uma transcrição não pode transformar uma permissão em pergunta de texto. A resposta é texto,
  nunca Allow, Deny, regra Always, mudança de política ou decisão pelo celular. Os caminhos de
  permissões e perguntas por hook existentes não ganham acesso ao ditado com esta decisão.
- **Um agente não pode abrir o microfone.** O usuário inicia a captura local, somente na memória,
  na pergunta exibida, pressionando sua tecla de ditado. O pedido do agente, foco,
  notificações e escuta de respostas em tempo real nunca iniciam nem estendem essa captura. A opção
  *Agentes podem pedir uma resposta falada* vem desligada; configurar MCP, por si só, não concede
  acesso à voz. A V7.3 aplica essa restrição a clientes externos sem ampliar a autoridade de permissões.
- **A fala edita um rascunho; o humano envia.** Mostre a transcrição final em um compositor editável.
  Enviar exige um clique separado ou Enter sem modificadores nesse compositor em foco, após a
  captura terminar. Soltar a tecla, silêncio, concluir a transcrição, falar "enviar" ou um pedido
  do agente/modelo não pode enviar. Shift+Enter e composição por IME mantêm seu comportamento de
  edição. Transcrição vazia, falha ou cancelada nunca envia. Rascunhos de perguntas usam somente
  transcrição local: sem limpeza opcional por modelo, requisição a provedor, comandos de voz ou
  encaminhamento ao Chat.
- **Vincular toda a troca ao seu dono.** A M7 deve verificar o cliente solicitante pela ascendência
  de processos, sem confiar em um nome/PID informado. Vincule captura, rascunho e envio confirmado
  a esse cliente, à pergunta ativa exata e à sua geração/prazo. Confira esses vínculos novamente ao
  publicar o texto e enviá-lo; consuma uma resposta confirmada no máximo uma vez. Substituição,
  expiração, desconexão do cliente, navegação, ocultar/fechar ou bloqueio da tela invalidam trabalho
  pendente e apagam o rascunho não enviado. Áudio, decodificação ou confirmações de envio atrasadas
  não podem preencher nem responder outra pergunta. Falha de envio é explícita; uma nova tentativa
  não pode duplicar uma resposta já consumida.
- **Separar autoridade, privacidade e instalação.** O áudio da pergunta fica na memória para
  transcrição local; nada vai para um modelo remoto nem para o disco. Uma resposta confirmada vai
  somente ao cliente solicitante verificado. A M7 registra
  localmente o resultado com o vínculo do pedido e o solicitante verificado, sem áudio, segredos
  nem uma cópia da transcrição. Nenhum caminho de voz alcança `Intent::Decide` ou `DecideAlways`.
  Configurar MCP em agentes externos continua exigindo backup datado, diff visível e clique pela
  [0005](0005-agent-configs-backup-diff-click.md). A
  [0027](0027-an-open-reply-for-any-tool.md) e seu protocolo de respostas do hook não mudam.

A V7.2 começa pelo ciclo de vida tipado de perguntas da M7, identidade verificada do cliente,
cancelamento, resposta de uso único e teste completo de auditoria. Depois conecta o ditado local
e o compositor. Seus testes de regressão devem cobrir donos forjados/substituídos, gerações antigas,
bloqueio/saída, confirmação duplicada, confirmação de recebimento perdida, IME/Enter e texto literal
parecido com aprovação que nunca se torna uma decisão de permissão. A V7.3 oferece a função aos
agentes separadamente, com a opção desligada por padrão e edição revisada da configuração.
Até essas etapas chegarem, o comportamento atual de cards e microfone permanece como foi entregue.

<a id="history"></a>

## Histórico

- 2026-10-19: a V7.3 adiciona perguntas externas desligadas por padrão, ordenação da revogação de
  consentimento e configuração MCP revisada somente em Configurações. As regras de permissões
  existentes e de captura pela tecla física não mudam.

- 2026-10-19: a V7.2.3 implementa ditado local em perguntas pela tecla física com aplicação/envio
  separados e verificações de cancelamento. A configuração externa continua na V7.3; a autoridade
  sobre permissões não muda.

- 2026-10-19: a N3 V7.1 distingue o rascunho de pergunta ditado da autoridade sobre permissões.
  Somente uma confirmação humana separada pode enviá-lo; o ciclo de perguntas da M7 e as restrições
  de V7.2/V7.3 continuam sem implementação. A ADR 0004 e a ADR 0027 não mudam.
- 2026-10-17 (aceita pelo mantenedor): a N3 V0.1 define vocabulário local, escuta opcional
  limitada para respostas, áudio temporário de reuniões e a comprovação de licença exigida
  para novos modelos.
- 0013, 2026-10-04: fala para texto local, nada de microfone sempre
  ligado, nada de GPL no binário, a voz nunca responde a um card. As respostas faladas locais e a
  transcrição opcional na nuvem dela foram substituídas aqui.
