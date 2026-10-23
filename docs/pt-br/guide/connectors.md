# Conectores
<!-- source: f012e5f747fa -->

No Nest, Connections mostra uma lista de triagem ao lado do detalhe selecionado. Marcas de
repositório identificam as linhas, o detalhe mostra a barra de alterações e cada check, e
**Refresh** fica ao lado do estado de sincronização. Os checks começam abertos; sua escolha de
expandir ou recolher é mantida nas atualizações. Janelas estreitas abrem o detalhe separado e
voltam à linha selecionada. O merge continua exigindo sua própria confirmação.

Os conectores trazem novidades de serviços externos para a ilha. Cada um fica desligado até você
ligá-lo em **Settings → Connectors**.

## GitHub

Ele verifica a cada cinco minutos, e a cada minuto enquanto houver checks de CI rodando:

- seus pull requests abertos: checks que falharam ou passaram, aprovado, mudanças solicitadas;
- pull requests em que sua revisão foi solicitada;
- os checks na branch padrão dos dez repositórios seus (de que você é dono) com push mais recente,
  e a release mais recente de cada um.

A mesma consulta única também traz o que a aba **Connections** do Room mostra de cada linha: o branch
de um pull request, o tamanho, os revisores e a última atualização, e cada check pelo nome
([O Nest](nest.md#connections)). As releases só aparecem lá: nunca viram alerta, nunca vão para a ilha.

Ele usa o GitHub CLI em que você já está logado (`gh auth login`), então o Vults nunca vê um token.
A primeira verificação só aprende como as coisas estão; os alertas começam na próxima mudança.
Clique em um alerta para abri-lo no GitHub, ou no × para dispensá-lo. Checks que passam substituem
uma falha, e uma decisão de revisão mais nova substitui a mais antiga.

Depois de um erro ele tenta de novo em 15 minutos se for um limite de requisições do GitHub, em 10
quando o `gh` não está instalado ou está deslogado, e de 4 a 15 (dobrando) nos outros casos. Abrir a
ilha ou o card verifica antes quando a última verificação tem mais de um minuto, exceto sob um
limite de requisições. O GitHub é verificado no máximo 3.000 vezes por dia; passado isso,
**Settings → Connectors** mostra **Budget spent**. Se o GitHub responder com dados parciais (por
exemplo, porque um repositório exige login via SAML), o Vults mantém o último snapshot completo e
mostra um erro. Campos ausentes não são tratados como prova de que um pull request ou repositório
não existe.

<a id="the-github-card"></a>

### O card do GitHub

Depois que o GitHub responde, o **Now** do Quick reúne as linhas que pedem uma ação. Uma linha
abre seu detalhe selecionado nas **Connections** do Nest; **Open in GitHub** ali abre seu link.
As Connections mantêm a lista completa, inclusive linhas mais tranquilas e releases:

![O card do GitHub: seus pull requests, uma revisão esperando, as branches padrão e um check que falhou embaixo](../../assets/island-github-card.png)

- **Your pull requests** (seus pull requests), do atualizado mais recentemente para o mais antigo:
  um ponto para os checks (verde passando, âmbar rodando, vermelho falhando) e a revisão
  (*Approved* ou *Changes requested*);
- **Waiting for your review** (esperando sua revisão);
- **Default branches** (branches padrão): o último commit em cada uma, com o resultado dos checks.
  Repositórios sem checks ficam de fora.

Selecione uma linha para ver o detalhe e escolha **Open in GitHub** para seguir o link. Quando um pull request é mergeado ou fechado, ou uma
solicitação de revisão é retirada, os alertas dele vão junto.

Com o Zeca ligado, uma linha pode perguntar a ele sobre ela:

- **Review with Zeca** (revisar com o Zeca), numa revisão que espera por você: ele lê o diff e diz
  os riscos antes de você aprovar;
- **Ask Zeca why** (perguntar ao Zeca por quê), em checks falhando: ele lê só o log da etapa que
  falhou e diz o que quebrou e onde.

Cada um abre o chat com a pergunta escrita no campo de texto. Nada é perguntado até você enviar,
então dá para mudá-la antes.

**Squash and merge** aparece no seu próprio pull request quando ele está aprovado e com os checks
passando. O primeiro clique só pergunta, embaixo da linha, o que o segundo vai fazer; **Merge** então
faz o squash na branch base e apaga a branch dele, pelo seu login do `gh`, e o card pergunta ao GitHub de
novo, então ela sai com a próxima resposta. Só o commit que o card mostrou verde é mergeado: se
alguém deu push depois, o GitHub recusa. Se o GitHub recusar (uma regra de branch, um conflito), a linha diz por quê.
Cada merge escreve uma linha no rastro local, e nada é mergeado por conta própria.

**Seen** nas notícias tranquilas do Now oculta aquela linha até os checks, a revisão ou o título
mudarem. **Mark all seen** faz isso para todas as notícias tranquilas das conexões no Now.
Solicitações e falhas continuam visíveis; marcar como visto nunca aprova, faz merge nem muda o GitHub.
As Connections do Nest mantêm as linhas. As marcações ficam somente neste computador.

Uma consulta do conector que falha mantém as últimas linhas carregadas e a informação de sincronização
ou erro nas Connections. Um card de permissão fica na frente do Quick até ser respondido ou
explicitamente adiado; o Now abre depois.

O que a última verificação viu (os títulos, links e estados de checks que o card mostra, e nada
mais) fica guardado em `connectors/github.json` na pasta de dados do app (`~/.local/share/vults/`
no Linux), para que uma reinicialização não gere alertas de novo sobre novidades antigas.


<a id="for-zeca"></a>
### Para o Zeca

Connections tem as telas **Para você** e **Para o Zeca**. Alternar entre elas mantém o pull
request selecionado e os filtros. **Para o Zeca** controla as ferramentas GitHub integradas.
`github_snapshot` lê o card da última sincronização sem fazer uma requisição e está disponível no
Chat API e na voz em tempo real. Nos provedores compatíveis do Chat API, `github_account` identifica a
conta e o host selecionados no `gh`, `github_search_repositories` encontra repositórios visíveis para
essa conta e `github_repository` lê os metadados de um repositório acessível pelo nome exato.
`github_work_item` lê uma issue ou um pull request pelo repositório e número exatos. Pull requests incluem páginas limitadas de arquivos,
revisões recentes, tópicos de revisão em linha, verificações e dados de prontidão para merge.
Ferramentas separadas leem o diff de um pull request, execuções, tarefas e logs do Actions com
segredos ocultos, ou as regras ativas de uma branch. Cada operação busca apenas o próprio contexto.
As leituras ao vivo do GitHub compartilham um controle que começa desligado e exige que o conector
GitHub esteja ligado. As duas escolhas ficam salvas neste computador e são verificadas em cada
chamada, inclusive em conversas já abertas. Se salvar falhar, a escolha anterior é restaurada.
Nenhuma escolha ativa o conector GitHub silenciosamente.

O snapshot em cache não faz requisições ao GitHub. Cada leitura ao vivo passa pelo gate de rede
compartilhado e deixa um rastro local. A busca retorna no máximo dez repositórios por página com
cursor. A discussão retorna cinco comentários por página; arquivos alterados, revisões, tópicos
de revisão e verificações têm cursores e páginas limitadas independentes. Execuções e tarefas do
Actions retornam dez itens por página; regras de branch retornam no máximo 100. Diffs e logs de
tarefas são leituras separadas, limitadas a 48 KiB de texto retornado; os segredos nos logs são
ocultos. Os resultados incluem repositório, URL de origem quando fornecida pelo GitHub, horário
de leitura e limites explícitos de página ou bytes. Uma resposta GraphQL parcial ou uma página
malformada gera erro em vez de parecer completa. Cada página consome outra requisição do
orçamento. A tela mostra chamadas registradas nas últimas 24 horas e o último uso registrado;
**Abrir rastros** abre os rastros locais em Atividade. A ausência de registros aparece
explicitamente. Leituras ao vivo e consultas do conector compartilham um limite de 3.000
requisições a cada 24 horas. O acesso a comandos da CLI é controlado separadamente no Chat. O
Chat reduz diffs e logs grandes para preservar o envelope JSON válido de 16 KiB e sinaliza a
cobertura parcial; outros contextos grandes falham explicitamente. Os logs verificam se o job
pertence à tentativa exata da execução; a busca do job e o download do log passam separadamente
pelo controle de rede.

As ferramentas completas de leitura ao vivo são oferecidas somente no Chat API do Zeca com
Anthropic, OpenAI, Google Gemini, OpenRouter, Groq, DeepSeek, Mistral e xAI. Ollama e LM Studio
locais, chats pela CLI e chats com especialistas não recebem essas ferramentas. `source.mode` em
cada resultado distingue `live` de `cached_snapshot`; Atividade registra o mesmo modo junto com
os argumentos. Uma leitura ao vivo bem-sucedida marca a conversa como conteúdo externo, para que
a política existente possa restaurar os cards.

Nos provedores compatíveis do Chat API, Zeca pode propor criar ou editar uma issue, publicar um
comentário ou editar um comentário existente, adicionar ou remover labels, substituir
responsáveis e fechar ou reabrir uma issue. Cada operação é uma ferramenta distinta e abre o
cartão de aprovação existente com o repositório, a issue, a operação e o conteúdo exato. O
cartão não pode ser salvo como uma escolha **Sempre**. Antes do clique, Vults lê a conta ativa,
o host e o destino atual e verifica a permissão de escrita no repositório; depois do clique,
repete as verificações e recusa destinos desatualizados ou alterações na solicitação aprovada. A
aprovação inclui os IDs imutáveis da conta, do repositório, da issue e do comentário. Essas
ferramentas não alteram pull requests. As escritas exigem acesso push ao repositório e um
repositório não arquivado. A busca de duplicatas lê até 1.000 issues ou comentários; se a
cobertura for incompleta, faça a alteração no GitHub.

Após uma escrita, Vults lê o destino novamente e só informa um resultado verificado. Se uma
solicitação falhar ou expirar, lê o destino e nunca repete a escrita às cegas. Comentários já
publicados e outras alterações duplicadas exatas são recusados. Cada tentativa aprovada registra
o hash da solicitação e o resultado no log de auditoria local; cancelar registra um resultado
incerto. Essas operações usam os mesmos controles desligados por padrão de GitHub ao vivo,
conexão, orçamento de requisições e portão de rede das leituras. A ação **Merge**, com
confirmação, continua disponível no seu pull request em **Para você**.

<a id="pull-request-changes"></a>
### Alterações em pull requests

O mesmo controle de GitHub ao vivo também habilita propostas tipadas nos provedores compatíveis
do Chat API:

- `github_pull_request_create` cria um PR ou draft entre duas branches existentes do mesmo
  repositório, com título, corpo e escolha de draft exatos.
- `github_pull_request_edit` altera o título, o corpo ou a branch base.
- `github_pull_request_ready` marca um draft como pronto para revisão; isso pode solicitar
  revisão dos responsáveis definidos no CODEOWNERS.
- `github_pull_request_draft` converte um PR aberto novamente em draft.
- `github_pull_request_request_reviewers` solicita revisão aos logins e identificadores de
  equipes exatos mostrados no cartão. Os destinatários recebem notificações.
- `github_pull_request_review_comment`, `github_pull_request_review_approve` e
  `github_pull_request_review_request_changes` enviam decisões distintas de revisão com o
  corpo exato aprovado e comentários em linha opcionais. Aprovar um rascunho na conversa nunca
  envia uma revisão; cada decisão exige seu próprio cartão humano.

O cartão mostra repositório, PR de destino, refs base/head e SHAs dos commits atuais, conteúdo
exato e destinatários. O serviço confere novamente a conta ativa, a permissão no repositório,
a identidade do destino, as refs e o contexto da revisão imediatamente antes de executar.
As revisões enviam o SHA aprovado de head em `commit_id`. Comentários em linha exigem um
arquivo alterado atual, lado `LEFT` ou `RIGHT` e uma linha existente no patch de texto atual.
Linhas desatualizadas, patches indisponíveis e comentários à direita em arquivos removidos
são recusados.

Alterações em PRs existentes exigem um PR aberto com head no mesmo repositório. Heads de forks
são recusados; criar branches ou enviar commits não faz parte dessas operações. Uma solicitação
pode conter até dez comentários em linha, cada um em uma única linha do diff. Cada coleção de
contexto ou duplicatas percorre até 1.000 itens; cobertura incompleta recusa a operação. Uma
revisão pendente existente deve ser concluída ou descartada no GitHub primeiro, para que seu
conteúdo nunca seja reaproveitado silenciosamente.

Toda alteração usa o cartão humano existente, o controle de rede por requisição e a auditoria.
O estado remoto é lido novamente após executar, inclusive após timeout ou erro de requisição,
sem repetir a escrita às cegas. Se as refs ou o estado remoto mudarem durante uma solicitação,
o resultado pode ficar incerto e deve ser conferido antes de tentar novamente. A leitura final
verifica o resultado; ela não torna atômicos os endpoints do GitHub baseados em branches.
Propostas de merge usam os controles separados descritos abaixo.

<a id="actions-controls"></a>
### Controles do Actions

Nos provedores compatíveis do Chat por API, `github_actions_run` lê a execução mais recente ou uma
tentativa histórica exata quando `attempt` é informado. Execuções e trabalhos distinguem as fases
na fila, em execução, aguardando, cancelada e concluída. Os logs ocultam segredos, têm limite de
48 KiB com indicação explícita de truncamento e respeitam também o limite de 16 KiB dos resultados
do Chat. Logs indisponíveis são informados como indisponíveis, nunca como um log vazio bem-sucedido.

Cada controle exige seu próprio cartão humano com repositório, execução, tentativa mais recente e commit:

- `github_actions_rerun_failed` repete os trabalhos com falha e seus dependentes.
- `github_actions_rerun_run` repete todo o workflow, incluindo os trabalhos que já passaram.
- `github_actions_rerun_job` repete um trabalho com falha selecionado e seus dependentes; exige `jobId`.
- `github_actions_cancel` solicita o cancelamento normal de uma tentativa ativa.

Reexecuções podem consumir minutos do Actions. Logs de depuração ficam desativados. A API não enumera
os IDs dos dependentes; o cartão inclui explicitamente os trabalhos dependentes no escopo. Essas
ferramentas nunca forçam cancelamentos nem aprovam implantações. Conta, identidade imutável do
repositório, tentativa, commit e escopo são conferidos novamente após a aprovação. É necessário
acesso de push ao repositório; o GitHub ainda pode rejeitar um token sem permissão de escrita no Actions.

Uma solicitação aceita pode ainda estar pendente. Os resultados distinguem `verified`,
`accepted_pending_verification`, `rejected`, `uncertain` e `completed_without_cancellation`, com a
fase observada. Uma reexecução verificada significa que uma nova tentativa foi observada, não que
seus trabalhos passaram. Um cancelamento só é verificado quando aquela tentativa exata é observada
como cancelada. Cada requisição usa os controles existentes do conector e das ferramentas ao vivo,
o orçamento e o gate de rede.

Antes de enviar um controle, o Vults grava atomicamente um recibo durável na auditoria local.
As variantes de reexecução compartilham um recibo por host, repositório imutável, execução e tentativa;
o cancelamento usa um recibo separado. As reservas sobrevivem ao reinício. Controles aceitos,
reservados ou incertos não podem ser reenviados naquela tentativa. Uma rejeição definitiva permite
uma nova proposta com aprovação humana. O Vults relê o estado sem repetir o POST. Se a verificação
estiver pendente ou indisponível, consulte a execução mais recente no GitHub em vez de reenviar.

<a id="safe-merge-and-branch-deletion"></a>
### Merge seguro e exclusão de branch

Nos provedores compatíveis do Chat por API, `github_pull_request_merge` propõe um merge com
repositório, número do PR, `headSha` exato de 40 dígitos minúsculos e `method` (`merge`, `squash`
ou `rebase`). O cartão humano existente mostra conta, refs, commits, checks, decisão de review e
regras aplicáveis. Ele não pode virar uma regra Sempre. Após o clique, o Vults relê conta,
permissões, commits de head/base, prontidão, rulesets ativos e proteção clássica. Uma mudança exige
novo cartão. Checks pendentes ou com falha, reviews ausentes, discussões abertas, conflitos,
prontidão desconhecida ou cobertura incompleta impedem a proposta. Heads de forks e PRs empilhados
não são suportados. Checks e discussões têm limite de 100 entradas cada; regras ativas, de 1.000.
A falta de permissão para inspecionar proteções impede a operação.

Os merges usam a API assíncrona do GitHub (versão `2026-03-10`) com o SHA aprovado e
`bypass_rules: false`. Servidores antigos sem essa API não são suportados; não há fallback para
merge sem verificações nem bypass administrativo. As regras são aplicadas remotamente mesmo se
mudarem após a última inspeção. Uma fila obrigatória usa seu método configurado; escolha esse método
no cartão. Uma entrada na fila é informada como **na fila**, nunca como mesclada. A aceitação
assíncrona fica **pendente** até ser observada. Um merge verificado exige reler o PR/head exato e
seu commit de merge. As APIs de ref base e exclusão de branch não fornecem uma transação com a
inspeção anterior; a releitura pode informar incerteza se o estado remoto mudar simultaneamente.

`github_pull_request_merge_status` lê o estado atual sem escrever. Informe o UUID opcional
`requestId` de uma solicitação assíncrona para consultar também seu resultado pendente ou com falha.
Pedidos pendentes, na fila e incertos nunca são reenviados automaticamente. Recibos duráveis na
auditoria bloqueiam merges duplicados para o mesmo ID de repositório, PR e SHA, entre métodos e
reinícios. Só uma rejeição definitiva permite nova proposta; uma reserva ou recibo incerto exige
inspeção no GitHub em vez de tentar novamente às cegas. Não há consultas em segundo plano.

`github_pull_request_delete_branch` é uma operação separada com aprovação humana após um merge
verificado. Exige o SHA exato do PR mesclado. O Vults só apaga sua branch head inalterada no mesmo
repositório, excluindo branches padrão/base, protegidas ou com regras ativas. Confere antes de
apagar e verifica depois que a ref exata está ausente. O GitHub não oferece comparação e exclusão
atômicas de refs; se essa branch puder receber pushes simultâneos, mantenha-a e gerencie a limpeza
no GitHub. O merge nunca inclui exclusão de branch. As configurações do repositório podem excluir
branches automaticamente após o merge de forma independente.

A ação existente **Squash e merge** compartilha serviço verificado, gate e recibo durável. Sua
confirmação mantém a branch e distingue mesclado, na fila e pendente. Os controles do Chat usam
os controles existentes de GitHub ao vivo e do conector, orçamento por requisição e trace.
Nenhum controle altera regras, atualiza a branch base ou ignora uma proteção.

Quer outro serviço? Veja [Como adicionar um conector](../contributing/connectors.md).
