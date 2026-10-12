# A memória do Zeca
<!-- source: 03439a8dcb18 -->

O Zeca pode lembrar de você, dos seus projetos e do que os seus agentes fizeram. Essa memória fica
desligada até você ligá-la, é guardada neste computador, e só você a escreve.

<a id="turning-it-on"></a>

## Ligando

Ligue em **Settings → Chat & voice → Memory → Zeca's memory**. O app então mantém um diário no
`vults.sqlite` local por 90 dias: o tempo, os passos e as mudanças de cada turno dos agentes, a
resposta de cada card com o comando ou caminho, e as linhas do chat do Zeca, com os segredos
redigidos antes de qualquer coisa ser escrita. Os outros interruptores da seção aparecem quando
este está ligado.

Desligada, nada da memória chega a um modelo (nenhum fato, nenhuma lembrança, nenhum diário): a
**Memory** ao lado do Chat do Nest mostra só *Zeca's memory is off…*, **By project** não mostra fatos da memória,
e o `/remember` é recusado. Desligá-la para a escrita e mantém o que já foi
escrito.

O que o Zeca lembra é um repositório git de pequenos arquivos Markdown em
`~/.local/share/vults/memory/`, cada mudança um commit. **Settings → Activity → Clear history**
apaga o diário, os resumos dos dias e os dados de lembrança, as propostas de memória, as rodadas de
sono, as conversas, o rastro, o log de auditoria e o arquivo com o retrato do bando, mas não essa
pasta: para apagar os fatos dele, apague-a.

<a id="what-joins-a-conversation"></a>

## O que entra numa conversa

Quando uma conversa começa, a memória dele entra nela depois das instruções dele: quem você é, o
que ele aprendeu, como falar com você, e a memória do projeto em que ela trabalha. Ela fica fixa a
conversa toda; um fato adicionado nesse meio-tempo entra na próxima. Desligar a memória não a tira
de uma conversa já em andamento; reaberta depois pela lista, ela começa sem os fatos dele.

Antes de cada mensagem, o app procura no diário e nos resumos dos dias o que as suas palavras
tratam, neste computador, e até três trechos entram na mensagem como o bloco `<memories>`,
contexto e nunca instruções. O chat de hoje fica de fora, e também tudo o que uma conversa disse
depois de ler conteúdo de fora. Os trechos vão para o provedor do chat junto com a sua mensagem. O
chat por API também pode pedir mais com a ferramenta `recall`, e sugerir um fato com o
`memory_propose`, que só escreve uma proposta.

<a id="writing-his-memory"></a>

## Escrevendo a memória dele

- **`/remember <fato>`** no chat, com a memória ligada, guarda um fato no que o Zeca aprendeu; **`/remember project:
  <fato>`** no projeto em que a conversa está. **`/forget <palavras>`** remove o único fato que as
  contém (com vários, ele os lista).
- **O que o Zeca propõe** espera na **Memory** ao lado do Chat do Nest, com para onde vai (um projeto pelo nome
  e pela pasta) e a mudança como um diff: **Remember** guarda, **Turn down** descarta.
- A Memory também lista o que ele lembra, arquivo por arquivo com o quanto cada um está cheio,
  e o histórico: quem escreveu cada mudança (você, ou o Zeca com o seu sim) e **Undo**, que a
  desfaz numa mudança nova (recusado quando uma mudança depois mexeu no mesmo arquivo; nunca
  oferecido numa que o Vults não escreveu).

Um fato nunca entra duas vezes: o `/remember` e toda proposta são recusados quando outro fato já
diz isso, sem contar maiúsculas, espaços e o ponto final, e o mesmo fato decidido no último mês não
é proposto de novo. Uma proposta passa pela verificação da memória (nada de caracteres escondidos,
palavras que falam com um modelo ou formatos de chave).

<a id="what-he-does-on-his-own"></a>

## O que ele faz sozinho

Cada um é um interruptor em **Settings → Chat & voice → Memory**, desligado até você ligá-lo. O Haiku é o
menor modelo do Claude Code, pelo seu `claude` e pela sua assinatura, sem ferramentas e sem nenhuma
das suas configurações; cada chamada passa pelo portão de rede e pelo rastro local
([Segurança](../safety.md#every-request-vults-makes)).

| Recurso | Interruptor | O que lê | Modelo | Requisições por dia |
|---|---|---|---|---|
| Um resumo de cada dia passado (no máximo 2.000 bytes) | **A summary of each day** | Um dia do diário, só os dias desde que você ligou | Haiku | 8 |
| Até três fatos para lembrar, quando você começa ou abre outra conversa | **Suggest what to remember** | A conversa que você deixou, redigida, com pelo menos 2 mensagens suas e nenhum conteúdo de fora | Haiku | 20 |
| Junções, e um resumo da volta mostrado como *While you were away* | **Let Zeca sleep on it** | O que ele lembra sobre você e entre projetos, e os resumos dos últimos 7 dias | Haiku | 4 |
| Lembrar pelo sentido, não só pelas palavras | **Recall by meaning** | Cada resumo, e cada mensagem que você manda | `nomic-embed-text` num [Ollama](https://ollama.com) em `127.0.0.1` | nenhuma sai do computador |

O resumo roda numa passada de hora em hora: cada dia passado sem resumo, até três por passada (ele
alcança o atraso depois de dias fora), três tentativas por dia enquanto o app roda. Os embeddings
rodam nessa passada, para cada resumo sem um, e antes de cada mensagem do chat (rode antes
`ollama pull nomic-embed-text`).

O sono vem uma vez por dia, a partir das duas da manhã e enquanto nada está trabalhando. Mesmo sem
o interruptor dele, ele propõe remover um fato que repete outro na mesma seção, sem modelo. Com ele,
também propõe junções e escreve o resumo da volta, no seu idioma, marcado como a leitura dele dos
resumos. Toda mudança espera o seu sim.

<a id="project-memory"></a>

## Memória de projeto

**Activity → By project** do Nest tem um card para cada repositório em que uma sessão trabalha: as pastas e
os branches dele, as sessões, os seus pull requests e checks (do card do GitHub), e o que o Zeca
lembra do projeto. Ele só lê este computador. Uma conversa que começa num repositório leva essa
memória junto.

<a id="sharing-agentsmd-with-claude"></a>

### Compartilhando o AGENTS.md com o Claude

Quando um repositório tem um `AGENTS.md` (a memória de projeto que o Codex lê) que o `CLAUDE.md`
dele não importa, o card dele oferece **Share it with Claude…**. Ele mostra a mudança (`@AGENTS.md`
adicionado no fim do `CLAUDE.md`, `@../AGENTS.md` no fim do `.claude/CLAUDE.md`, ou um `CLAUDE.md`
novo) e só a escreve quando você clica em **Apply**, sobre o mesmo arquivo que você viu, depois de
um backup datado em `~/.local/share/vults/backups/`. Aí o Claude Code e o Codex leem a mesma
memória de projeto.

<a id="through-claude-code"></a>

## Pelo Claude Code

O chat entrega ao Claude Code o próprio servidor MCP do Zeca, `vults-zeca`, com `--mcp-config` (a
sua configuração do Claude nunca é tocada): `flock_list` (as suas sessões agora), `recall` e
`journal_day` (o diário dele), `roost_memory` (a memória do projeto) e `memory_propose`; só o
`flock_list` com a memória desligada. O Claude Code pergunta com um card antes de cada chamada que as suas regras não permitem, e
uma proposta ainda espera na aba Memory. Essas chamadas não marcam a conversa como tendo lido
conteúdo de fora. O servidor é este app rodando de novo (`vults --zeca-mcp`), e não envia nada a
lugar nenhum. O Codex só lê servidores MCP da própria configuração, então não os recebe.
