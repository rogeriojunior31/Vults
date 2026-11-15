# Decisões
<!-- source: b06f2bf4ef00 -->

Registros de decisão de arquitetura (ADRs): um arquivo por tema que molda o projeto, com a regra em
vigor, o contexto em que foi feita e o que ela custa. O código e os guias dizem *o que* o app faz;
estes registros dizem *por quê*. Planos ficam no [roadmap](../../dev/ROADMAP.md), o que já saiu no
[changelog](../../../CHANGELOG.md); nenhum dos dois pertence aqui.

| Tema | Registro | Status | Incorporados |
|---|---|---|---|
| Stack | [0002](0002-rust-backend-no-sidecars.md) Rust em todo o backend; nenhum sidecar em Python | Aceita | |
| O hook | [0003](0003-never-block-an-agent.md) O hook nunca bloqueia um agente | Aceita | |
| Permissões | [0004](0004-a-human-answers-permissions.md) Quem responde a uma permissão | Aceita | 0014, 0017, 0025, 0026 |
| Configurações dos agentes | [0005](0005-agent-configs-backup-diff-click.md) As configurações dos agentes, e os agentes que suportamos | Aceita | 0029 |
| Ações | [0011](0011-only-actions-that-work.md) Oferecer só as ações que os agentes permitem | Aceita | |
| Zeca | [0018](0018-zeca-as-an-agent.md) Zeca: opcional, separado do bando, regras antes de modelos | Aceita | 0010, 0012, 0031 |
| Rede e segredos | [0019](0019-requests-on-the-users-behalf.md) Rede, segredos e telemetria | Aceita | 0006, 0016 |
| Superfícies | [0021](0021-four-sizes-one-nest.md) Superfícies: um núcleo, quatro tamanhos, um card respondido onde você está | Aceita | 0008, 0009, 0022, 0023, 0024, 0030, 0031 |
| Voz | [0028](0028-spoken-replies-through-a-realtime-model.md) Voz: escuta local, respostas faladas opcionais | Aceita | 0013 |
| Ditado | [0032](0032-dictation-into-other-apps.md) Ditado em outros aplicativos | Aceita | |

Um número da última coluna foi um registro próprio; um commit ou PR antigo que o cita quer dizer o
registro em que ele foi incorporado (a 0031, remoção de Tasks, foi para a 0018 e a 0021). Três
números foram aposentados em 2026-11-15 e só existem no histórico do git: 0001 (como escrever um
registro: as regras abaixo), 0020 (releases: agora em [Releasing](../../dev/releasing.md)) e 0027
(uma resposta aberta para qualquer ferramenta: uma proposta, agora a tarefa G1/G2 do roadmap).
Números nunca são reusados: o próximo tema é o 0033.

<a id="writing-one"></a>

## Como escrever um

Um registro guarda uma regra que sobrevive ao trabalho que a criou. Escreva ou edite um quando uma
escolha muda uma regra do `CLAUDE.md`, a arquitetura ou o que o app promete aos usuários. Um plano,
uma fase, uma meta de release ou uma remoção não é uma decisão: isso vai no roadmap ou no changelog.

- **Um tema, um registro vivo.** Uma decisão nova sobre um tema que já tem registro é uma edição
  desse registro, num PR próprio, com uma linha datada no seu *Histórico*. Só um tema novo ganha
  número novo: copie o formato de um registro existente (**Status**, **Contexto**, **Decisão**,
  **Consequências**, **Histórico**), dê a ele o próximo número e acrescente-o à tabela.
- **O histórico registra decisões, não entregas.** Uma linha datada diz qual regra mudou e por quê.
  Qual PR ou etapa a implementou vai no changelog.
- **Consolide em vez de alterar entre registros.** Registros do mesmo tema são unidos em um; a
  união não decide nada novo, descarta só o que já tinha sido substituído, e todo link passa para o
  registro vivo. O git guarda o texto antigo.
- **Uma regra do `CLAUDE.md` muda primeiro no seu registro**, nunca só no `CLAUDE.md`.
