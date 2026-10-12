# Aprovando pela ilha
<!-- source: cda06964d9a7 -->

Quando o Claude Code, o Codex, o OpenCode, o Qwen Code ou o Copilot CLI pede permissão para uma chamada de
ferramenta, a ilha abre num card que mostra exatamente o que **Allow** autoriza (o
Antigravity, o Cursor, o Factory Droid e o Kimi Code são as exceções, [abaixo](#agents-that-ask-in-their-own-prompt)):

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
- Embaixo do comando, uma linha diz por que ele pergunta quando há mais a dizer do que "nada
  permite": *Destructive: no policy ever allows it* ou *Your policy asks first for
  this*.
- **Ctrl+Alt+Y** e **Ctrl+Alt+N** fazem o mesmo de qualquer lugar, sem sair da sua janela
  ([atalhos](island.md#getting-to-a-session)). Os botões mostram as teclas de fato associadas. Um
  atalho só responde a um card que está na tela; sem nada esperando, ele não faz nada.
- No [Nest](nest.md), o card é respondido onde você está: enquanto a janela dele tem o foco, a ilha
  fica de lado e o card da sessão em **Flock** tem os mesmos botões (e aceita as mesmas teclas).
  Saia do Nest com um card esperando e a ilha aparece para ele.
- No modo *Panel* a ilha fica fora de vista, então um card também aparece como uma notificação do
  desktop. O **Open** dela traz a ilha com o card; ela não tem Allow nem Deny
  ([notificações](island.md#desktop-notifications)).
- Enquanto o app está em *Paused* ([presença](island.md#presence-how-much-it-shows)), nenhum card
  aparece: o agente pergunta no terminal dele na hora, e nem as regras *Always* nem as
  [políticas](#policies) respondem.
- **Always allow** permite o pedido e todo pedido idêntico daí em diante: o mesmo agente, a mesma
  ferramenta e exatamente o mesmo comando, arquivo ou URL, na mesma pasta de projeto. `cargo test` não
  cobre `cargo test && rm -rf build`, nem o mesmo comando em outro projeto. Esses pedidos são
  respondidos sem card, e o passo diz *always allowed*; um pedido idêntico que já espera também é
  respondido. **Settings → Approvals** lista todas as
  regras, e **Remove** tira uma delas. Um comando com mais de uma linha, ou com mais de 300
  caracteres, não oferece **Always allow**: uma regra só veria o começo dele.
- Um comando destrutivo (`rm -rf`, um push forçado) sempre pergunta: o card dele diz isso em laranja,
  deixa **Deny** como o botão claro, contorna **Allow** em vermelho e não oferece **Always allow**.
  Nenhuma regra ou política o responde, nem uma regra salva antes.
- Só você responde: com um clique ou um atalho agora, ou antes com **Always allow** ou uma
  [política](#policies) que você aceitou. Nenhum timer ou padrão aprova nada, nunca.

<a id="policies"></a>

## Políticas

Uma política responde cards por você: você a escreve em **Settings → Approvals → Policies**, vê como
um diff contra a que está em vigor e aceita com um clique. Uma política por pasta de projeto, mais
uma para todo lugar; uma mais nova substitui a mais velha. **Revoke** desliga na hora, e os cards
dela voltam a perguntar. Por enquanto uma política só **permite** chamadas do Claude Code, o único
agente cujos cards mostram a chamada inteira (os shells dos outros rodam numa pasta que o card
omite); para Codex, OpenCode, Qwen Code e Copilot CLI ela pode negar ou perguntar. Agentes cujas
permissões não viram card (Antigravity, Cursor, Factory Droid, Kimi Code) não são respondidos por ela. O chat do Zeca só as segue quando
você escolhe **Your policies** ou **Full access** para ele ([Quem responde os cards do
Zeca](chat.md#who-answers-zecas-cards)).

```
# Linhas que começam com # são notas.
project ~/Projects/site
agents claude codex
allow read
allow write
allow command git status
allow command git diff *
deny external
ask tool WebFetch
```

- `project` define a pasta em que ela vale (e abaixo dela); sem ele, vale em todo lugar. `agents`
  limita a alguns agentes; sem ele, todo agente embutido.
- `allow`, `deny` ou `ask`, depois uma classe (`read`, `read-external`, `write`, `external`),
  `command` e as palavras dele (um `*` no fim deixa vir mais palavras), ou `tool` e o nome exato de
  uma ferramenta (um shell ou uma ferramenta MCP só com `deny` ou `ask`).
- Uma ação destrutiva (`rm`, um push forçado, um redirecionamento por cima de um arquivo) sempre
  pergunta: nenhuma linha a responde.
- Um deny vence um ask, e um ask vence um allow, entre todas as políticas que valem; um deny vence
  também um **Always allow** mais antigo. Uma linha `deny` ou `ask` cobre o que vier depois das
  palavras dela, com `*` ou não, e um comando que tem as duas primeiras palavras dela em qualquer
  lugar (`git --no-pager push`, ou um `git push` sozinho, com `deny command git push origin main`)
  nunca é permitido. Ela nomeia um programa: `deny command curl` não diz nada sobre `wget`.
  `deny tool Bash` vale para o shell de todo agente.
- Um allow precisa que o Vults saiba o que a ação faz: uma ferramenta MCP, uma ferramenta ou comando
  desconhecido, um interpretador, um build ou um teste (`cargo test` roda o código do projeto)
  sempre perguntam, e também os comandos git que rodam os hooks do repositório (`commit`, `merge`,
  `rebase`, `pull`, `push`, `checkout`, `switch`, `cherry-pick`, `revert`, `am`, `worktree`,
  `clone`). Também perguntam um comando com símbolo do shell, aspas, glob ou tab, um comando
  longo ou de várias linhas, e um card cujo alvo foi cortado. O **Always allow** continua cobrindo
  um comando exato.
- Um allow só alcança a pasta da sessão: aquela em que o Vults a viu começar, nunca `/` nem a sua
  pasta pessoal, e nada depois que o agente saiu dela (uma sessão que o Vults viu pela primeira vez
  depois, após um reinício, não ganha allow). Um caminho fora dela, um que sobe (`..`), vários
  caminhos num alvo só ou uma escrita dentro de `.git` pergunta. As palavras de um comando permitido
  não nomeiam caminho absoluto, URL, `@arquivo`, `..`, `.git` nem opção grudada num caminho, então
  um fetch com URL sempre pergunta.
- Uma linha ampla cobre toda a classe: `allow external` cobre `git push`, `gh pr merge`,
  `gh workflow run`, `gh gist create` e outras ações para fora deste computador que o Vults conhece,
  inclusive as que mandam um arquivo da pasta para fora; `allow write` cobre toda escrita conhecida
  que pode ser desfeita. Prefira linhas `command`.
- Uma resposta de uma política não mostra card; o passo diz *allowed by your policy*, e o log de
  auditoria nomeia o consentimento da política ([Segurança](../safety.md)).
- Editar uma política para que ela nomeie outra pasta revoga a que você editou quando você aceita;
  a revisão avisa.
- O que ela não vê: links. Um symlink dentro da pasta que leva para fora dela conta como dentro. E
  `allow write` deixa o agente mudar arquivos que rodam depois, quando você ou uma ferramenta os
  roda (um `Makefile`, um `.envrc`, os scripts do `package.json`). Uma
  política é uma linha no seu banco de dados local: outro programa rodando como você poderia
  adicionar uma, como poderia mudar qualquer arquivo seu.

<a id="more-than-one-at-a-time"></a>

## Mais de um ao mesmo tempo

Os pedidos esperam em fila, cada um pelo seu agente. O card mostra o primeiro com **1 of 3**, embaixo
de um ponto por sessão na cor do agente. Responda um e o próximo aparece.

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
terminal diz *Waiting for your answer in Vults*. Nos últimos 30 segundos o card faz uma contagem
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
pergunta e leva você ao terminal, como faz para o Qwen Code.

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

## Copilot CLI

As permissões do Copilot CLI ganham um card pelo arquivo de hooks que **Settings → Agents →
Copilot CLI** escreve, `~/.copilot/hooks/vults.json` (conferido com o Copilot CLI 1.0.96). O Copilot lê
todos os arquivos dessa pasta, então o que o Vults escreve guarda só os hooks dele; um arquivo ali
que o Vults não escreveu nunca é tocado.

- **Allow** e **Deny** respondem àquela chamada; **Always allow** é uma regra do Vults.
- O card de uma edição mostra as linhas que ela adiciona e remove; um arquivo novo aparece como escrita.
- Sem clique a tempo, ou com o app fechado, o Copilot pergunta no próprio prompt.
- Ele não tem ferramenta de perguntas, então nenhuma pergunta chega à ilha.
- Um comando que falhou aparece como falha; o que uma ferramenta imprimiu nunca sai do hook.

<a id="antigravity"></a>
<a id="cursor"></a>
<a id="factory-droid"></a>
<a id="kimi-code"></a>
<a id="agents-that-ask-in-their-own-prompt"></a>

## Agentes que perguntam no próprio prompt

Antigravity, Cursor, Factory Droid e Kimi Code pedem as permissões no próprio terminal ou prompt, e
a ilha nunca as responde: os pássaros deles mostram os passos. [O que cada agente consegue
fazer](other-agents.md#what-each-agent-can-do) os resume, e a seção de cada um ali diz por que ele
não tem card. Uma tarefa que o Vults roda no Kimi Code pede com um card cada escrita e cada comando,
como o agente de qualquer tarefa ([Tarefas](tasks.md)).
