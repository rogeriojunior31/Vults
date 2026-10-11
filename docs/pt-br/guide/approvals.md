# Aprovando pela ilha
<!-- source: e0a6d58c31ee -->

Quando o Claude Code, o Codex, o OpenCode ou o Qwen Code pede permissão para uma chamada de
ferramenta, a ilha abre num card que mostra exatamente o que **Allow** autoriza (o Gemini CLI e o
Antigravity são as exceções, [abaixo](#gemini-cli)):

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
- **Ctrl+Alt+Y** e **Ctrl+Alt+N** fazem o mesmo de qualquer lugar, sem sair da sua janela. São
  atalhos globais pelo portal do desktop, então só no Linux (a prévia para Windows não tem nenhum): o
  KDE pede uma vez que você os aceite, e **Configurações do Sistema → Atalhos** (System Settings →
  Shortcuts) troca as teclas. Os botões mostram as teclas de fato associadas. Um atalho só responde a
  um card que está na tela; sem nada esperando, ele não faz nada.
- No modo *Panel* a ilha fica fora de vista, então um card também aparece como uma notificação do
  desktop. O **Open** dela traz a ilha com o card; ela não tem Allow nem Deny
  ([notificações](island.md#desktop-notifications)).
- Enquanto o app está em *Paused* ([presença](island.md#presence-how-much-it-shows)), nenhum card
  aparece: o agente pergunta no terminal dele na hora, e as regras *Always* não respondem nada.
- **Always allow** permite o pedido e todo pedido idêntico daí em diante: o mesmo agente, a mesma
  ferramenta e exatamente o mesmo comando, arquivo ou URL, na mesma pasta de projeto. `cargo test` não
  cobre `cargo test && rm -rf build`, nem o mesmo comando em outro projeto. Esses pedidos são
  respondidos sem card, e o passo diz *always allowed*; um pedido idêntico que já espera também é
  respondido. **Settings → Approvals** lista todas as
  regras, e **Remove** tira uma delas. Um comando com mais de uma linha, ou com mais de 300
  caracteres, não oferece **Always allow**: uma regra só veria o começo dele.
- Só você responde: com um clique ou um atalho agora, ou com **Always allow** antes. Nenhum timer ou
  padrão aprova nada, nunca.

<a id="more-than-one-at-a-time"></a>

## Mais de um ao mesmo tempo

Os pedidos esperam em fila, cada um pelo seu agente. O card mostra o primeiro com **1 of 3**; as
sessões atrás dele dizem *Needs you* na lista do bando, com um selo âmbar. Responda um e o próximo
aparece.

Para responder primeiro um que está mais atrás, clique em **Open** na notificação dele (no *Panel*)
ou no pássaro dele no widget do canto: o card dele vai para a frente, mantendo o próprio prazo.
Projetos silenciados ou ocultos ([por projeto](island.md#per-project-mute-pin-hide)) continuam
recebendo seus cards e sons.

Uma pergunta respondida pela metade recomeça quando outro card vem para a frente dela. Um card cujo
agente seguiu adiante não vem para a frente, mas um subagente trabalhando em paralelo não tira um
card. Quando um agente pede duas chamadas de uma vez, responder a primeira deixa a segunda esperando.

<a id="when-nobody-answers"></a>

## Quando ninguém responde

Um pedido espera enquanto o agente dele espera (um pouco menos de dois minutos); enquanto isso, o
terminal diz *Waiting for your answer on the island*. Nos últimos 30 segundos o card faz uma contagem
regressiva, *Goes back to the terminal in 0:25*, e então o terminal pergunta como se o Vults não
estivesse ali. Se você responder no terminal, o card avisa e some.

Enquanto espera, um card sobe uma escada, para que um card que você perdeu chegue até você:

1. Ele abre a ilha, com o som dele (no *Panel*, também com uma notificação do desktop, se as
   notificações estiverem ligadas).
2. O som dele toca de novo aos 45, 75 e 105 segundos.

**Do not disturb** (não perturbe; **Settings → General**, por 30 minutos, 1 hora ou 4 horas)
silencia os sons e as notificações de tudo o que está em repouso, e os lembretes dessa escada, até
terminar pelo relógio (uma suspensão não o estica). Uma lua no cabeçalho da ilha diz que ele está
ligado; um clique nela o encerra. Um card ainda abre a ilha com o som dele (e, no *Panel*, a
notificação): nada esconde um card pelo qual um agente está esperando.

Se o app estiver fechado ou sem responder, o hook não responde nada em milissegundos e todo agente
pergunta no terminal dele, como de costume.

<a id="claude-codes-auto-mode"></a>
<a id="auto-mode-and-questions"></a>

## O modo automático do Claude Code

No modo automático (auto mode), o classificador libera a maioria dos pedidos num piscar, então a
ilha só mostra um card, toca um som ou abre para um pedido que ainda está esperando depois de um
instante. O mesmo vale para "terminou": só uma sessão que continua pronta é novidade.

<a id="questions"></a>

## Perguntas

Quando o Claude Code ou o OpenCode pergunta algo a você com opções (a ferramenta `AskUserQuestion`
do Claude Code, a ferramenta `question` do OpenCode), a ilha abre num card de pergunta:

![Um card de pergunta: a pergunta Theme, 1 de 2, com três opções, Other… e Reply in the terminal](../../assets/island-question.png)

- Uma pergunta por vez, com a etiqueta dela (**Theme**) e **1 of 2** quando ele faz várias.
- Um clique numa opção a responde. Quando várias podem ser escolhidas, marque-as e aperte **Next**
  (ou **Send** na última).
- **Other…** abre um campo para as suas próprias palavras; **Enter** o envia, **Escape** volta.
- **Reply in the terminal** devolve a pergunta ao terminal do agente. Até lá, ou até o tempo do card
  acabar, o Claude Code espera pela ilha.
- **Ctrl+Alt+Y** e **Ctrl+Alt+N** nunca respondem a uma pergunta.

Isso precisa do Claude Code 2.1.85 ou mais recente e de hooks instalados por esta versão. Hooks mais
antigos mostram **Update available** em **Settings → Agents**; até você atualizá-los, a ilha mostra a
pergunta e leva você ao terminal, como faz para o Gemini CLI e o Qwen Code.

## Gemini CLI

Os hooks do Gemini podem bloquear uma ferramenta, mas não aprovar: a confirmação dele sempre roda,
então uma sessão do Gemini nunca recebe um card. Quando o Gemini pergunta, o pássaro dele mostra a
pergunta (*Run rm -rf dist? Answer in Gemini's terminal.*) e você responde lá.

## OpenCode

As permissões e as [perguntas](#questions) do OpenCode ganham um card pelo plugin que **Settings →
Agents → OpenCode** escreve (conferido com o OpenCode 1.18.35 e 2.0.26). O prompt do próprio OpenCode
continua aberto enquanto isso: responda na ilha ou no OpenCode, e vale a primeira resposta; o card
some quando você responde no OpenCode.

- **Allow** é o *Allow once* do OpenCode; **Deny**, o *Reject* dele. **Always allow** é uma regra do
  Vults: o *Always* do próprio OpenCode, que mudaria as configurações dele, nunca é usado.
- Algumas perguntas ficam no prompt do OpenCode: todas no OpenCode 2, que não deixa um plugin
  respondê-las, e no OpenCode 1 as com mais de quatro opções ou mais de quatro de uma vez. A ilha
  mostra que uma espera lá.
- Sem clique a tempo, com o app fechado ou com um plugin desatualizado (**Update plugin…** em
  **Settings → Agents**), o OpenCode só espera no próprio prompt.

## Qwen Code

As permissões do Qwen Code ganham um card pelos hooks que **Settings → Agents → Qwen Code** instala
(conferido com o Qwen Code 0.25.0). O Qwen lê a mesma resposta que o Claude Code, então **Allow**,
**Deny** e **Always allow** funcionam como lá.

- Enquanto o card está aberto, o Qwen mostra só o spinner; sem clique a tempo, ou com o app fechado,
  ele pergunta no próprio prompt.
- As perguntas dele (`ask_user_question`) ficam no terminal: o Qwen não aceita resposta de um hook
  para elas. A ilha mostra que ele espera lá.
- Os cards aparecem no modo de aprovação *default* do Qwen (**Shift+Tab** no Qwen, ou
  `qwen --approval-mode default`); uma instalação nova começa no *auto*, em que o próprio
  classificador dele decide. No *yolo* o Qwen aprova tudo, então nenhum card aparece.

## Antigravity

O Antigravity pede as permissões no próprio terminal, e a ilha nunca as responde: o pássaro dele
mostra os passos ([Outros agentes](other-agents.md#antigravity)).
