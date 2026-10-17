# O Nest
<!-- source: 1e43f6000e7d -->

O Nest é a janela do próprio Vults, para quando a ilha fica pequena: o bando inteiro, o
chat, o que aconteceu e as suas conexões. Abra pelo ícone da bandeja (**Nest…**) ou pela
seta no topo do chat da ilha. Ele é criado quando você abre e some quando você fecha, e abre também
com o Zeca desligado, só que sem o Chat. Enquanto ele está aberto a ilha sai, com pílula e aves: uma
superfície na tela por vez. O Nest não tem a barra de título do sistema: o cabeçalho dele o move (um
clique duplo ocupa a tela) e as bordas e os cantos o redimensionam. Ele abre grande da primeira vez
(até 1180 × 820, dentro da tela), e depois no tamanho em que você o deixou. Ela volta quando o Nest fecha ou volta para **Quick** ou **Mini**. Um
card que espera é respondido onde você está: no Nest enquanto você está nele (a janela dele tem o
foco), e na ilha, que aparece para ele, enquanto você está em outro lugar.

Atividade e Conexões usam o mesmo fundo preto, cards elevados, escala de títulos e controles
neutros. Conexões usa a navegação segmentada de Atividade. As cores de status mantêm seus
significados.

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

O cabeçalho conta o bando (*4 sessions · 1 needs you*) e traz a música que o seu player está tocando
(com o Now playing ligado; anterior, tocar/pausar e próxima ao lado), a lua enquanto o não perturbe
dura (um clique o encerra), o som, a busca (a paleta, como **Ctrl+K**), o seletor de tamanho e o
botão de tela cheia. Embaixo dele, quatro abas com os seus
ícones, o número que precisa de você em **Flock** e as linhas dos conectores em **Connections**, e
**Settings** à direita, que mostra as Configurações nesta mesma janela, sob as mesmas abas; uma aba
traz o Nest de volta. Sobre todas as abas menos **Flock**, uma etiqueta para cada sessão que precisa
de você (ou que falhou); um clique abre **Flock** com ela na frente (um card que espera vem
primeiro ali), onde você o responde. As Configurações continuam uma página própria ali: são a única que muda a
config de um agente, uma chave, uma política ou um celular, e nunca mostram texto de agente. O
seletor mantém a aba em que você está: **Room** é a janela, o botão de tela cheia a tela toda
(**Esc** volta para Room), **Quick** fecha o Nest e abre a ilha, **Mini** o fecha e deixa a pílula.
**Alt+1** a **Alt+4** escolhem uma aba. Embaixo, **Zeca on · optional** / **Zeca off** liga e
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

## Flock

O bando de ponta a ponta da janela, três vezes maior, cada ave sobre um card com o projeto, o estado
e o agente da sessão, e ao lado do título quantas estão fora numa saída pela web e quantas estão no
fio. Um clique numa ave ou num card (ou **↑ ↓**) põe aquela sessão na frente. Um clique direito (ou
a tecla de menu, ou **Shift+F10**) abre as ações rápidas dela, como na ilha: **Open terminal**, **Open folder**,
**Activity**, **View the last diff**, **Keep
in front** (ou **Let the flock choose**), e silenciar, fixar ou esconder o projeto dela. Enquanto
um card espera, só a **Activity** e o diff daquela sessão podem aparecer.
Embaixo, **Session details**: **Needs you** deixa só as sessões que esperam por você, e **Find a
session…** lê projetos, repositórios, branches, agentes e o que cada uma faz agora.

A ave selecionada e seu card compartilham uma marca clara de seleção. Nomes de
projetos ocupam até duas linhas; passar o mouse num card mostra nome completo,
branch, agente e estado. **Needs you** inclui o total de sessões aguardando no bando,
mesmo durante uma busca. O card de detalhes separa o título do projeto de agente e
estado, e as abas usam uma entrada breve só quando você as troca; atualizações ao
vivo não a repetem. Movimento reduzido e pausa manual pulam essa entrada.

A busca, o filtro **Needs you** e a aba de detalhes continuam onde você deixou
ao fechar e reabrir o Nest ou voltar para a ilha, até sair do Vults. **Clear
filters** na barra da busca limpa a busca e o Needs you, mantendo a aba de detalhes. Com o Zeca
desligado, a aba de detalhes dele salva abre **Changes**. Filtrar a lista de
sessões mantém o card de permissão atual visível; restaurar esses controles
nunca responde esse card.

A sessão da frente tem o card dela à esquerda: o estado, o que está fazendo e, numa que espera, o
que ela pede (o comando, com **Copy**, e *1 of 2* quando outras esperam). Uma que trabalha mostra no
topo os passos até agora, os subagentes e o custo, o passo que acabou e o que está em andamento (o
**+N −M** de uma edição terminada a abre em **Changes**) e, quando ela fica quieta, **Snooze**,
**Keep going** e **Dismiss**, como na ilha. Uma que terminou ou falhou mostra a última resposta ou o
erro. Embaixo do card, quantos arquivos a sessão alterou e as linhas adicionadas e removidas. À
direita, as **Changes** (o diff de cada uma das suas últimas 32 edições, escolhida na lista; **Focus
diff** dá ao diff a largura toda; embaixo, todos os arquivos que a sessão alterou, e um clique num
cujo diff ainda está guardado o mostra; num repositório, um arquivo que um comando escreveu durante
um turno aparece como *by a command*), a **Activity** (os últimos passos, cada um na hora em que começou), o **Context** (worktree, repositório, branch, pasta,
agente, editor, estado e custo) e, com o Zeca ligado, **Zeca**: **Ask Zeca** abre o Chat com a
pergunta pronta. No Full, o uso dos seus planos fica embaixo, âmbar a partir de 70% e vermelho a
partir de 90%.

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
pergunta. Uma sem card aqui (ela pergunta no terminal dela) diz isso.
**Open terminal**
aparece onde o Vults consegue trazer o terminal dela para a frente
([A ilha](island.md#getting-to-a-session)).

## Chat

A conversa da ilha com o Zeca, larga, com respostas em Markdown completo (um link mostra o endereço
e nunca é seguido); uma mensagem, **New** ou outro motor em qualquer um aparece nos dois. Os cards
são respondidos aqui também, com **Deny**, **Allow** e **Always in this chat**, como na ilha.
A barra de título traz o título da conversa, a pílula do motor (como na ilha: o motor na cor dele e
o modelo; o menu dela traz os motores, os modelos e o esforço), a pasta e dois botões de painel. O
campo de texto tem um clipe: os arquivos que você escolhe vão para este chat, como soltar na ilha. O botão de painel à esquerda abre **Recent**: uma coluna numa janela larga ou no Full,
ou uma visão própria com **Back to Chat** no Room estreito. As conversas são agrupadas por data,
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

Histórico e Auditoria mantêm suas buscas separadamente até você sair do Vults.
Voltar para Mini ou Quick aguarda o salvamento das buscas pendentes; reabrir o Nest
restaura os filtros. **Limpar filtros** limpa apenas a busca daquela subseção.
Uma restauração atrasada preserva qualquer texto digitado depois.

O que este computador guarda, só lido aqui:

- **Week**: os quatro números da semana (tempo com os seus agentes, turnos, linhas alteradas, as
  suas respostas), uma barra por dia (um clique mostra aquele dia), **Save as image…** com ou sem
  os nomes dos projetos, e a grade do ano, com os interruptores do histórico ([Atividade](activity.md)).
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
