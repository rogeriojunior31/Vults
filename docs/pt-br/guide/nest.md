# O Nest
<!-- source: 6e110a5a59a7 -->

O Nest é a janela do próprio Vults, para quando a ilha fica pequena: o bando inteiro, o
chat, o que aconteceu e as suas conexões. Abra pelo ícone da bandeja (**Nest…**) ou pela
seta no topo do chat da ilha. Ele é criado quando você abre e some quando você fecha, e abre também
com o Zeca desligado, só que sem o Chat. Enquanto ele está aberto a ilha sai, com pílula e aves: uma
superfície na tela por vez. O Nest não tem a barra de título do sistema: o cabeçalho dele o move (um
clique duplo ocupa a tela) e as bordas e os cantos o redimensionam. Ele abre grande da primeira vez
(até 1180 × 820, dentro da tela), e depois no tamanho em que você o deixou. Ela volta quando o Nest fecha ou volta para **Quick** ou **Mini**. Um
card que espera é respondido onde você está: no Nest enquanto você está nele (a janela dele tem o
foco), e na ilha, que aparece para ele, enquanto você está em outro lugar.

Atividade, Conexões e Configurações usam controles neutros, títulos, cards e estados de leitura
compartilhados. **Settings → General → Theme** escolhe **Black** (o padrão) ou **Light** em todas
as janelas. Atividade e Conexões usam abas sublinhadas; as cores de status mantêm seus significados. O palco do Flock,
os diffs e os controles do Compact seguem o mesmo tema, incluindo a atualização ao vivo
das cores dos agentes nas aves e do fio.

Em larguras de pelo menos 900 px, arraste o separador de Settings, Connections, Dictation, Recent ou Memory
para redimensionar esse painel lateral. Um clique duplo restaura a largura; o botão de recolher
mantém uma faixa estreita. O separador também aceita as setas, Home/End e Enter. A largura e o
estado recolhido ficam salvos localmente, sem alterar seleção ou rascunhos. Janelas estreitas
mantêm a navegação existente.

As telas de leitura usam os mesmos painéis de carregamento, conteúdo vazio, busca sem resultado
e erro, com uma explicação curta e a próxima ação disponível. A primeira leitura não mostra
registros vazios nem resumos zerados enquanto carrega. Se a atualização de Atividade falhar,
os últimos registros, projetos ou resumo semanal continuam visíveis com um aviso de possível
desatualização e **Try again**; uma falha na primeira leitura encerra o aviso de carregamento
e oferece nova tentativa. Erros técnicos de leitura ficam em **Technical details**. Tentar
de novo só lê a tela; nunca inicia sessão, responde permissão nem altera o histórico.

Minimizar o Nest devolve a ilha e seus atalhos de aprovação. Abrir o Nest de novo
restaura a janela; minimizar não altera o tamanho salvo da Sala.

Reabrir pela bandeja mantém a última aba e subseção de Atividade até você sair do Vults.
Abrir uma tela específica tem prioridade. Com Zeca desligado, uma aba Chat salva abre o Flock.

Fechar pelo comando do desktop (como **Alt+F4**) também espera o salvamento do
rascunho do Chat, da navegação e das buscas nos registros. Se salvar ou fechar
falhar, a janela continua aberta com um aviso; tente fechar de novo. Isso também
salva o estado do Nest quando as Configurações estão visíveis na janela compartilhada.
Fechar não responde uma permissão pendente nem interrompe uma execução.

O seletor de tamanho e **Back to the island** esperam a restauração inicial do
rascunho do Chat e os salvamentos pendentes. Cliques repetidos durante a troca de
tela usam uma tentativa só. Se salvar ou mudar a tela falhar, o Nest mantém o
rascunho e mostra um aviso; tente o mesmo controle de novo.

Em uma janela baixa do Chat, o compositor ocupa menos espaço e as sugestões da
conversa vazia rolam quando necessário. Isso também vale para janelas largas,
mantendo os controles do cabeçalho e o rodapé acessíveis.

Quando há espaço, o cabeçalho conta o bando (*4 sessions · 1 needs you*) e traz a música que o seu player está tocando
(com o Now playing ligado; anterior, tocar/pausar e próxima ao lado), a lua enquanto o não perturbe
dura (um clique o encerra), o som, a busca (a paleta, como **Ctrl+K**), o seletor de tamanho e o
botão de tela cheia. O mesmo cabeçalho traz cinco abas com os seus
ícones, o número que precisa de você em **Flock** e as linhas dos conectores em **Connections**, e
**Settings** à direita, que mostra as Configurações nesta mesma janela, sob as mesmas abas; uma aba
traz o Nest de volta. Sobre todas as abas menos **Flock**, uma etiqueta para cada sessão que precisa
de você (ou que falhou); um clique abre **Flock** com ela na frente (um card que espera vem
primeiro ali), onde você o responde. As Configurações continuam uma página própria ali: são a única que muda a
config de um agente, uma chave, uma política ou um celular, e nunca mostram texto de agente. O
seletor mantém a aba em que você está: **Room** é a janela, o botão de tela cheia a tela toda
(**Esc** volta para Room), **Quick** fecha o Nest e abre a ilha, **Mini** o fecha e deixa a pílula.
**Alt+1** a **Alt+5** escolhem uma aba. Embaixo, **Zeca on · optional** / **Zeca off** liga e
desliga o Zeca (como **Settings → Flock & Zeca → Zeca**), o lápis (**Zeca's look**) abre a página dele nas
Configurações, **Open terminal** traz o terminal da sessão da frente e **Back to the island** traz a
ilha.

Os interruptores e seletores de Configurações mostram salvando, salvo ou uma falha local.
Enquanto a linha salva, novas mudanças esperam mesmo se a página atualizar. Uma falha
restaura a escolha anterior; uma gravação malsucedida nunca aparece como concluída.

**Ctrl+K**, no Nest ou na ilha aberta (quando ela tem o teclado: aberta com Ctrl+Alt+Space, ou no
chat dela), abre a paleta: digite para pular para uma sessão, para
começar `/continue` ou `/remember` no chat (com o Zeca ligado), ou para abrir uma
aba. Ela nunca responde um card: uma sessão que espera por você só leva até o card dela.

As Configurações usam um título por página e agrupam os controles relacionados de
forma consistente. A referência de atalhos em Geral e o diagnóstico de capacidades
em Agentes abrem sob demanda. Chat e voz mantém as explicações extensas de setup,
acesso e API junto dos controles em **Mais**. Explicações abertas e a posição de
rolagem sobrevivem a atualizações; um campo focado mantém o texto ainda não salvo
e sua seleção durante uma atualização não relacionada.

O **Now** do Quick abre uma conexão diretamente no detalhe selecionado, limpando um filtro
antigo que a ocultaria; uma proposta de memória abre o painel Memory do Chat. Nenhum desses
atalhos executa uma ação. Voltar de Activity ou Connections para o Quick abre o Now.

## Flock

O bando fica num fio, com o triplo do tamanho, e o nome do projeto sob cada ave. Embaixo,
uma lista de sessões mostra o estado, o agente, o número do passo, o tempo em execução informado
e o uso de contexto de cada projeto. Selecione uma ave ou linha (ou use **↑ ↓**) para trazer a
sessão para a frente. Passe o mouse para ver projeto, branch, agente e estado completos. O clique
direito, a tecla de menu ou **Shift+F10** abre as ações rápidas existentes: terminal, pasta,
Activity, último diff, foco, silenciar, fixar e esconder. Um card aguardando mantém a própria
sessão na frente.

Acima da lista, **All sessions**, **Needs you**, **Working**, **Failed** e **Quiet sessions**
filtram as linhas. Quiet inclui sessões em repouso e aves sinalizadas por silêncio. As contagens
cobrem todo o bando, mesmo durante uma busca. **Find a session…** lê projetos, repositórios,
branches, agentes e passos atuais. Filtrar mantém a sessão selecionada e seu card no lugar.
Lista e detalhes rolam de forma independente. Num Nest estreito, **Back to sessions** abre a
lista; escolher uma linha volta aos detalhes dela. Numa janela baixa, o fio cede espaço ao
conteúdo selecionado. Uma nova solicitação volta ao seu card, inclusive com a lista estreita aberta.

Busca, filtro de estado, aba de detalhes e diff escolhido sobrevivem ao fechar e reabrir o Nest
ou voltar para a ilha, até sair do Vults. **Clear filters** limpa busca e estado, mantendo o
detalhe selecionado. Com o Zeca desligado, a aba dele salva abre **Session**. Restaurar a navegação
só lê o estado local. Atualizações ao vivo preservam rolagem e foco do teclado; trocas de aba
respeitam movimento reduzido e pausa manual. Esquerda/direita e Home/End percorrem as abas de detalhes.

O card da sessão selecionada mostra seu estado e o comando ou pergunta que ela faz, com as ações
existentes. Um turno concluído ou com falha mostra a última resposta ou erro. **Snooze**, **Keep
going** e **Dismiss** continuam numa sessão sinalizada por silêncio. Abaixo do card:

- **Session** abre por padrão: tempo em execução, custo, uso de contexto, quatro passos recentes e
  até quatro arquivos alterados. **All steps** e **See the changes** abrem Activity e Changes
  completas que foram guardadas. Zero informado aparece como zero; uma métrica ausente diz
  **Not reported** e não tem medidor.
- **Changes** guarda os últimos 32 diffs de edição, um seletor de alteração e **Focus diff**, além
  de todos os arquivos alterados acompanhados. Um arquivo com edição guardada abre aquele diff.
  Um arquivo detectado no repositório durante um turno sem edição informada diz *by a command*,
  sem inventar contagens de linhas.
- **Activity** mostra os passos recentes guardados e seus horários de início.
- **Context** lista projeto, repositório, branch, pasta, agente, editor, estado e métricas informadas.
- Com o Zeca ligado, **Zeca** abre o Chat com uma pergunta sobre aquela sessão pronta para enviar.

Tempo em execução e contexto são a última leitura informada pela CLI, hoje pela
[status line](https://code.claude.com/docs/en/statusline) do Claude Code. O tempo inclui execuções
retomadas; não é deduzido da contagem de passos ou do tempo aberto do Vults. Contexto é a ocupação
atual e pode cair após uma compactação. Custo é uma estimativa a preços de API, não uma cobrança
da assinatura. Agentes que não informam esses valores, incluindo os hooks do Codex, mostram
**Not reported**. Ler o resumo não faz requisição de rede nem muda configuração de agente. No
Full, o uso dos planos continua embaixo, âmbar a partir de 70% e vermelho a partir de 90%.

Se uma permissão ou pergunta chegar enquanto o **Focus diff** estiver expandido,
o card volta. A expansão fica indisponível enquanto a solicitação espera. Os
atalhos de permissão só respondem um card do Flock ativo e visível.

A alteração escolhida e a visualização expandida continuam ao reabrir o Nest
para a mesma sessão enquanto aquele passo estiver guardado. Outra sessão começa
na própria alteração mais recente; um passo que expira volta para a última
alteração guardada e o card. Se o conteúdo do diff estiver indisponível, o card
volta e a expansão fica indisponível.

Uma sessão que espera por você é respondida no card dela, como na ilha
([Aprovar pela ilha](approvals.md)): **Deny**, **Allow** e **Always allow** para uma permissão
(**Ctrl+Alt+Y** e **Ctrl+Alt+N** também, enquanto o Nest tem o foco), as escolhas para uma
pergunta. **Adiar por enquanto** adia esse card sem respondê-lo nem renovar seu prazo;
selecione a sessão ou **Mostrar o card** para reabri-lo. Cards adiados mantêm suas aves e
a contagem de espera, enquanto a faixa **Needs you** oculta suas tags. Uma sessão sem
card aqui (ela pergunta no terminal dela) diz isso.
**Open terminal**
aparece onde o Vults consegue trazer o terminal dela para a frente
([A ilha](island.md#getting-to-a-session)).

## Chat

O cabeçalho identifica Zeca como destinatário, separado do executor. O envio mantém a conversa e
a pasta selecionadas no clique. Trocar de conversa cancela um envio na fila; uma solicitação de
janela desatualizada é recusada antes da busca de memórias ou de um turno do modelo, e salvamentos
antigos de rascunho não substituem o rascunho da nova conversa. Os comandos locais `/remember`,
`/forget` e `/continue` também conferem Zeca como destinatário e mantêm o texto editável em caso de
recusa. Um resultado atrasado preserva a digitação mais recente. Trocar de conversa cancela o
microfone do chat e descarta transcrições atrasadas; o encaminhamento de um comando de voz não
pode assumir uma conversa vazia criada depois.

As ações **Perguntar ao Zeca** no Bando, nas Conexões e na paleta de comandos conferem o
destinatário atual antes de acrescentar uma pergunta. Elas preservam o texto do rascunho, os
anexos e a origem, incluindo texto digitado durante a preparação. Outro destinatário ou uma troca
de conversa recusa a preparação sem reiniciar uma conversa nem enviar nada. Erros de preparação
mantêm a resposta em andamento e seus cards de permissão ativos.

A conversa da ilha com o Zeca, larga, com respostas em Markdown completo (um link mostra o endereço
e nunca é seguido); uma mensagem, **New** ou outro motor em qualquer um aparece nos dois. Os cards
são respondidos aqui também, com **Deny**, **Allow** e **Always in this chat**, como na ilha.
A barra de título traz o título da conversa, a pílula do motor (como na ilha: o motor na cor dele e
o modelo; o menu dela traz os motores, os modelos e o esforço), a pasta e os controles de painel disponíveis. O
campo de texto tem um clipe: os arquivos que você escolhe vão para este chat, como soltar na ilha. O botão de painel à esquerda abre **Recent**: uma coluna numa janela larga ou no Full,
ou uma visão própria com **Back to Chat** no Room estreito. Enquanto Recent está visível, seu próprio
botão de recolher (janela larga) ou Back to Chat (janela estreita) substitui o botão da barra de título.
As conversas são agrupadas por data,
com projeto, executor e horário abaixo do título; a conversa atual fica marcada.
**Search conversations** busca localmente em todos os títulos, pastas e executores carregados,
inclusive além das primeiras 30 conversas. **Show more** revela o restante. A busca fica neste
computador ao fechar e reabrir o Nest. **+ New conversation** e a troca de conversa esperam
enquanto o Zeca responde. Se a atualização falhar, a última lista fica disponível com **Try again**.
O botão à direita abre **Memory**, o que o Zeca lembra, ao lado da conversa: o
que espera o seu sim, os arquivos e o histórico deles ([A memória do Zeca](memory.md)). Antes de
qualquer mensagem, o compositor espera no meio com algumas perguntas para começar. Sem o Zeca não há
aba Chat.

## Activity

**Histórico** usa **Buscar turnos**; **Auditoria** usa **Buscar respostas e comandos**, conforme os
registros de cada tela. **Limpar filtros** fica disponível quando existe uma busca. Pressione **Esc**
numa busca preenchida para limpá-la e manter o foco ali; a composição de texto mantém sua tecla Escape.
Histórico e Auditoria mantêm suas buscas separadamente até você sair do Vults.
Voltar para Mini ou Quick aguarda o salvamento das buscas pendentes; reabrir o Nest
restaura os filtros. **Limpar filtros** limpa apenas a busca daquela subseção.
Uma restauração atrasada preserva qualquer texto digitado depois.

O que este computador guarda, só lido aqui:

- **Week**: os números da semana dos agentes (tempo, turnos, linhas alteradas, as
  suas respostas), palavras ditadas, uma barra por dia (um clique mostra aquele dia), **Save as image…** com ou sem
  os nomes dos projetos, e a grade do ano, com os interruptores do histórico ([Atividade](activity.md)).
- **Ditado**: uso medido nos últimos 7 dias, 30 dias ou em todo o período, com barras diárias e
  aplicativos mais usados ([Estatísticas do ditado](activity.md#dictation-statistics)). O período escolhido sobrevive à reabertura.
- **By project**: o tempo de cada projeto nesta semana em barras (os worktrees de um repositório
  somam sob ele, com quantos ele tem), uma busca por projeto, branch ou agente, e um card para cada repositório em que uma sessão
  trabalha: as pastas e os branches, as sessões (um clique põe uma na frente em **Flock**), os seus
  pull requests e checks (do card do GitHub; um pull request ou os
  checks do branch padrão abrem no GitHub), o que o Zeca lembra do projeto e quanto custaram os
  agentes dele ([A memória do Zeca](memory.md#project-memory)). **Preferences ↗** abre **Settings →
  Projects**.
- **History**: cada turno terminado dos seus agentes, só contagens. Busque nos registros
  carregados por projeto, agente ou estado; **Limpar filtros** restaura a lista.
- **Cost**: quanto os agentes custaram hoje e o **orçamento diário** definido em
  **Settings → Activity**, as janelas de uso dos seus planos e o custo das últimas duas semanas,
  frente a esse orçamento, e por roost. **Configurar orçamento** abre o editor do orçamento
  ([Custo e orçamento diário](activity.md#cost-and-a-daily-budget)).
- **Traces**: o trabalho do próprio Zeca, cada turno do chat com o modelo, os tokens e o custo, e
  abaixo dele cada card de ferramenta com quanto tempo esperou por uma resposta; guardado por 30 dias.
- **Audit**: busque nos registros carregados por projeto, agente, resposta, ferramenta ou comando.
  **Limpar filtros** restaura a lista. Quem respondeu cada card e como (você, o seu telefone, uma regra Always ou uma política
  que você aceitou); guardado por 90 dias e nunca alterado.

Atividade > **By project** mantém a busca até você sair do Vults, inclusive ao
voltar para a ilha ou fechar e reabrir o Nest. **Clear filters** limpa só essa
busca; History e Audit mantêm as próprias. Se a lista de projetos ficar vazia,
uma busca ativa continua editável. Digitar filtra os projetos já carregados sem
recarregá-los nem recarregar a atividade semanal.

## Connections

Fechar e reabrir o Nest mantém o filtro de repositório, a subseção e o item selecionado
até você sair do Vults. Em uma janela estreita, o detalhe selecionado abre de novo;
Voltar retorna à lista filtrada. Se o item desaparecer, a lista abre no lugar dele.
Uma confirmação de merge ainda pendente é descartada ao fechar o Nest.

O conector escolhido tem um único cabeçalho: a última sincronização (ou por que falhou),
**↻ Refresh** para consultar de novo (no máximo uma vez por minuto) e **Manage connections** para
**Settings → Connectors**. O seletor aparece só quando vários conectores têm cards. Um conector
ligado aguardando seu primeiro card mostra a espera ou o erro com Atualizar e Configurações;
um status que não pôde ser lido oferece **Try again**, sem tratar o conector como desligado.
Quando o limite diário de requisições se esgota, o motivo da espera fica explícito.

No GitHub, quatro abas mantêm suas contagens: **Inbox** (o que espera por você: revisões pedidas
a você, seus pull requests falhando, devolvidos ou prontos para merge e um branch padrão
falhando; quantos são novos), **Pull requests**, **Actions** (os checks dos seus branches padrão;
em vermelho com quantos falham) e **Releases** (a mais recente de cada repositório). A busca
filtra a aba carregada por título, referência, repositório, branch ou autor, junto do filtro
**All repos**. Ela não faz requisições de rede. A busca fica neste computador ao reabrir;
**Clear filters** restaura a lista.

Cada linha com dados de checks mostra um pequeno resumo segmentado ao lado do estado. As falhas
vêm primeiro, seguidas pelos checks em execução e em espera. Passe o mouse para ler o nome e o
estado de cada check; a tecnologia assistiva lê as mesmas informações. Matrizes grandes mostram
até doze segmentos e a contagem restante; o detalhe selecionado continua listando todos os checks.
Linhas sem dados de checks não mostram esse resumo.

A lista e o detalhe selecionado rolam de forma independente; as atualizações normais mantêm as
posições de leitura, o foco da busca e os detalhes expandidos. Em janelas estreitas, **Voltar** ou
**Esc** retorna do detalhe para a linha selecionada. Com foco na busca, **Esc** limpa o texto;
uma confirmação de merge pendente é fechada primeiro. Selecionar, buscar ou expandir nunca executa uma ação.

O item selecionado mostra título, branch, autor e tamanho (`+120 −40 · 6 files`) antes das ações,
incluindo **Open in GitHub ↗**, como na ilha ([O card do GitHub](connectors.md#the-github-card)).
As ações e a confirmação explícita de merge vêm antes dos checks. Os checks aprovados começam
fechados; os que estão rodando ou acabaram de falhar abrem, com falhas primeiro, contagem e barra
de progresso durante a execução. A escolha manual de expandir ou fechar permanece nas
atualizações sem mudança de estado. Checks falhando têm prioridade sobre o rótulo de aprovação.
**Technical details**, quando presente, mantém informações da atualização e um comando `gh` com
**Copy** e aviso para tentar de novo em caso de falha (o Vults nunca o executa).

Uma atualização que falha mantém as linhas atuais e permite tentar de novo. **✓ Seen** deixa o
título mais discreto sem esmaecer o status ou as ações. **Add a connection** lista só conectores
suportados que estão comprovadamente desligados; **Turn on in Settings…** e **Manage connections**
abrem o único lugar que liga um conector ([Conectores](connectors.md)).

<a id="agent-profiles"></a>
## Perfis de agentes

Abra **Bando > Perfis de agentes** para consultar os perfis integrados de Desenvolvedor, Revisor e
Pesquisador. Escolha um cartão para ler suas instruções e a revisão explícita. **Criar perfil** abre
um editor próprio em branco: informe seu nome e suas instruções sem escolher ou copiar um perfil
integrado. **Criar uma cópia** continua disponível nos perfis integrados e preenche o mesmo editor
com as instruções e a aparência desse perfil. O editor destaca nome e instruções, com aparência e
skills opcionais ao lado numa janela larga e abaixo numa janela estreita. **Salvar perfil** cria um
perfil personalizado com acesso Ask por padrão; escolha outro nível de acesso depois de salvar.
**Editar perfil** salva uma nova revisão. Os perfis integrados permanecem inalterados.
Em **Aparência**, escolha uma **Espécie** e um **Acessório**; a prévia usa os mesmos sprites do bando.
Os acessórios são escolhas fixas, incluindo **Nenhum**, sem mudanças sazonais.
O cartão mostra a aparência salva após o salvamento. Em **Skills fixadas**, escolha entre as skills
integradas e consulte seu conteúdo literal e sua revisão. Cada salvamento mantém o conteúdo
selecionado nessa revisão imutável, e uma conversa futura vinculada poderá reter o mesmo snapshot.
Skills fixadas não habilitam ferramentas.

O catálogo mantém as aves numa lista compacta ao lado do perfil selecionado. A ave animada
cumprimenta ao clicar e acompanha as escolhas de aparência no editor; páginas ocultas param a
animação, e o movimento reduzido mantém a ave parada. **Abrir chat do especialista** e **Criar uma
cópia** ficam ao lado da ave selecionada, com a escolha de acesso abaixo.
Expanda **Instruções**, **Skills fixadas** ou **Origem** para consultar o conteúdo completo e o escopo
de execução. A abertura espera o acesso terminar de salvar; uma falha fica visível e permite tentar
novamente. Escolha **Abrir chat do especialista** para iniciar ou retomar esse perfil na aba **Chat** existente.
O seletor de destinatário alterna entre Zeca e os perfis, com o mesmo compositor, conversa, histórico
lateral e cartões de aprovação, preservando o rascunho e a conversa de cada destinatário. O poleiro
animado e as assinaturas das respostas mostram a espécie e o acessório fixados nessa conversa,
inclusive com o Zeca desligado. Editar o perfil afeta novas conversas; uma conversa antiga mostra
a revisão fixada e a revisão mais recente disponível. Inicie uma nova conversa para usá-la. Ela usa uma sessão isolada do Claude Code com apenas o perfil e as skills fixadas: sem
contexto de projeto ou do bando, arquivos, voz ou memória do Zeca. Continua disponível com o Zeca
desligado. O histórico e os cartões de aprovação ficam no perfil selecionado e nunca aparecem no
histórico do Chat do Zeca.

Cada perfil tem sua própria opção **Acesso deste perfil**: **Pergunta antes** é o padrão, **Suas
políticas** aplica as políticas aceitas, e **Acesso total** responde o que as políticas deixam em
aberto. As recusas das políticas bloqueiam a chamada. Comandos destrutivos, perguntas das políticas e ações
depois de ler conteúdo externo ainda mostram um cartão de aprovação. Escolha qualquer nível de acesso diretamente no seletor do Chat ou no perfil, sem abrir as configurações.
Mudanças de acesso ficam sincronizadas entre o perfil e o Chat e valem para a próxima chamada de
ferramenta depois de salvas. Uma falha mantém o acesso anterior e permite tentar novamente no Chat. Revise o comando e qualquer aviso de conteúdo externo
antes de escolher **Permitir**; **Sempre nesta conversa** repete apenas aquela chamada exata naquela conversa. O app
registra a decisão.

Os perfis personalizados ficam no banco local, independentes do histórico de conversas. Cada
salvamento mantém as revisões anteriores. Um editor desatualizado é rejeitado em vez de sobrescrever
alterações mais recentes; cancele e abra novamente para ler a última revisão. Falhas ao salvar
preservam o rascunho. Os rascunhos também sobrevivem à navegação na janela atual, mas não são
salvos ao reiniciar o app. O catálogo e o Chat do especialista funcionam com o Zeca desligado e não
acessam sua memória privada. Salvar um perfil não inicia agentes, não faz solicitações a provedores,
não escreve configurações de agentes nem altera permissões.

**Voltar ao Bando** retorna à visualização existente das suas sessões. Os indicadores de sessões
aguardando continuam disponíveis durante a leitura; abra um para revisar o cartão no Bando. Uma
falha de leitura oferece **Tentar novamente**, sem iniciar agentes nem fazer solicitações a provedores.

<a id="dictation"></a>

## Ditado

A quinta aba abre o [histórico de ditado](dictation.md#dictation-history), inclusive com o Zeca
desligado. A navegação compartilhada reúne Histórico, Comandos de voz, Dicionário e Trechos com
páginas separadas de configuração: Voz, Atalhos de teclado, Configurações dos comandos,
Vocabulário, Estilos, Colagem, Privacidade e Diagnóstico. O editor compartilhado de modelo e idioma
de transcrição fica em Voz; o Chat oferece um link. A configuração mantém sua webview privilegiada;
as transcrições ficam no Nest. Estatísticas abre Atividade > Ditado. Janelas estreitas usam um seletor
de página com rótulo; o conteúdo rola separadamente. Busca, detalhes, rascunhos e posição de leitura
sobrevivem à troca de tamanho. Excluir transcrições mantém uso medido e preferências de Suas palavras.
Veja o [processamento das suas palavras](dictation.md#your-words-processing).
