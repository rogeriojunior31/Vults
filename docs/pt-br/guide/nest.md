# O Nest
<!-- source: 85bb9c7cbae4 -->

O Nest é a janela do próprio Vults, para quando a ilha fica pequena: o bando inteiro, as suas
tarefas, o chat, o que aconteceu e as suas conexões. Abra pelo ícone da bandeja (**Nest…**) ou pela
seta no topo do chat da ilha. Ele é criado quando você abre e some quando você fecha, e abre também
com o Zeca desligado, só que sem o Chat. Enquanto ele está aberto a ilha sai, com pílula e aves: uma
superfície na tela por vez. O Nest não tem a barra de título do sistema: o cabeçalho dele o move (um
clique duplo ocupa a tela) e as bordas e os cantos o redimensionam. Ele abre grande da primeira vez
(até 1180 × 820, dentro da tela), e depois no tamanho em que você o deixou. Ela volta quando o Nest fecha ou volta para **Quick** ou **Mini**. Um
card que espera é respondido onde você está: no Nest enquanto você está nele (a janela dele tem o
foco), e na ilha, que aparece para ele, enquanto você está em outro lugar.

Minimizar o Nest devolve a ilha e seus atalhos de aprovação. Abrir o Nest de novo
restaura a janela; minimizar não altera o tamanho salvo da Sala.

Reabrir pela bandeja mantém a última aba e subseção de Atividade até você sair do Vults.
Abrir uma tela específica tem prioridade. Com Zeca desligado, uma aba Chat salva abre o Flock.

Fechar pelo comando do desktop (como **Alt+F4**) também espera o salvamento do
rascunho do Chat, da navegação e das buscas nos registros. Se salvar ou fechar
falhar, a janela continua aberta com um aviso; tente fechar de novo. Isso também
salva o estado do Nest quando as Configurações estão visíveis na janela compartilhada.
Fechar não responde uma permissão pendente nem interrompe uma execução.

Em uma janela baixa do Chat, o compositor ocupa menos espaço e as sugestões da
conversa vazia rolam quando necessário. Isso também vale para janelas largas,
mantendo os controles do cabeçalho e o rodapé acessíveis.

O cabeçalho conta o bando (*4 sessions · 1 needs you*) e traz a música que o seu player está tocando
(com o Now playing ligado; anterior, tocar/pausar e próxima ao lado), a lua enquanto o não perturbe
dura (um clique o encerra), o som, a busca (a paleta, como **Ctrl+K**), o seletor de tamanho e o
botão de tela cheia. Embaixo dele, cinco abas com os seus
ícones, o número que precisa de você em **Flock** e as linhas dos conectores em **Connections**, e
**Settings** à direita, que mostra as Configurações nesta mesma janela, sob as mesmas abas; uma aba
traz o Nest de volta. Sobre todas as abas menos **Flock**, uma etiqueta para cada sessão que precisa
de você (ou que falhou); um clique abre **Flock** com ela na frente (um card que espera vem
primeiro ali), onde você o responde. As Configurações continuam uma página própria ali: são a única que muda a
config de um agente, uma chave, uma política ou um celular, e nunca mostram texto de agente. O
seletor mantém a aba em que você está: **Room** é a janela, o botão de tela cheia a tela toda
(**Esc** volta para Room), **Quick** fecha o Nest e abre a ilha, **Mini** o fecha e deixa a pílula.
**Alt+1** a **Alt+5** escolhem uma aba. Embaixo, **Zeca on · optional** / **Zeca off** liga e
desliga o Zeca (como **Settings → Flock & Zeca → Zeca**), **Zeca's look** abre a página dele nas
Configurações, **Open terminal** traz o terminal da sessão da frente e **Back to the island** traz a
ilha.

**Ctrl+K**, no Nest ou na ilha aberta (quando ela tem o teclado: aberta com Ctrl+Alt+Space, ou no
chat dela), abre a paleta: digite para pular para uma sessão, para
começar `/task`, `/delegate`, `/continue` ou `/remember` no chat (com o Zeca ligado), ou para abrir uma
aba. Ela nunca responde um card: uma sessão que espera por você só leva até o card dela.

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

A sessão da frente tem o card dela à esquerda: o estado, o que está fazendo e, numa que espera, o
que ela pede (o comando, com **Copy**, e *1 of 2* quando outras esperam). Uma que trabalha mostra no
topo os passos até agora, os subagentes e o custo, o passo que acabou e o que está em andamento (o
**+N −M** de uma edição terminada a abre em **Changes**) e, quando ela fica quieta, **Snooze**,
**Keep going** e **Dismiss**, como na ilha. Uma que terminou ou falhou mostra a última resposta ou o
erro. Embaixo do card, o que as edições guardadas mudaram (dos últimos passos dela). À direita, as
**Changes** (o diff de cada edição terminada, escolhida na lista; **Focus diff** dá ao diff a largura
toda), a **Activity** (os últimos passos, cada um na hora em que começou), o **Context** (worktree, repositório, branch, pasta,
agente, editor, estado e custo) e, com o Zeca ligado, **Zeca**: **Ask Zeca** abre o Chat com a
pergunta pronta. No Full, o uso dos seus planos fica embaixo, âmbar a partir de 70% e vermelho a
partir de 90%.

Uma sessão que espera por você é respondida no card dela, como na ilha
([Aprovar pela ilha](approvals.md)): **Deny**, **Allow** e **Always allow** para uma permissão
(**Ctrl+Alt+Y** e **Ctrl+Alt+N** também, enquanto o Nest tem o foco), as escolhas para uma
pergunta. Uma sem card aqui (ela pergunta no terminal dela) diz isso.
**Open terminal**
aparece onde o Vults consegue trazer o terminal dela para a frente
([A ilha](island.md#getting-to-a-session)).

## Tasks

As suas [tarefas](tasks.md), sob o roost de cada uma (as sem roost por último): cada uma com o
objetivo, os critérios, o dono, o status e o custo, por que o roteador escolheu o agente dela, as
rodadas de revisão como marcas (até três, a última na cor do veredito), os botões que a iniciam,
revisam ou passam adiante, e o gasto de hoje frente ao seu orçamento diário (definido em Activity →
Cost). **New task** guarda uma
sem o chat, que é como você cria uma com o Zeca desligado.

## Chat

A conversa da ilha com o Zeca, larga, com respostas em Markdown completo (um link mostra o endereço
e nunca é seguido); uma mensagem, **New** ou outro motor em qualquer um aparece nos dois. Os cards
são respondidos aqui também, com **Deny**, **Allow** e **Always in this chat**, como na ilha.
A barra de título traz o título da conversa, a pílula do motor (como na ilha: o motor na cor dele e
o modelo; o menu dela traz os motores, os modelos e o esforço), a pasta e dois botões de painel. O
campo de texto tem um clipe: os arquivos que você escolhe vão para este chat, como soltar na ilha. Numa janela larga ou no Full, o da esquerda mostra **Recent**: as suas 30 conversas mais
novas, a atual marcada, e **+ New conversation** (os dois esperam enquanto o Zeca responde); um
clique continua uma delas. O da direita abre **Memory**, o que o Zeca lembra, ao lado da conversa: o
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
  checks do branch padrão abrem no GitHub), o que o Zeca lembra do projeto e quanto custaram as
  tarefas dele ([A memória do Zeca](memory.md#project-memory)). **Preferences ↗** abre **Settings →
  Projects**.
- **History**: cada turno terminado dos seus agentes, só contagens. Busque nos registros
  carregados por projeto, agente ou estado; **Limpar filtros** restaura a lista.
- **Cost**: quanto os agentes custaram hoje e o **orçamento diário** que você define aqui, as janelas
  de uso dos seus planos e o que os agentes custaram nas últimas duas semanas, frente a esse
  orçamento, e por roost ([Custo e orçamento diário](tasks.md#cost-and-a-daily-budget)).
- **Traces**: o trabalho do próprio Zeca, cada turno do chat com o modelo, os tokens e o custo, e
  abaixo dele cada card de ferramenta com quanto tempo esperou por uma resposta; guardado por 30 dias.
- **Audit**: busque nos registros carregados por projeto, agente, resposta, ferramenta ou comando.
  **Limpar filtros** restaura a lista. Quem respondeu cada card e como (você, o seu telefone, uma regra Always ou uma política
  que você aceitou); guardado por 90 dias e nunca alterado.

## Connections

Fechar e reabrir o Nest mantém o filtro de repositório, a subseção e o item selecionado
até você sair do Vults. Em uma janela estreita, o detalhe selecionado abre de novo;
Voltar retorna à lista filtrada. Se o item desaparecer, a lista abre no lugar dele.
Uma confirmação de merge ainda pendente é descartada ao fechar o Nest.

Um chip para cada conector que você ligou, com quantas linhas ele tem; **+** abre **Settings →
Connectors**. Embaixo, o conector escolhido: quando sincronizou pela última vez (ou por que a última
consulta falhou), um filtro de repositório (**All repos** ou um deles) e **↻ Refresh** para consultar
de novo (no máximo uma vez por minuto). No GitHub, quatro abas com a contagem: **Inbox** (o que
espera por você: revisões pedidas a você, seus pull requests falhando, devolvidos ou prontos para
merge, um branch padrão falhando; quantos são novos), **Pull requests**, **Actions** (os checks dos
seus branches padrão; em vermelho com quantos falham) e **Releases** (a mais recente de cada
repositório). Uma linha escolhida aparece ao lado: o repositório, o branch, o tamanho
(`+120 −40 · 6 files`), quem revisou ou foi chamado, quando mudou pela última vez, cada check um a um
(uma barra enquanto rodam), um comando `gh` para olhar mais de perto com **Copy** (o Vults nunca o
roda), **Open in GitHub ↗** e as ações da linha, como na ilha ([O card do
GitHub](connectors.md#the-github-card)); uma linha marcada com **✓ Seen** fica esmaecida aqui. **Add a connection** lista cada conector
do Vults que está desligado, com o que ele observa; **Turn on in Settings…** e **Manage
connections** abrem **Settings → Connectors**, o único lugar que liga um conector
([Conectores](connectors.md)).
