# O Nest
<!-- source: 5f3be4699d2a -->

O Nest é a janela do próprio Vults, para quando a ilha fica pequena: o bando inteiro, as suas
tarefas, o chat, o que aconteceu e as suas conexões. Abra pelo ícone da bandeja (**Nest…**) ou pela
seta no topo do chat da ilha. Ele é criado quando você abre e some quando você fecha, e abre também
com o Zeca desligado, só que sem o Chat.

Cinco abas ficam no topo, depois o seletor de tamanho e a engrenagem, que mostra as Configurações
nesta mesma janela, sob as mesmas abas; uma aba traz o Nest de volta. As Configurações continuam uma
página própria ali: são a única que muda a config de um agente, uma chave, uma política ou um
celular, e nunca mostram texto de agente. O
seletor mantém a aba em que você está: **Room** é a janela, **Full** a tela toda (**Esc** volta para
Room), **Quick** fecha o Nest e abre a ilha, **Mini** o fecha e deixa a pílula. **Alt+1** a
**Alt+5** escolhem uma aba. Embaixo, **Zeca on** / **Zeca off**
liga e desliga o Zeca (como **Settings → Flock & Zeca → Zeca**) e **Back to the island** traz a ilha.

**Ctrl+K**, no Nest ou na ilha aberta (quando ela tem o teclado: aberta com Ctrl+Alt+Space, ou no
chat dela), abre a paleta: digite para pular para uma sessão, para
começar `/task`, `/delegate`, `/continue` ou `/remember` no chat (com o Zeca ligado), ou para abrir uma
aba. Ela nunca responde um card: uma sessão que espera por você só leva até a ilha.

## Flock

O bando no dobro do tamanho, no habitat dele, e depois cada sessão numa linha: o projeto, o
repositório e o branch, o agente e o que ela está fazendo. Um clique numa ave ou numa linha (ou
**↑ ↓**) põe aquela sessão na frente, e os detalhes dela aparecem: o que ela mudou, os últimos
passos, a pasta, o repositório, o branch, o editor e o custo, e com o Zeca ligado **Ask Zeca about
this session**, que abre o Chat com a pergunta pronta. **Needs you** deixa só as sessões que esperam
por você; **Find a session…** lê projetos, repositórios, branches, agentes e o que cada uma faz agora. Numa janela larga
ou no Full, os detalhes ficam ao lado das linhas.

Uma sessão que espera por você mostra **Review on the island**: só a ilha responde um card
([Aprovar pela ilha](approvals.md)), e o botão a traz com essa sessão na frente. **Open terminal**
aparece onde o Vults consegue trazer o terminal dela para a frente
([A ilha](island.md#getting-to-a-session)).

## Tasks

As suas [tarefas](chat.md#tasks), sob o roost de cada uma (as sem roost por último): cada uma com o
objetivo, os critérios, o dono, o status e o custo, por que o roteador escolheu o agente dela, as
rodadas de revisão como marcas (até três, a última na cor do veredito), os botões que a iniciam,
revisam ou passam adiante, e o gasto de hoje frente ao seu orçamento diário (definido em Activity →
Cost). **New task** guarda uma
sem o chat, que é como você cria uma com o Zeca desligado.

## Chat

A conversa da ilha com o Zeca, larga, com respostas em Markdown completo (um link mostra o endereço
e nunca é seguido); uma mensagem, **New** ou outro motor em qualquer um aparece nos dois. Os cards
aparecem aqui também, mas só a ilha os responde: **Answer on the island** traz o chat dela.
Numa janela larga ou no Full, **Recent** fica ao lado dela: as suas 30 conversas mais novas, a atual marcada,
e **New conversation** (os dois esperam enquanto o Zeca responde); um clique continua uma delas. **Memory** abre, ao lado da conversa, o que o
Zeca lembra: o que espera o seu sim, os arquivos e o
histórico deles ([A memória do Zeca](memory.md)). Sem o Zeca não há aba Chat.

## Activity

O que este computador guarda, só lido aqui:

- **Week**: o resumo da semana e a grade do ano, com os interruptores do histórico ([Atividade](activity.md)).
- **By project**: um card para cada repositório em que uma sessão trabalha: as pastas e os
  branches, as sessões, os seus pull requests e checks (do card do GitHub; um pull request ou os
  checks do branch padrão abrem no GitHub), o que o Zeca lembra do projeto e quanto custaram as
  tarefas dele ([A memória do Zeca](memory.md#project-memory)).
- **History**: cada turno terminado dos seus agentes, só contagens.
- **Cost**: quanto os agentes custaram hoje e o **orçamento diário** que você define aqui, as janelas
  de uso dos seus planos e o que os agentes custaram nas últimas duas semanas, frente a esse
  orçamento, e por roost ([Custo e orçamento diário](chat.md#cost-and-a-daily-budget)).
- **Traces**: o trabalho do próprio Zeca, cada turno do chat com o modelo, os tokens e o custo, e
  abaixo dele cada card de ferramenta com quanto tempo esperou por uma resposta; guardado por 30 dias.
- **Audit**: quem respondeu cada card e como (você, o seu telefone, uma regra Always ou uma política
  que você aceitou); guardado por 90 dias e nunca alterado.

## Connections

O card de cada conector que você ligou, como a ilha mostra: no GitHub, os seus pull requests, as
revisões que esperam por você e os seus branches padrão, cada um com os checks. **All**, **Waiting
for your review** e **Checks failed**, cada um com a contagem, mostram o que você quer. Uma linha abre no
GitHub. **Add a connection** lista cada conector do Vults que está desligado, com o que ele observa;
**Turn on in Settings…** e **Manage connections** abrem **Settings → Connectors**, o único lugar que
liga um conector ([Conectores](connectors.md)).
