# Decisões
<!-- source: deab022c9004 -->

Registros de decisão de arquitetura (ADRs): um arquivo por escolha que molda o projeto, com o
contexto em que foi feita e o que ela custa. O código e os guias dizem *o que* o app faz; estes
registros dizem *por quê*.

| # | Decisão | Status |
|---|---|---|
| [0001](0001-record-decisions.md) | Registrar decisões como ADRs | Aceita |
| [0002](0002-rust-backend-no-sidecars.md) | Rust em todo o backend; nenhum sidecar em Python | Aceita |
| [0003](0003-never-block-an-agent.md) | O hook nunca bloqueia um agente | Aceita |
| [0004](0004-a-human-answers-permissions.md) | Uma permissão é respondida por um humano, ou por uma regra exata que um humano criou | Aceita, emendada pela 0014, pela 0017, pela 0025 e pela 0026 |
| [0005](0005-agent-configs-backup-diff-click.md) | As configurações dos agentes só mudam depois de um backup, um diff e um clique | Aceita |
| [0006](0006-keyring-no-telemetry.md) | Segredos só no chaveiro do sistema; sem telemetria | Aceita, emendada pela 0016 e pela 0019 |
| [0007](0007-versions-to-0.5.md) | Primeira release 0.1.0; um tema por versão até a 0.5 | Substituída pela 0015 |
| [0008](0008-one-core-many-surfaces.md) | Um núcleo, várias superfícies na área de trabalho; a ilha é a única que exibe cards | Aceita, emendada pela 0017, pela 0021, pela 0023 e pela 0024; o lugar dos cards foi substituído pela 0030 |
| [0009](0009-presence-never-hides-a-card.md) | Nenhum modo de presença deixa um card confirmado sem ser visto | Aceita |
| [0010](0010-zeca-is-optional.md) | O Zeca é opcional e fica separado do bando | Aceita |
| [0011](0011-only-actions-that-work.md) | Oferecer só as ações que os agentes permitem | Aceita |
| [0012](0012-rules-before-models.md) | Regras antes de modelos nas decisões do próprio app | Aceita |
| [0013](0013-voice-local-first.md) | A voz é local primeiro; nuvem só quando escolhida | Aceita; respostas faladas e transcrição na nuvem substituídas pela 0028 |
| [0014](0014-consent-before-autonomy.md) | Uma nova regra de consentimento antes de qualquer autonomia | Aceita, emendada pela 0025 e pela 0026 |
| [0015](0015-everything-by-0.2-in-small-releases.md) | Tudo o que está planejado chega até a 0.2.0, em pequenas releases 0.1.x | Substituída pela 0020 |
| [0016](0016-updates-asked-for-and-signed.md) | Atualizações: pedidas, assinadas, um canal por vez | Aceita |
| [0017](0017-a-signed-phone-decision-is-a-click.md) | Uma decisão assinada num celular pareado vale como um clique | Aceita |
| [0018](0018-zeca-as-an-agent.md) | Zeca como agente: um cérebro em Connect que nunca responde permissão | Aceita |
| [0019](0019-requests-on-the-users-behalf.md) | Requisições em nome do usuário: cada uma ligada, com orçamento e registrada | Aceita |
| [0020](0020-0.2.0-at-wave-4-then-0.2.x.md) | A 0.2.0 no portão de Operations; o companheiro, o celular e o resto em 0.2.x | Aceita, emendada pela 0021 |
| [0021](0021-four-sizes-one-nest.md) | Quatro tamanhos de atenção, um Nest com cinco abas | Aceita, emendada pela 0023, pela 0024 e pela 0030 |
| [0022](0022-settings-stays-a-window.md) | As Configurações continuam uma janela própria | Substituída pela 0023 |
| [0023](0023-settings-a-webview-of-the-nest.md) | As Configurações, um webview da janela do Nest | Aceita |
| [0024](0024-one-surface-at-a-time.md) | Uma superfície por vez: a ilha sai para o Nest | Aceita, emendada pela 0030 |
| [0025](0025-destructive-calls-always-ask.md) | Uma chamada destrutiva sempre pergunta, mesmo com uma regra Always | Aceita |
| [0026](0026-the-chats-access-is-the-users-choice.md) | Quem responde os cards do chat é escolha do usuário | Aceita |
| [0027](0027-an-open-reply-for-any-tool.md) | Uma resposta aberta para qualquer ferramenta que peça uma | Proposta |
| [0028](0028-spoken-replies-through-a-realtime-model.md) | Respostas faladas por um modelo em tempo real, opcional | Aceita |
| [0029](0029-the-supported-agents.md) | Os agentes suportados, e como aposentar um | Aceita |
| [0030](0030-a-card-is-answered-where-you-are.md) | Um card é respondido onde você está | Aceita |

<a id="writing-one"></a>

## Como escrever um

Copie o formato de um registro existente: **Status**, **Contexto**, **Decisão**, **Consequências**.
Dê a ele o próximo número e acrescente-o à tabela. Um registro nunca é reescrito para dizer outra
coisa: um novo o substitui, e o status do antigo diz qual. Escreva um quando uma escolha muda uma
regra do `CLAUDE.md`, a arquitetura, o plano de releases ou o que o app promete aos usuários.
