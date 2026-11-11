# Paridade do GitLab e Gitea
<!-- source: 6b245a9586ed -->

Solicitado em 2026-11-10. O GitHub é a referência de comportamento. Entregar os recursos nativos
equivalentes com a mesma confirmação, escopo, auditoria e tratamento de falhas. Não anunciar recursos
pendentes. O Notion vem depois desse trabalho, sem substituí-lo. Manter cobertura do GitHub e Linux.

<a id="delivery-sequence"></a>
## Sequência de entrega

| Etapa | GitLab | Gitea | Aceitação |
| --- | --- | --- | --- |
| CP1: painel completo | Implementado com limites documentados | Implementado com limites documentados | Tamanho dos diffs, revisores, pedidos de alteração, jobs/checks do mesmo commit, branches padrão, releases, sem notícias repetidas |
| CP2: snapshot no chat | Pendente | Pendente | Permissões independentes em Settings/perfil, leitura do cache no API Chat e realtime, sem nova consulta de rede |
| CP3: leituras ao vivo | Pendente | Pendente | Conta, repositório/busca, issues/PRs, comentários/reviews, diff/arquivos, execuções/jobs/logs de CI e regras de branch; paginação explícita e limitada |
| CP4: escritas em issues | Pendente | Pendente | Criar/editar, comentários e suas edições, rótulos, responsáveis, fechar/reabrir; prévia humana, revalidação e leitura de confirmação auditada |
| CP5: escritas em pull requests | Pendente | Pendente | Criar/editar, rascunho/pronto, solicitar revisores, reviews e comentários em linhas vinculados ao commit revisado |
| CP6: controles de CI | Pendente | Pendente | Repetir jobs com falha/execução/job e cancelar quando suportado; vínculo exato de projeto/execução/commit e proteção persistente contra duplicações |
| CP7: controles de merge | Pendente | Pendente | Confirmar o commit exato, rever permissões/reviews/regras/discussões/CI, sem ignorar proteções; confirmar estado e excluir branch separadamente |
| CP8: aceitação | Pendente | Pendente | Interface nativa no Windows, CI do Linux, API simulada e aceitação com contas/permissões reais quando disponíveis |

<a id="invariants"></a>
## Invariantes

- Cada consulta passa pela opção independente do conector, bloqueio/pausa, limite e rastro. Operações
  com várias consultas autorizam cada uma separadamente, incluindo validação e confirmação.
- Credenciais ficam na CLI ou no cofre do sistema. Ações do Gitea vinculam instância, geração da
  credencial, conta, identidade do repositório e objeto/commit; reconectar invalida prévias antigas.
- Escritas remotas exigem uma ação humana revisada. Recibos persistentes distinguem recusa, aceitação,
  verificação e incerteza; timeout nunca provoca reenvio automático.
- CI aprovado sozinho não libera merge na interface. As proteções do servidor são exigidas na
  execução. Regras ou checks obrigatórios ausentes/inacessíveis recusam a ação, sem alternativa administrativa.
- Diferenças entre serviços ficam explícitas. Pipelines do GitLab e Actions do Gitea mantêm seus
  significados. Estados desconhecidos não viram sucesso; releases futuras não viram rascunhos.
- Respostas parciais preservam o último painel completo. Alterações de formato estabelecem nova referência uma vez.

<a id="current-evidence"></a>
## Evidências atuais

A consulta CP1 do GitLab foi validada sem credenciais no schema público do GitLab.com em 2026-11-10.
A complexidade medida fica abaixo do limite sem autenticação; não houve conta pessoal nem escrita.
Isso valida o formato da consulta, não a aceitação dos recursos autenticados.

O app em execução no Windows foi validado com uma CLI simulada local: desligado não executa a CLI,
somente Settings ativa, uma consulta limitada produz linhas de MR/branch padrão/release e o merge
continua desabilitado. Limites e pontos de clique foram verificados em escalas emuladas de 100%, 125%,
150% e 200%. Em janelas baixas, o detalhe mantém espaço rolável; convites para conectores desligados
não espremem mais o painel ativo. A tela vazia e Gerenciar conexões continuam oferecendo a configuração.

Simulações locais do Gitea cobrem leituras completas de PRs/reviews/status/Actions/branches/releases,
autorização por requisição, enriquecimento sem duplicação, IDs de conta inválidos, commits/tentativas
divergentes, ausência de cobertura do Actions e páginas incompletas. Reviews usam a hora de envio.
A frequência das consultas considera a quantidade de requisições. Simulações no WebView nativo
cobrem abas Actions/Releases e pontos de clique dos jobs com falha em 100%, 125%, 150% e 200%,
incluindo uma área de 480 por 360.

O Gitea ainda precisa de uma instância real de teste. Simulações e escalas emuladas da interface não
substituem permissões reais, DPI/entrada físicos do Windows nem aceitação das regras de merge ao vivo.
