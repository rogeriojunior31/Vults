# Atividade
<!-- source: 92d605d05b2b -->

**Settings → Activity** mostra o que os seus agentes fizeram: o resumo de uma semana e o último ano
numa grade. Tudo é contado neste computador, a partir de um histórico local que o Vults guarda
enquanto os seus agentes trabalham.

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
inteira no Settings; ele e o **×** a marcam como lida.

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
  propostas de memória esperando, e as execuções do sono ([Segurança](../safety.md)). O que o Zeca
  lembra (os arquivos de memória dele, **Settings → Chat → Memory**) fica.
- O histórico fica num banco local, `vults.sqlite` (cada turno por 12 semanas, os totais de cada
  dia por um ano), em `~/.local/share/vults/` (`%LOCALAPPDATA%\Vults\` no Windows). Veja
  [Configurações e arquivos](../reference/settings.md).
