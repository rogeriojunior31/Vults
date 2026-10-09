# Aprovando pela ilha
<!-- source: 7f6f1d8efff1 -->

Quando o Claude Code, o Codex ou o OpenCode pede permissão para uma chamada de ferramenta, a ilha abre num card
que mostra exatamente o que **Allow** autoriza (o Gemini CLI é a exceção, [abaixo](#gemini-cli)):

- as próprias palavras do agente sobre ela, quando ele as dá (*Run the test suite, then the linter*);
- o comando inteiro, o arquivo ou a URL, não só o nome da ferramenta. Um comando longo aparece
  completo (até quatro linhas, depois ele rola);
- numa edição, as linhas que ela adiciona e remove (**+12 −3**). Patches do Codex também mostram
  seus arquivos e contagens.

![Um card de permissão com o comando inteiro, Deny, Allow e Always allow](../../assets/island-approval.png)

O card ocupa toda a largura da ilha até você responder. Depois ele diz o que aconteceu por um
instante (**Allowed** em verde, **Denied** em vermelho) antes de a próxima coisa aparecer.

<a id="answering"></a>

## Respondendo

- **Allow** e **Deny** respondem direto ao agente; ele segue na hora.
- **Ctrl+Alt+Y** e **Ctrl+Alt+N** fazem o mesmo de qualquer lugar, sem sair da janela em que você
  está. São atalhos globais pelo portal do desktop: o KDE pede uma vez que você os aceite, e você pode
  trocar as teclas em **Configurações do Sistema → Atalhos** (System Settings → Shortcuts). Os botões
  mostram as teclas de fato associadas. Um atalho só responde a um card que está na tela (de um
  agente, ou um no chat); sem nada esperando, ele não faz nada.
- No modo *Panel*, em que a ilha fica fora de vista, um card também aparece na hora como uma
  notificação do desktop. O **Open** dela traz a ilha com o card; ela não tem Allow nem Deny
  ([notificações](island.md#desktop-notifications)). No topo da tela (*Island*, *Quiet*) a ilha já
  mostra o card com o som dele, e nada vai para o desktop.
- Enquanto o app está em *Paused* ([presença](island.md#presence-how-much-it-shows)), nenhum card
  aparece: o agente pergunta no terminal dele na hora, e as regras *Always* não respondem nada.
- **Always allow** permite o pedido e todo pedido idêntico daí em diante: o mesmo agente, a mesma
  ferramenta e exatamente o mesmo comando, arquivo ou URL, na mesma pasta de projeto. `cargo test` não
  cobre `cargo test && rm -rf build`, nem o mesmo comando em outro projeto. Esses pedidos são
  respondidos na hora, sem card, e o passo diz *always allowed* na ilha. Um pedido idêntico que já
  está esperando também é respondido. **Settings → Approvals** lista todas as regras, e **Remove**
  tira uma delas. Um comando com mais de uma linha, ou mais longo do que a primeira linha do card
  mostra (300 caracteres), não oferece **Always allow**: uma regra só veria o começo dele.
- Só você responde: com um clique ou um atalho agora, ou com **Always allow** antes. Nenhum timer ou
  padrão aprova nada, nunca.

<a id="more-than-one-at-a-time"></a>

## Mais de um ao mesmo tempo

Os pedidos esperam em fila, cada um pelo seu agente. O card mostra o primeiro com **1 of 3**; as
sessões esperando atrás dele dizem *Needs you* na lista do bando, com um selo âmbar. Responda um e o
próximo aparece.

Para responder primeiro um que está mais atrás, clique em **Open** na notificação da sessão dele (no *Panel*), ou
no pássaro dele no widget do canto: o card dele vai para a frente da fila. Só a ordem muda: cada card
mantém o próprio prazo, e só um clique nele o responde. Um card de pergunta que você tinha começado a
responder recomeça quando outro card é trazido para a frente dele. Um card cujo agente já seguiu em
frente (só um subagente dele ainda trabalha) não é trazido para a frente: ele não seria mostrado.

Um projeto que você silenciou ou ocultou ([por projeto](island.md#per-project-mute-pin-hide)) continua
recebendo seus cards, com som (e, no *Panel*, notificação) como qualquer outro; oculto, a sessão dele aparece com o
card e sai de novo depois que ele é respondido.

Um subagente trabalhando em paralelo não tira um card da tela: só o agente que perguntou seguir em
frente tira. Quando um agente pede duas chamadas de uma vez, responder a primeira deixa o segundo card
esperando: a primeira chamada terminar não diz nada sobre a outra.

<a id="when-nobody-answers"></a>

## Quando ninguém responde

Um pedido espera enquanto o agente dele espera (um pouco menos de dois minutos); enquanto isso, o
terminal diz *Waiting for your answer on the island*. Nos últimos 30 segundos o card faz uma contagem
regressiva, *Goes back to the terminal in 0:25*, e então o terminal pergunta como se o Vults não
estivesse ali. Se você responder no terminal, o card avisa e some.

Enquanto espera, um card sobe uma escada, para que um card que você perdeu chegue até você:

1. Ele abre a ilha, com o som dele (no *Panel*, também com uma notificação do desktop, se as
   notificações estiverem ligadas).
2. Aos 45 segundos, e a cada 30 segundos depois disso, o som dele toca de novo: aos 45, 75 e 105
   segundos.

**Do not disturb** (não perturbe; **Settings → General**, por 30 minutos, 1 hora ou 4 horas)
silencia os sons e as notificações de tudo o que está em repouso, e os lembretes dessa escada, até
terminar sozinho (pelo relógio: uma suspensão não o estica); uma lua no cabeçalho da ilha diz que ele
está ligado, e um clique nela o encerra. Um card ainda abre a ilha com o som dele (e, no *Panel*, a
notificação): nada esconde ou silencia um card pelo qual um agente está esperando.

<a id="auto-mode-and-questions"></a>

## Modo automático e perguntas

- No modo automático (auto mode), o classificador libera a maioria dos pedidos de permissão num
  piscar: a ilha só mostra um card, toca um som ou abre para um pedido que ainda está esperando
  depois de um instante, para não piscar a cada chamada de ferramenta. O mesmo vale para "terminou":
  só uma sessão que continua pronta é novidade.

<a id="questions"></a>

## Perguntas

Quando o Claude Code ou o OpenCode pergunta algo a você com opções (a ferramenta `AskUserQuestion`
do Claude Code, a ferramenta `question` do OpenCode), a ilha abre num card de pergunta em vez de um
card de permissão:

![Um card de pergunta: a pergunta Theme, 1 de 2, com três opções, Other… e Reply in the terminal](../../assets/island-question.png)

- Uma pergunta por vez, com a etiqueta dela (**Theme**) e **1 of 2** quando ele faz várias de uma
  vez.
- Um clique numa opção a responde. Quando várias podem ser escolhidas, marque-as e aperte **Next**
  (ou **Send** na última).
- **Other…** abre um campo para as suas próprias palavras; **Enter** o envia, **Escape** volta para
  as opções.
- **Reply in the terminal** devolve a pergunta ao terminal do agente, onde você responde como de
  costume.

O Claude Code espera pela ilha enquanto o card está aberto, então a pergunta só aparece no terminal
depois que você escolhe **Reply in the terminal**, ou quando o tempo do card acaba (a mesma contagem
regressiva de uma permissão). **Ctrl+Alt+Y** e **Ctrl+Alt+N** nunca respondem a uma pergunta: ela
precisa da sua escolha.

Isso precisa do Claude Code 2.1.85 ou mais recente e de hooks instalados por esta versão. Hooks de
uma versão mais antiga mostram **Update available** em **Settings → Agents**; até você atualizá-los, a
ilha mostra a pergunta e leva você ao terminal, como faz para o Codex e o Gemini.

Se o app estiver fechado ou sem responder, o hook não responde nada em milissegundos e todo agente
pergunta no terminal dele, como de costume.

## Gemini CLI

Os hooks do Gemini podem bloquear uma ferramenta, mas não aprovar: a confirmação dele sempre roda.
Por isso uma sessão do Gemini nunca recebe um card. Quando o Gemini pergunta, o pássaro dele mostra
uma pergunta (*Run rm -rf dist? Answer in Gemini's terminal.*) e você responde lá. Todo o resto (os
passos, o bando, pular para o terminal) funciona como para os outros agentes.

## OpenCode

As permissões e as [perguntas](#questions) do OpenCode também ganham um card, pelo plugin que
**Settings → Agents → OpenCode** escreve (conferido com o OpenCode 1.18.35). O prompt do próprio OpenCode continua aberto enquanto o
card espera: responda na ilha ou no OpenCode, e vale a primeira resposta; o card some quando você
responde no OpenCode.

- **Allow** permite aquela chamada (o *Allow once* do OpenCode). **Always allow** é uma regra do
  Vults, como para os outros agentes: o *Always* do próprio OpenCode, que mudaria as configurações
  dele, nunca é usado.
- **Deny** rejeita a chamada, como o *Reject* do OpenCode.
- Uma pergunta com mais de quatro opções, ou mais de quatro perguntas de uma vez, fica no prompt do
  OpenCode: a ilha mostra que ela espera lá.
- Nenhum clique antes do prazo do card, o app fechado, um plugin de uma versão anterior: o OpenCode
  só espera a sua resposta no próprio prompt. A tela do OpenCode não mostra *Waiting for your answer
  on the island*.

Um plugin instalado por uma versão anterior só informa: **Settings → Agents** mostra **Update
available**, e **Update plugin…** escreve o novo.

## Antigravity

O Antigravity pede as permissões no próprio terminal, e a ilha nunca as responde: o pássaro dele
mostra os passos, e você aprova onde o Antigravity pergunta. Veja
[Outros agentes](other-agents.md#antigravity).
