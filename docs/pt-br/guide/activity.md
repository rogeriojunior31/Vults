# Atividade
<!-- source: 74e51484fb2c -->

**Settings → Activity** mostra o que os seus agentes fizeram: o resumo de uma semana e o último ano
numa grade. Tudo é contado neste computador, a partir de um histórico local que o Vults guarda
enquanto os seus agentes trabalham.

## O que é um turno

Um turno é o que um prompt pôs para andar: começa com o prompt (ou com o primeiro passo ou card de
um agente que não manda o evento de prompt) e termina quando o agente para, falha, ou a sessão dele
acaba. Uma sessão que morre sem avisar, ou um turno sem nenhum evento por duas horas, termina no
último evento.

Para cada turno terminado o Vults guarda quando ele terminou e em que dia, quanto durou, o agente, o
nome da pasta do projeto, e contagens: passos, comandos, arquivos e linhas que as edições mudaram,
as permissões que você permitiu ou negou e as perguntas que respondeu (uma regra que você criou com
**Always** conta como o seu Allow), e se ele falhou. Nunca um prompt, uma resposta, um comando, um
nome de arquivo ou um caminho, e nada sai deste computador ([Segurança](../safety.md)).

## A semana

O card mostra uma semana, de segunda a domingo, pelos seus dias locais: esta semana primeiro, e
**‹ ›** para passar pelas semanas que o histórico guarda (12).

- **Tempo com os seus agentes**: turnos rodando lado a lado contam uma vez só, então duas sessões
  trabalhando na mesma hora dão uma hora.
- **Turnos** e os passos deles, **linhas** adicionadas e removidas e os arquivos mexidos,
  **comandos**, **respostas** (permitidas, negadas, perguntas respondidas), e **turnos que
  falharam**.
- O tempo por dia, em barras, e o agente e o projeto com mais turnos, o dia mais movimentado e o
  turno mais longo.

## Compartilhando a semana

**Save as image…** embaixo da semana a desenha como uma imagem (1080 × 1350): o tempo, a frase, seis
números e os dias, com o Zeca no fio dele. **Hide project names** tira o projeto da frase. O diálogo
*Salvar como* do próprio desktop pergunta onde (no Windows ela vai para a sua pasta Imagens), a
página diz onde ela ficou, e nada é enviado.

## Na segunda de manhã

A partir das 08:00 de segunda, a ilha conta a semana passada numa linha (*Last week: 41 turns, 6 h
20 min with your agents, most on site.*), uma vez, se os seus agentes tiveram algum turno naquela
semana. No preset *Island* ela abre a ilha; no painel ou no *Quiet* ela espera lá até você abrir, e
no *Paused* ela não vem. Um card esperando por você vem primeiro. **Open Activity** mostra a semana
inteira no Settings; ele e o **×** a marcam como lida, e ela não volta.

## A grade

O último ano, uma coluna por semana e uma linha por dia, com a segunda em cima. O tom de um dia
segue o tempo com os seus agentes: nenhum, depois quatro degraus pelos quartis dos seus dias ativos,
como o gráfico de contribuições do GitHub. Passe o mouse num dia para ver os turnos e o tempo.

A aba **GitHub** mostra o seu calendário de contribuições do GitHub, com os níveis do próprio GitHub
e as semanas começando no domingo, como no seu perfil. Ela precisa do conector GitHub ligado
(**Settings → Connectors**): é pedido pelo `gh` em que você está logado, uma vez quando a aba abre, e
guardado por uma hora na memória, nunca em disco. Sem o conector a aba avisa, e nada é pedido.

## O histórico

- **Keep a history** vem ligado. Desligado, nada novo é guardado e o que existe fica.
- **Clear history…** pergunta uma vez, depois apaga todos os turnos e dias guardados.
- O histórico fica num banco local, `vults.sqlite` (cada turno por 12 semanas, os totais de cada
  dia por um ano), em `~/.local/share/vults/` (`%LOCALAPPDATA%\Vults\` no Windows). O
  `history.jsonl` e o `days.json` das versões anteriores entram nele na primeira abertura e depois
  saem. Veja [Configurações e arquivos](../reference/settings.md).
