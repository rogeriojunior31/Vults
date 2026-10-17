# Atividade
<!-- source: 37a7c01b6c59 -->

Week compara os números com a semana anterior do calendário, mostra os turnos registrados por
agente e identifica os meses e dias da semana na grade anual. Uma base de comparação ausente ou
zerada é informada explicitamente. Um histórico ilegível gera erro com nova tentativa, nunca uma
tela vazia. **History settings** abre Configurações; retenção e limpeza do histórico ficam lá.
A imagem do resumo tem sua própria opção de exportação **Hide project names**.

**Activity → Week** do Nest mostra o que os seus agentes fizeram: o resumo de uma semana e o último
ano numa grade. **Settings → Activity** mantém os controles de histórico e de orçamento diário
e um botão
**Open Activity**; o resumo e o calendário ficam no Nest. Tudo é contado neste
computador, a partir de um histórico local que o Vults guarda enquanto os seus agentes trabalham.

O Nest organiza Atividade em seis telas, com o mesmo estilo de navegação e resumo:

- **Week**: totais, tempo por dia, a grade anual e um resumo para salvar. Selecione um dia para
  ler a duração exata; **Como a atividade é contada** explica as regras de contagem.
- **By project**: tempo por repositório, worktrees, sessões, pull requests e memória do projeto.
- **History**: turnos concluídos agrupados por projeto, com resultado, agente, contagens e duração.
- **Cost**: orçamento de hoje, janelas dos planos, custo e tokens das últimas duas semanas e custo por projeto.
- **Traces**: turnos do chat do Zeca e suas chamadas de ferramentas, com estado, tempo e contagens de tokens disponíveis.
- **Audit**: quem respondeu, o resultado e o comando ou alvo completo, agrupados por projeto.

Histórico e Auditoria mostram os 200 registros mais recentes. Seus resumos descrevem os registros
carregados naquela tela, e a busca filtra a lista sem alterar esses totais. Os títulos dos projetos
mostram a quantidade de registros encontrados. Navegar, buscar e redimensionar não modificam o histórico guardado.

A prévia compacta de **Week** na ilha usa o mesmo estilo de resumo: a semana e suas datas no topo,
quatro totais e o tempo por projeto (até quatro projetos), ou um pequeno gráfico diário quando
nenhum projeto tem nome. As setas alternam entre a semana anterior e a atual; **Open in the Nest**
abre a tela completa de Atividade. As horas no resumo do topo são arredondadas; passe o mouse no
valor para ver a duração completa. Uma semana ainda não lida mostra uma mensagem de carregamento
ou erro em vez de totais zerados.

<a id="cost-and-a-daily-budget"></a>

## Custo e um orçamento diário

**Activity → Cost** mostra o gasto registrado de hoje, as janelas dos planos e os custos por dia
e projeto. Custos de assinatura aparecem a preços de API; não são uma cobrança adicional do plano.
Medições ausentes continuam desconhecidas. O histórico de gastos existente continua legível.

**Configurar orçamento** abre **Settings → Activity**, o único lugar para editar o orçamento diário.
Digite dólares, de $0,01 a $10.000, ou deixe vazio para não ter orçamento. Salvar desabilita os
controles até a escrita terminar; uma falha mantém seu rascunho para outra tentativa.
Em 80% o Vults avisa; quando acaba, avisa novamente e não inicia sessões novas até amanhã,
incluindo `/continue`. Nunca para uma sessão em andamento nem bloqueia um agente que você iniciou.
O Chat do Zeca, os resumos e o sono têm limites próprios e não entram neste orçamento.

<a id="what-a-turn-is"></a>

## O que é um turno

Um turno é o que um prompt pôs para andar, até o agente parar. Para cada um o Vults guarda quando
ele rodou, o agente, o nome da pasta do projeto e contagens (passos, comandos, linhas mudadas, as
suas respostas). Nunca um prompt, uma resposta, um comando, um nome de arquivo ou um caminho, e nada
sai deste computador ([Segurança](../safety.md)).

<a id="the-week"></a>

## A semana

O card mostra uma semana, de segunda a domingo, pelos seus dias locais: esta semana primeiro, e
**‹ ›** para passar pelas semanas que o histórico guarda (12).

- **Tempo com os seus agentes**: turnos rodando lado a lado contam uma vez só.
- **Turnos** e os passos deles, **linhas** adicionadas e removidas e os arquivos mexidos,
  **comandos**, **respostas** (permitidas, negadas, perguntas respondidas), e **turnos que
  falharam**.
- O tempo por dia em barras, o agente e o projeto com mais turnos, o dia mais movimentado e o turno
  mais longo.

<a id="sharing-the-week"></a>

## Compartilhando a semana

**Save as image…** embaixo da semana a desenha como uma imagem (1080 × 1350), com o Zeca no fio
dele. **Hide project names** tira o projeto dela. O diálogo *Salvar como* do desktop pergunta onde
(no Windows ela vai para a sua pasta Imagens), e nada é enviado.

<a id="on-monday-morning"></a>

## Na segunda de manhã

A partir das 08:00 de segunda, a ilha conta a semana passada numa linha (*Last week: 41 turns, 6 h
20 min with your agents, most on site.*), uma vez, se os seus agentes tiveram algum turno naquela
semana. Ela aparece em cada preset como a linha que vem depois de uma tela bloqueada
([A ilha](island.md#while-you-were-away)), nunca no *Paused*. **Open Activity** mostra a semana
inteira no Nest; ele e o **×** a marcam como lida.

<a id="the-grid"></a>

## A grade

O último ano, uma coluna por semana e uma linha por dia, com a segunda em cima. O tom de um dia
segue o tempo com os seus agentes, em quatro degraus pelos quartis dos seus dias ativos. Passe o
mouse num dia para ver os turnos e o tempo.

A aba **GitHub** mostra o seu calendário de contribuições do GitHub, com os níveis do próprio GitHub
e as semanas começando no domingo, como no seu perfil. Ela precisa do conector GitHub ligado
(**Settings → Connectors**) e é pedida pelo `gh` uma vez quando a aba abre, depois guardada por uma
hora na memória, nunca em disco.

<a id="the-history"></a>

## O histórico

- **Keep a history** vem ligado. Desligado, nada novo é guardado e o que existe fica.
- **Clear history…** pergunta uma vez, depois apaga os turnos e dias, o log de auditoria das suas
  respostas, o trace local de pedidos (os turnos do Zeca e cada pedido feito em seu nome), as
  conversas salvas do Zeca, o diário dele, os resumos dos dias e os dados de lembrança deles, as
  propostas de memória, as execuções do sono e o retrato do bando guardado para o servidor MCP do
  Zeca, tudo de uma vez, deixando uma linha no log de
  auditoria: que você o limpou, e quando. O que fica: o que o Zeca lembra (os arquivos de memória
  dele, **Settings → Chat & voice → Memory**), as políticas que você aceitou, os seus telefones
  pareados e os custos registrados dos agentes ([Segurança](../safety.md)). As tabelas de recursos
  removidos ficam intactas; os custos antigos continuam disponíveis em **Cost**.
- O histórico fica num banco local, `vults.sqlite` (cada turno por 12 semanas, os totais de cada
  dia por um ano), em `~/.local/share/vults/` (`%LOCALAPPDATA%\Vults\` no Windows). Veja
  [Configurações e arquivos](../reference/settings.md).
