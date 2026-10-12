# Decisões
<!-- source: a5e6964f0102 -->

Registros de decisão de arquitetura (ADRs): um arquivo por escolha que molda o projeto, com o
contexto em que foi feita e o que ela custa. O código e os guias dizem *o que* o app faz; estes
registros dizem *por quê*.

Os registros de um mesmo tema são consolidados num registro vivo ([0001](0001-record-decisions.md)):
leia esse para saber a regra em vigor. Os registros incorporados a ele continuam como esboços, para
que links antigos ainda cheguem a algum lugar.

| Tema | Registro vivo | Status | Incorporados |
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
| Superfícies | [0021](0021-four-sizes-one-nest.md) Superfícies: um núcleo, quatro tamanhos, um lugar para os cards | Aceita, emendada pela 0030 | 0008, 0009, 0022, 0023, 0024 |
| Superfícies | [0030](0030-a-card-is-answered-where-you-are.md) Um card é respondido onde você está | Aceita | |
| Voz | [0028](0028-spoken-replies-through-a-realtime-model.md) Voz: escuta local, respostas faladas opcionais | Aceita | 0013 |

<a id="merged-records"></a>

## Registros incorporados

| # | Decisão | Incorporado em |
|---|---|---|
| [0006](0006-keyring-no-telemetry.md) | Segredos só no chaveiro do sistema; sem telemetria | [0019](0019-requests-on-the-users-behalf.md) |
| [0007](0007-versions-to-0.5.md) | Primeira release 0.1.0; um tema por versão até a 0.5 | [0020](0020-0.2.0-at-wave-4-then-0.2.x.md) |
| [0008](0008-one-core-many-surfaces.md) | Um núcleo, várias superfícies na área de trabalho; a ilha é a única que exibe cards | [0021](0021-four-sizes-one-nest.md) |
| [0009](0009-presence-never-hides-a-card.md) | Nenhum modo de presença deixa um card confirmado sem ser visto | [0021](0021-four-sizes-one-nest.md) |
| [0010](0010-zeca-is-optional.md) | O Zeca é opcional e fica separado do bando | [0018](0018-zeca-as-an-agent.md) |
| [0012](0012-rules-before-models.md) | Regras antes de modelos nas decisões do próprio app | [0018](0018-zeca-as-an-agent.md) |
| [0013](0013-voice-local-first.md) | A voz é local primeiro; nuvem só quando escolhida | [0028](0028-spoken-replies-through-a-realtime-model.md) |
| [0014](0014-consent-before-autonomy.md) | Uma nova regra de consentimento antes de qualquer autonomia | [0004](0004-a-human-answers-permissions.md) |
| [0015](0015-everything-by-0.2-in-small-releases.md) | Tudo o que está planejado chega até a 0.2.0, em pequenas releases 0.1.x | [0020](0020-0.2.0-at-wave-4-then-0.2.x.md) |
| [0016](0016-updates-asked-for-and-signed.md) | Atualizações: pedidas, assinadas, um canal por vez | [0019](0019-requests-on-the-users-behalf.md) |
| [0017](0017-a-signed-phone-decision-is-a-click.md) | Uma decisão assinada num celular pareado vale como um clique | [0004](0004-a-human-answers-permissions.md) |
| [0022](0022-settings-stays-a-window.md) | As Configurações continuam uma janela própria | [0021](0021-four-sizes-one-nest.md) |
| [0023](0023-settings-a-webview-of-the-nest.md) | As Configurações, um webview da janela do Nest | [0021](0021-four-sizes-one-nest.md) |
| [0024](0024-one-surface-at-a-time.md) | Uma superfície por vez: a ilha sai para o Nest | [0021](0021-four-sizes-one-nest.md) |
| [0025](0025-destructive-calls-always-ask.md) | Uma chamada destrutiva sempre pergunta, mesmo com uma regra Always | [0004](0004-a-human-answers-permissions.md) |
| [0026](0026-the-chats-access-is-the-users-choice.md) | Quem responde os cards do chat é escolha do usuário | [0004](0004-a-human-answers-permissions.md) |
| [0029](0029-the-supported-agents.md) | Os agentes suportados, e como aposentar um | [0005](0005-agent-configs-backup-diff-click.md) |

<a id="writing-one"></a>

## Como escrever um

Uma decisão sobre um tema que já tem registro vivo é uma edição desse registro, num PR próprio,
com uma linha datada no seu *Histórico*. Só um tema novo ganha número novo: copie o formato de um
registro existente (**Status**, **Contexto**, **Decisão**, **Consequências**), dê a ele o próximo
número e acrescente-o à tabela. Escreva ou edite um quando uma escolha muda uma regra do
`CLAUDE.md`, a arquitetura, o plano de releases ou o que o app promete aos usuários. Uma
consolidação incorpora os registros de um tema ao registro vivo dele, não decide nada novo e deixa
cada registro incorporado como um esboço ([0001](0001-record-decisions.md)).
