# Decisões
<!-- source: e7e88c8f3d41 -->

Registros de decisão de arquitetura (ADRs): um arquivo por escolha que molda o projeto, com o
contexto em que foi feita e o que ela custa. O código e os guias dizem *o que* o app faz; estes
registros dizem *por quê*.

| # | Decisão | Status |
|---|---|---|
| [0001](0001-record-decisions.md) | Registrar decisões como ADRs | Aceita |
| [0002](0002-rust-backend-no-sidecars.md) | Rust em todo o backend; nenhum sidecar em Python | Aceita |
| [0003](0003-never-block-an-agent.md) | O hook nunca bloqueia um agente | Aceita |
| [0004](0004-a-human-answers-permissions.md) | Uma permissão é respondida por um humano, ou por uma regra exata que um humano criou | Aceita, emendada pela 0014 |
| [0005](0005-agent-configs-backup-diff-click.md) | As configurações dos agentes só mudam depois de um backup, um diff e um clique | Aceita |
| [0006](0006-keyring-no-telemetry.md) | Segredos só no chaveiro do sistema; sem telemetria | Aceita |
| [0007](0007-versions-to-0.5.md) | Primeira release 0.1.0; um tema por versão até a 0.5 | Substituída pela 0015 |
| [0008](0008-one-core-many-surfaces.md) | Um núcleo, várias superfícies na área de trabalho; a ilha é a única que exibe cards | Aceita |
| [0009](0009-presence-never-hides-a-card.md) | Nenhum modo de presença deixa um card confirmado sem ser visto | Aceita |
| [0010](0010-zeca-is-optional.md) | O Zeca é opcional e fica separado do bando | Aceita |
| [0011](0011-only-actions-that-work.md) | Oferecer só as ações que os agentes permitem | Aceita |
| [0012](0012-rules-before-models.md) | Regras antes de modelos nas decisões do próprio app | Aceita |
| [0013](0013-voice-local-first.md) | A voz é local primeiro; nuvem só quando escolhida | Aceita |
| [0014](0014-consent-before-autonomy.md) | Uma nova regra de consentimento antes de qualquer autonomia | Aceita |
| [0015](0015-everything-by-0.2-in-small-releases.md) | Tudo o que está planejado chega até a 0.2.0, em pequenas releases 0.1.x | Aceita |
| [0016](0016-updates-asked-for-and-signed.md) | Atualizações: pedidas, assinadas, um canal por vez | Aceita |

<a id="writing-one"></a>

## Como escrever um

Copie o formato de um registro existente: **Status**, **Contexto**, **Decisão**, **Consequências**.
Dê a ele o próximo número e acrescente-o à tabela. Um registro nunca é reescrito para dizer outra
coisa: um novo o substitui, e o status do antigo diz qual. Escreva um quando uma escolha muda uma
regra do `CLAUDE.md`, a arquitetura, o plano de releases ou o que o app promete aos usuários.
