# 0031. Tasks sai do Vults
<!-- source: 2ff0f3aaffcd -->

**Status:** Aceita, 2026-10-16. Implementa a ordem de trabalho do dono no
[Plan Zeca](../../dev/plan-zeca.md#order-of-work-2026-10-16).
Numeração das releases alterada em 2026-10-26 pela [0020](0020-0.2.0-at-wave-4-then-0.2.x.md).

<a id="context"></a>

## Contexto

Tasks ganhou seu próprio sistema de execução: objetivos, clones isolados, pacotes de contexto,
ciclos de revisão e escolha de agentes. O dono decidiu remover esse recurso e concentrar o Vults
no bando e no Chat. O antigo portão de release da onda 4 exigia Tasks, então remover o recurso
também muda quando a 0.2.0 sai.

Este registro define o contrato da N0. Sua implementação chega em etapas separadas; aceitar a
decisão não significa que o código já foi removido ou que o teste no desktop passou.

<a id="decision"></a>

## Decisão

- **Remover Tasks e tudo que só ele usa.** Não ficam a aba Tasks, a ação Open Tasks, `/task`,
  `/delegate`, os comandos nem os eventos de tarefas. Saem a execução em clones isolados, a
  passagem por pacote de contexto, os ciclos de revisão, a escolha de agentes, suas leituras e
  escritas e seus testes e evals exclusivos. Saem `VULTS_TASK` e o estado de decolagem que ele
  aciona, com a nota de protocolo necessária. O template de issue do GitHub chamado Task fica:
  ele descreve issues.
- **Manter `/continue` (W7).** Seu driver sai da execução de tarefas para um módulo próprio.
  Ele mantém seu próprio canal; `VULTS_ZECA` continua separando os eventos do Chat do Zeca das
  sessões do bando. D8 continua valendo: o Vults só para uma execução que ele iniciou, agora por
  `/continue`; observar uma sessão nunca concede controle sobre seu processo. Não se oferece
  Stop sem um driver que o suporte.
- **Manter os gastos e seu orçamento (W9).** Activity > Cost fica com os números; as
  Configurações, com o valor do orçamento. Os gastos já registrados com origem `task:N` continuam
  legíveis e entram nos mesmos totais. As migrações 14 a 20 são histórico já distribuído: não
  devem ser editadas. Suas tabelas de tarefas ficam sem leitores nem escritores; uma migração
  posterior pode apagá-las depois de uma release.
- **Superfícies e D7:** o Nest tem Flock, Chat, Connections e Activity, com as Configurações em
  sua própria webview. Com o Zeca desligado, tudo exceto o Chat e os recursos próprios do Zeca
  continua funcionando, incluindo registros, custos e orçamento. Memory continua ao lado do
  Chat. Remover Tasks não concede a Zeca, a um agente ou a uma superfície nenhum novo poder de
  responder permissões ([0004](0004-a-human-answers-permissions.md)).
- **Um PR por etapa da N0**, cada um passando toda a CI: decisão, superfícies, app, crates,
  protocolo/core e, por fim, guias e referências. Preservar o comportamento mantido em cada
  etapa. Remover um ponto de entrada antes da implementação não autoriza essa implementação
  por outra superfície.
- **A numeração das releases segue a [0020](0020-0.2.0-at-wave-4-then-0.2.x.md).** Sua alteração
  de 2026-10-26 escolhe a 0.1.9 a partir da main atual e substitui a relação fixa entre N0 a N4 e
  versões. O portão da onda 4 continua substituído; releases pequenas e frequentes e Linux primeiro
  continuam sendo a regra.
- **R3 continua sendo um teste humano no app real, no desktop, antes de cada tag de release.**
  Ele cobre a ilha, um card pendente, o Chat, a voz, o GitHub e o Nest na interface refinada pela
  N1. Conferir as áreas restantes do Nest, `/continue` e seu Stop de execução própria, gastos e
  orçamento e o funcionamento com Zeca desligado. Tasks, execução isolada de tarefas, revisão e
  `/delegate` deixam de ser requisitos desse teste. A aceitação nativa e os testes automatizados
  continuam como evidências separadas; nenhum substitui o R3.

<a id="consequences"></a>

## Consequências

- O Vults deixa de ter um ciclo interno de tarefas ou de escolher um agente para `/delegate`.
  Código continua sendo trabalho dos agentes. O harness de API/modelo local não ganha execução
  de código como substituto; o Chat via CLI e `/continue` mantêm seus limites de permissão.
- Os bancos existentes e o histórico de gastos sobrevivem à remoção. A limpeza das tabelas de
  tarefas sem uso fica para depois desta release, em vez de reescrever migrações distribuídas.
- [0018](0018-zeca-as-an-agent.md), [0020](0020-0.2.0-at-wave-4-then-0.2.x.md) e
  [0021](0021-four-sizes-one-nest.md) mantêm as regras atualizadas no próprio registro. Plan Zeca
  preserva os ids de passos concluídos como histórico e acompanha a remoção da N0 separadamente
  da aceitação da release.

<a id="history"></a>

## Histórico

- 2026-10-26, REL-1: a numeração das releases segue a decisão de estabilização 0.1.9 da ADR 0020.
  O contrato de remoção de Tasks e os requisitos de R3 humano permanecem iguais.
