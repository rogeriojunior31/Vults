# Decisões
<!-- source: 0153afdcbab6 -->

Registros de decisão de arquitetura (ADRs): um arquivo por tema que molda o projeto, com a regra em
vigor, o contexto em que foi feita e o que ela custa. O código e os guias dizem *o que* o app faz;
estes registros dizem *por quê*.

| Tema | Registro | Status | Incorporados |
|---|---|---|---|
| Como decidimos | [0001](0001-record-decisions.md) Registrar decisões como ADRs | Aceita | |
| Stack | [0002](0002-rust-backend-no-sidecars.md) Rust em todo o backend; nenhum sidecar em Python | Aceita | |
| O hook | [0003](0003-never-block-an-agent.md) O hook nunca bloqueia um agente | Aceita | |
| Permissões | [0004](0004-a-human-answers-permissions.md) Quem responde a uma permissão | Aceita | 0014, 0017, 0025, 0026 |
| Permissões | [0027](0027-an-open-reply-for-any-tool.md) Uma resposta aberta para qualquer ferramenta que peça uma | Proposta | |
| Configurações dos agentes | [0005](0005-agent-configs-backup-diff-click.md) As configurações dos agentes, e os agentes que suportamos | Aceita | 0029 |
| Ações | [0011](0011-only-actions-that-work.md) Oferecer só as ações que os agentes permitem | Aceita | |
| Zeca | [0018](0018-zeca-as-an-agent.md) Zeca: opcional, separado do bando, regras antes de modelos | Aceita | 0010, 0012 |
| Rede e segredos | [0019](0019-requests-on-the-users-behalf.md) Rede, segredos e telemetria | Aceita | 0006, 0016 |
| Releases | [0020](0020-0.2.0-at-wave-4-then-0.2.x.md) Releases: pequenas e frequentes, a 0.2.0 no portão de Operations | Aceita | 0007, 0015 |
| Superfícies | [0021](0021-four-sizes-one-nest.md) Superfícies: um núcleo, quatro tamanhos, um card respondido onde você está | Aceita | 0008, 0009, 0022, 0023, 0024, 0030 |
| Voz | [0028](0028-spoken-replies-through-a-realtime-model.md) Voz: escuta local, respostas faladas opcionais | Aceita | 0013 |

Um número da última coluna foi um registro próprio até 2026-10-12; um commit ou PR antigo que o
cita quer dizer o registro em que ele foi incorporado. Números nunca são reusados: o próximo tema é
o 0031.

<a id="writing-one"></a>

## Como escrever um

Uma decisão sobre um tema que já tem registro é uma edição desse registro, num PR próprio, com uma
linha datada no seu *Histórico*. Só um tema novo ganha número novo: copie o formato de um registro
existente (**Status**, **Contexto**, **Decisão**, **Consequências**), dê a ele o próximo número e
acrescente-o à tabela. Escreva ou edite um quando uma escolha muda uma regra do `CLAUDE.md`, a
arquitetura, o plano de releases ou o que o app promete aos usuários
([0001](0001-record-decisions.md)).
