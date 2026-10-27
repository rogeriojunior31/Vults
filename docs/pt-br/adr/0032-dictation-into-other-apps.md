# 0032. Ditado em outros aplicativos
<!-- source: 37d3afdb8881 -->

**Status:** Aceita, 2026-10-17, sob a instrução do mantenedor para continuar a N3 com os padrões
planejados. Este é o contrato da V0.2; captura, inserção e validação nativa chegam separadamente.

<a id="context"></a>

## Contexto

A transcrição local já preenche o campo de composição do Chat do Vults. A N3 a estende ao
aplicativo ou à sessão do bando que o usuário estava usando ao pressionar a tecla de fala.
Texto pode virar ação num terminal: um Enter, inclusive embutido numa transcrição, pode
responder ao prompt de um agente. O foco também pode mudar durante a transcrição. A autoridade
de permissão deve continuar com o humano sob a [0004](0004-a-human-answers-permissions.md).

<a id="decision"></a>

## Decisão

- **Capturar a intenção ao pressionar a tecla.** Ler a janela em foco por EWMH no X11 ou por um
  script do KWin no Plasma Wayland, reutilizando a integração de plataforma. Guardar a identidade
  da janela e, quando conhecidas, a sessão exata do bando e o painel do multiplexador. O título
  da janela é contexto, não prova de identidade. Conferir novamente o destino imediatamente
  antes da entrega; não colar em outro aplicativo em primeiro plano nem trazer outro aplicativo
  à frente para recuperar um destino perdido. Destinos desconhecidos, alterados ou fechados
  resultam em cópia para a área de transferência. Outros desktops Wayland usam a mesma
  alternativa até serem suportados.
- **A inserção pertence a `platform`; a orquestração, a `app`.** O X11 usa XTest para o atalho de
  colar. O Wayland usa o portal RemoteDesktop, solicitando apenas entrada de teclado depois que
  o usuário habilita a inserção. Explicar que o desktop concede a capacidade de digitar em
  outros aplicativos. Uma autorização recusada, ausente ou revogada resulta em cópia; nunca
  contorná-la com outro serviço de entrada. O token de restauração é um segredo: guardá-lo só
  no chaveiro do sistema por `secrets`, nunca em configurações, banco, logs ou armazenamento
  do renderer. Sem chaveiro utilizável, não persisti-lo. Desligar a inserção fecha a sessão e
  remove o token armazenado.
- **Usar uma posse temporária da área de transferência de texto.** Preservar o texto anterior
  na memória, definir a transcrição e solicitar exclusão do histórico onde houver suporte.
  Após cerca de 700 ms, restaurar o texto anterior só se a área de transferência ainda
  pertencer àquela operação e não tiver mudado; nunca sobrescrever uma cópia mais recente.
  Operações sobrepostas não podem restaurar textos antigos umas das outras. Não gravar o
  conteúdo em disco. Explicar que a restauração cobre só texto, não imagens ou formatos ricos,
  e que a exclusão do histórico depende do desktop. A alternativa de apenas copiar deixa a
  transcrição disponível para colar manualmente, em vez de restaurar o conteúdo anterior.
- **A entrega ao agente exige destino exato e seguro.** Usar um multiplexador suportado só
  quando a identidade do painel for conhecida e ainda pertencer à sessão capturada. Não enviar
  texto, atalho de colar ou Enter para uma sessão aguardando permissão ou pergunta, nem enquanto
  seu card estiver aberto. Conferir o estado atual do core na entrega, não só na captura. Um
  card adiado continua aguardando. Quando não for possível estabelecer a identidade ou a
  segurança da sessão num destino de terminal, apenas copiar. Nenhuma entrada fica na fila
  para executar automaticamente depois que um card for respondido.
- **Inserir texto sem enviá-lo.** O envio automático fica desligado por padrão. Uma futura
  opção explícita de envio automático não pode ignorar a proteção de cards pendentes. A
  inserção em terminal também precisa tratar CR/LF embutidos e caracteres de controle com
  segurança; remover o Enter final é insuficiente. Se um caminho suportado de colagem literal
  ou delimitada não puder estabelecer uma inserção segura, apenas copiar. Ditado nunca mapeia
  palavras faladas para uma decisão de permissão. Comandos de voz usam sua tecla separada e
  nunca entram neste caminho de inserção.
- **Informar só o que a entrega comprova.** Mostrar *Na sessão* quando o transporte suportado
  do painel confirmar a entrega, *Colado* quando o envio de colagem para o destino em primeiro
  plano validado tiver sucesso e *Copiado* quando o texto só tiver sido copiado. Esses estados
  não afirmam que outro aplicativo enviou, executou ou salvou algo. Falha de escrita na área de
  transferência é erro, nunca *Copiado*.
- **Ditado é independente do Zeca.** Com Zeca desligado, a tecla de fala, transcrição local,
  inserção e histórico continuam funcionando. O atalho de fala sempre usa o ditado independente,
  mesmo sobre janelas do Vults. Somente o botão de microfone do Chat preenche diretamente seu
  rascunho; nunca envia uma mensagem automaticamente. A ilha
  nunca toma o foco do destino para mostrar o progresso do ditado nem esconde um card pendente.
- **O histórico guarda só texto, com retenção.** Guardar texto ouvido e entregue, resultado e
  tempos no armazenamento local, não o áudio nem o conteúdo anterior da área de transferência.
  Oferecer uma escolha de retenção e *Não guardar*; essa opção ainda permite a execução na
  memória sem gravar sua transcrição no histórico, nos logs ou nos registros de rede. Limpeza
  por modelo e voz remota continuam sendo adesões separadas sob a
  [0019](0019-requests-on-the-users-behalf.md) e a [0028](0028-spoken-replies-through-a-realtime-model.md).

<a id="natural-voice-commands-vc1"></a>
## Comandos de voz naturais (VC1)

A tecla separada de comando solicita uma ação, não inserção de texto. O catálogo da 0.1.9 é limitado:

| Escopo | Pedidos suportados | Destino e execução |
|---|---|---|
| Bando | Navegação existente, consultas de sessão/uso/GitHub, silenciar notificações/fixar/ocultar projeto, repetir e desfazer condicionalmente | Identidade atual da sessão/projeto; nunca uma resposta de permissão |
| Mídia | Pausar, retomar, próxima, anterior e volume do reprodutor | Reprodutor MPRIS ativo explícito; respeitar a opção de mídia e esclarecer múltiplos destinos |
| Áudio do computador | Silenciar/reativar saída padrão, definir volume, aumentar/reduzir em 10 pontos percentuais | Capturar e revalidar a identidade da saída; limitar volume a 0..100 por cento |
| Aplicativos | Abrir um aplicativo instalado | Identidade validada de entrada desktop executável; sem programa, URI, arquivo ou argumentos fornecidos |

- Ações locais de mídia, saída do computador e abertura de aplicativos têm, cada uma, uma opção
  própria desligada por padrão nas configurações de comandos. Funcionam com o Zeca desligado.
  Silenciar projeto significa silenciar notificações, nunca o navegador ou sistema. `Abrir <nome>`
  resolve projetos e aplicativos instalados; colisões perguntam qual escopo o usuário quis.
  Aplicativo ausente é informado como não instalado.
- Usar o modelo de transcrição de voz existente para ditado e comandos. O ditado entrega texto;
  comandos interpretam essa transcrição localmente com frases limitadas e resolução de destinos.
  Não é necessária integração adicional com Claude, provedor de texto ou consentimento de rede
  para intenção. O envio opcional de fala desconhecida ao Chat continua separado sob a
  [0019](0019-requests-on-the-users-behalf.md), nunca obrigatório para ações locais.
- Aceitar variações completas de pedidos suportados, sem procurar palavras soltas. Limitar entrada
  a 4096 bytes e lotes a oito ações. Frases desconhecidas e nomes não resolvidos não viram efeitos
  adivinhados. O modelo de voz transcreve; não concede autoridade de execução nem garante compreensão
  semântica geral. Manter ações tipadas e identidades resolvidas localmente no aplicativo.
- Revalidar destino, opções, bloqueio e cancelamento antes de cada efeito. Resultado atrasado ou
  duplicado nunca executa. Sem repetição automática, shell ou argumentos de programa falados.
- Fala negada, citada, informativa ou ambígua não pode virar ação adivinhada. Esclarecer destino ou
  escopo de áudio incerto. Voz nunca aprova, nega, libera ou modifica cartões de permissão, edita
  configurações de agentes, faz merge, fecha aplicativos ou executa operações destrutivas.
- Validar o lote antes dos efeitos. Se um efeito externo falhar, parar e informar o trecho realmente
  concluído; não prometer reversão nem repetir ações já feitas. Não restaurar redução/pausa
  temporária do ditado sobre uma mudança deliberada por voz. Desfazer depende de estado anterior
  conhecido e restaurável; nunca encerra um aplicativo aberto pelo comando.
- Mostrar texto ouvido, destino, esclarecimento e resultado real no feedback/histórico existente,
  respeitando retenção. Nunca informar Concluído antes da execução nativa. Ações desligadas,
  ausência de destino, backend nativo ausente, destinos desatualizados, cancelamento,
  tempo excedido e falhas nativas geram explicação local e nenhum efeito. São resultados
  distintos, não um genérico Isso não é um comando. Transcrição e ditado comum continuam usáveis.

<a id="consequences"></a>

## Consequências

- *Copiado* é um resultado suportado quando não for possível estabelecer foco, permissões ou
  segurança do terminal. O Vults nunca transforma falha de transcrição em permissão para
  digitar em outro lugar.
- A posse temporária da área de transferência pode substituir conteúdo não textual, e o
  gerenciador do desktop pode reter uma cópia. Settings deve explicar esses limites antes de
  a inserção ser habilitada.
- Os testes cobrem mudanças de foco, mudanças de posse e operações sobrepostas na área de
  transferência, cards pendentes e adiados, texto multilinha no terminal, identidade de painel
  ausente, acesso ao portal revogado e uso com Zeca desligado. A validação nativa no X11 e no
  Plasma Wayland confere o destino, autorização de teclado, comportamento da área de
  transferência e inserção real. A ilha continua mapeada uma vez no tamanho fixo; só sua região
  de entrada muda. Verificações automatizadas não substituem essa validação no desktop.

<a id="history"></a>

## Histórico

- 2026-10-26: correção do responsável substitui a proposta de modelo de texto separado pela
  transcrição de voz existente e interpretação local; retira consentimento e transporte não ativados.

- 2026-10-26: a VC1 propôs inicialmente ações limitadas e consentimento independente de modelo; a correção
  acima substitui o escopo de modelo de texto separado.

- 2026-10-17: definir os limites da N3 V0.2 para destino, área de transferência, capacidade de
  teclado e permissões. Usar 0032 porque a [0031](0031-tasks-leave-vults.md) já registra a remoção
  de Tasks.

- 2026-10-19: reservar o atalho global de fala para o ditado independente, seja qual for o foco;
  a entrada de voz no rascunho do Chat continua sendo uma ação explícita no botão de microfone.
