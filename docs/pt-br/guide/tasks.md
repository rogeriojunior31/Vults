# Tarefas
<!-- source: 552f0b794170 -->

Uma tarefa é um objetivo com os critérios que dizem que ela está pronta, guardada neste computador,
que o Vults pode passar para um agente trabalhando num clone só dele. A aba **Tasks** do Nest as lista
([O Nest](nest.md#tasks)).

<a id="keeping-a-task"></a>

## Guardando uma tarefa

`/task` no chat guarda uma tarefa numa mensagem só, mandada para nenhum modelo: o objetivo na
primeira linha, e depois uma linha para cada critério que diz que ela está pronta, começando com `-`:

```
/task Fix the flaky login test
- passes 20 runs in a row
- no sleep added
```

Numa linha só, `done when:` começa os critérios, separados por `;`. O roost dela é o repositório da
pasta da conversa. A aba **Tasks** do Nest lista as tarefas, as que faltam primeiro, com cada
critério para marcar, quem trabalha nela (você, o Zeca ou um agente), em que pé está (*To do*,
*Doing*, *Blocked*, *Done*, *Dropped*) e quanto custou depois que o Vults começar sessões para ela.
**New task** cria uma lá também. Com o conector do GitHub ligado, uma tarefa num repositório do
GitHub ganha **Open an issue**: no seu clique, o seu `gh` abre uma issue com o objetivo e os
critérios, e a tarefa fica ligada a ela. Qualquer tarefa pode se ligar a uma issue que você já tem:
`#12` (no repositório do GitHub dela) ou o endereço da issue, e depois **Link**. As tarefas ficam
no `vults.sqlite`, neste computador.

O campo da issue mantém o texto, o foco e a seleção quando a lista atualiza.
**Vincular** limpa o rascunho apenas depois que o salvamento tem sucesso; um erro
deixa o texto pronto para tentar de novo. Vincular, Abrir uma issue e Desvincular
aguardam um único pedido de issue por tarefa, inclusive ao atualizar a lista, para
que cliques repetidos não enviem pedidos concorrentes. Uma issue vazia mantém
Vincular desabilitado. O backend valida números e endereços de issues.

**New task** mantém o objetivo, os critérios e o projeto até **Add** ter sucesso. Ao
adicionar, o formulário aguarda sem aceitar um pedido duplicado. Uma falha mostra
o erro e deixa o rascunho pronto para tentar novamente. Atualizações da lista mantêm
o campo focado e a seleção do texto; o projeto escolhido continua disponível mesmo
se sua sessão sair do bando. Um objetivo vazio mantém Add desabilitado.

Se a lista não puder atualizar, Tarefas mantém os últimos cards carregados, os rascunhos
e as seções abertas, com um aviso de que a lista pode estar desatualizada. **Tentar de novo**
recarrega sem iniciar trabalho nem enviar o rascunho. Uma falha no primeiro carregamento
mostra um erro em vez de afirmar que não há tarefas. Erros de ações continuam visíveis até
outra ação ter sucesso ou você abrir Nova tarefa.

Cada card destaca objetivo e estado, seguidos de uma barra de progresso dos critérios e do
próximo critério pendente. **Em execução** descreve a execução do agente separadamente do
ciclo da tarefa: terminar um turno não marca a tarefa como concluída. Iniciar,
Continuar ou Parar permanecem visíveis; projeto e custo ficam como contexto de apoio.
Atualizações da lista e edições salvas mantêm o foco do teclado no estado,
responsável ou critério daquela tarefa. Se o controle focado desaparecer, o foco
volta para Mais ações, ou Nova tarefa quando o card sair da lista. Atualizações
não tiram o foco de outra área.
**Mais ações** reúne mudanças de estado/responsável, links de issues, novas sessões,
handoffs, revisões e remoção. A seção continua aberta quando a lista atualiza. Excluir a tarefa e
remover a pasta são ações separadas, cada uma exigindo um segundo clique.
**Cancelar exclusão** e **Cancelar remoção da pasta** cancelam apenas seu próprio pedido.
**Esc** dentro de Mais ações cancela as confirmações pendentes da tarefa atual;
o foco volta para sua ação, ou para Mais ações quando ela estiver indisponível.
Atualizações mantêm o foco no botão de confirmação ou
cancelamento, e segurar Enter não vale como os dois cliques de confirmação. Uma tarefa
rodando mantém Excluir desabilitado até você pará-la.

Iniciar ou continuar envia um turno ao agente, que pode chamar ferramentas várias
vezes nesse turno. Quando ele retornar, revise **What it said last** e use **Go on**
se ainda faltar trabalho. Um turno terminado não marca a tarefa como Done nem seus critérios.
O card explica essa diferença enquanto a tarefa continuar aberta.

Se uma execução ou revisão falhar, a tarefa fica **Blocked** e **What it said last**
abre com a resposta parcial e a causa da falha. Se você fechar a resposta, ela continua
fechada ao atualizar; uma nova falha a abre novamente. O urubu mantém o estado **Failed**,
e todos os cards sem resposta da execução encerrada são devolvidos sem aprovação.

Tarefas Claude e Codex perguntam pelo canal de permissões do executor. Seus hooks do Vults
observam o trabalho sem perguntar pela mesma permissão novamente. Passos paralelos e
eventos de subagentes mantêm os cards sem resposta visíveis. Quando o CLI da tarefa encerra,
seu urubu terminado fica no bando pelos dez minutos normais de retenção, em vez de sair
imediatamente. Sessões do terminal mantêm o comportamento normal de SessionEnd.

<a id="delegating"></a>

## Delegando

`/delegate` cria uma tarefa do mesmo jeito e a começa na hora no agente que o roteador escolhe, e
o card da tarefa diz por quê. A regra do próprio projeto escolhe primeiro (`/delegate default
codex`, ou `claude`, digitado numa conversa nesse projeto; `/delegate default none` a apaga); sem
ela, o Claude Code. Se o plano desse agente passa de 85% e o do outro está, que se saiba, abaixo
disso, a tarefa vai para o outro. Quando um projeto tem 10 tarefas terminadas de um tipo (uma
correção, testes, docs, uma funcionalidade), o agente que mais terminou delas ali substitui a regra
para esse tipo, mas os 85% continuam valendo. Uma tarefa conta para o agente dono dela quando é
marcada como feita ou abandonada.

<a id="starting-an-agent-on-a-task"></a>

## Começando um agente numa tarefa

**Start with Claude Code** trabalha numa tarefa que está num repositório. O Vults faz para a tarefa
um clone próprio, num branch só dela (`vults/task-12`), dentro da pasta de dados do Vults: um clone
`--shared`, que lê os objetos do seu repositório mas nunca os escreve. Ele começa o Claude Code ali
com o objetivo e os critérios da tarefa, orientado a fazer commit nesse branch e nunca dar push. Ele
roda no sandbox do Claude Code (bubblewrap): não começa sem ele, todo comando continua perguntando,
e ele só escreve no clone dele e nas pastas temporárias, nunca no seu repositório, nos branches, nos
hooks ou na config dele. Comandos que você tirou do sandbox nas suas próprias configurações do Claude
Code continuam rodando fora dele, cada um perguntando antes. Ele lê as suas próprias configurações,
então aparece no bando como qualquer sessão e as permissões dele são cards da ilha, respondidos por
você, pelas suas regras ou pelas suas políticas. Duas tarefas rodam lado a lado sem mexer nos
arquivos uma da outra. Quando o turno acaba, o Vults traz o branch da tarefa, e só ele, para o seu
repositório, sem rodar nada do clone; **What it said last** embaixo da tarefa guarda o fim da
resposta, e **Go on** continua a mesma sessão. **Start fresh** passa a tarefa para uma sessão nova,
quando a antiga já cresceu demais: a sessão antiga escreve um pacote de contexto (Objective,
Constraints, Done, In progress, Decisions, Files, Failures, Next), o Vults põe o que sabe com certeza
(o objetivo, quais critérios estão marcados, os arquivos mudados no branch), e a sessão nova começa
só desse pacote. O pacote fica embaixo da tarefa como **Handoff notes**; se a sessão antiga não
escreve nenhum, nada é passado adiante e ela mantém a sessão. **Stop** pede para o turno parar,
espera e então o encerra; só uma sessão que o Vults começou pode ser parada, e uma tarefa rodando não
pode ser apagada. O que ela custou entra na tarefa. **Remove its folder** traz o branch de volta e
remove o clone; trabalho que nunca teve commit nele vai junto, então pergunta duas vezes.

### Codex

**Start with Codex** faz o mesmo com o Codex, no sandbox dele (`workspace-write`: o clone dele e as
pastas temporárias). O Codex pede aprovações pelo próprio canal: o Vults transforma cada uma no card
da ilha que o hook dele mandaria (o comando inteiro, ou os arquivos e a contagem de linhas de uma
edição; uma chamada que ele não consegue nomear inteira é um não), então as suas regras, políticas e
o telefone respondem do mesmo jeito, e uma execução parada tira o card que esperava. Numa tarefa com
sessão, **Hand off to Codex** (ou **Hand off to Claude Code**) é o **Start fresh** no outro agente: a
sessão escreve o pacote, e o outro começa dele.

<a id="other-agents"></a>

### Outros agentes

**Start with another agent…** lista os agentes instalados aqui que falam o [Agent Client
Protocol](https://agentclientprotocol.com): OpenCode, Qwen Code, Copilot CLI e Kimi Code.
Escolha um e clique em **Começar com** ao lado do seletor para executar a tarefa. Mudar
a opção, inclusive com as setas do teclado, não envia nada. A escolha fica com aquela tarefa
quando a lista atualiza. Se o agente escolhido não estiver mais disponível, a opção
é limpa e Começar aguarda outra seleção; outro agente nunca é usado em seu lugar.
O agente trabalha no clone da tarefa do mesmo jeito, logado com a própria conta. O Vults não oferece arquivos
nem terminal próprios: ele usa as ferramentas dele e pede antes de cada uma que escreve, roda ou
busca algo, e cada pedido é um card da ilha, respondido só daquela vez, nunca com o "sempre" do
próprio agente. Uma mudança num arquivo fora do clone, ou no `.git` dele, um comando rodado em outro
lugar, uma movimentação ou uma exclusão é um não sem card, assim como um arquivo alcançado por um
link que aponta para fora do clone. O OpenCode roda toda ferramenta sem perguntar por padrão, e o
Qwen Code e o Kimi Code podem rodar, pelas suas próprias configurações: o Vults inicia cada um com
uma configuração, só para aquele processo ou sessão, que o faz perguntar (nunca escrita na config dele), e uma
escrita ou um comando que ainda assim roda sem perguntar para o turno. Os hooks deles não perguntam
nada nessas sessões, então uma chamada nunca mostra dois cards. Stop, Go on, Start fresh e Review
funcionam como com o Codex; a revisão é do Codex. Testado com o OpenCode, e com o Kimi Code
2.1.1 contra um modelo de mentira; o Qwen Code e o Copilot CLI seguem o mesmo protocolo, mas ainda
não foram testados com uma conta real.

<a id="review"></a>

## Revisão

**Review with Codex** (ou **with Claude Code**) faz o outro agente revisar o trabalho de uma tarefa,
no clone dela, como um revisor que não escreveu nada daquilo, num sandbox em que não escreve nada:
ele roda os testes, lê o diff contra onde o branch começou, confere cada critério e lista o que
está errado. O Codex usa o revisor
próprio dele. Quando a revisão pede mudanças, os achados vão para a sessão da tarefa, que os corrige
e faz commit, e a rodada seguinte revisa de novo, até três rodadas. Uma aprovação encerra; uma
revisão sem veredito, ou uma terceira rodada que ainda pede mudanças, fica para você ler. A última
revisão fica embaixo da tarefa, e quem marca a tarefa como feita continua sendo você.

<a id="cost-and-a-daily-budget"></a>

## Custo e um orçamento diário

O Vults conta o que os agentes dizem que custaram: cada turno de uma sessão que ele iniciou para uma
tarefa ou um `/continue` (os dólares do Claude Code; o Codex informa só tokens, então as tarefas dele
mostram os tokens) e cada sessão do Claude Code que ele acompanha, pela status line, contada a partir
da primeira vez que o Vults a vê (o que ela custou antes fica fora do dia). Nada é estimado por
tabela de preços. O chat do próprio Zeca, os resumos e o sono não entram nessa conta, e o orçamento
não os para. Numa
assinatura, os dólares são o que os tokens custariam a preços de API, que o seu plano não cobra; o
Vults diz isso quando consegue saber (um plano que informa as janelas de uso). Cada tarefa mostra
custo e tokens, cada roost em **Activity → By project** do Nest o custo de hoje e o total, e o topo da aba **Tasks** o
de hoje.

**Activity → Cost**, no Nest, define um **orçamento diário** em dólares. Em 80% uma notificação avisa; quando ele
acaba, outra avisa, e o Vults não inicia nenhuma sessão nova até o dia seguinte: nada de Start,
handoff, review, `/delegate` ou `/continue`. É só isso. Ele nunca para uma sessão que está rodando, e
nunca bloqueia nem atrasa um agente que você mesmo iniciou. Os custos ficam em `vults.sqlite`, neste
computador; o **Clear history** não os apaga.
