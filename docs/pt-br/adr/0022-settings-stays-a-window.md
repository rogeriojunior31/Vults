# 0022. As Configurações continuam uma janela própria
<!-- source: 053699fb970b -->

**Status:** Aceita, 2026-10-12. Emenda a [0021](0021-four-sizes-one-nest.md). Substituída pela
[0023](0023-settings-a-webview-of-the-nest.md), 2026-10-12: as Configurações são um webview da janela do Nest.

<a id="context"></a>

## Contexto

A [0021](0021-four-sizes-one-nest.md) levou as Configurações para dentro do Nest, atrás da
engrenagem, para poupar uma webview. Mas as Configurações são a única janela em que o app confia o
que muda o comportamento do Vults e dos agentes: `settings_page` (`app/src/lib.rs`) e
`policies::from_settings` recusam essas chamadas vindas de qualquer outra janela. Essa confiança se
sustenta porque as Configurações não mostram texto de nenhum agente: uma resposta, um passo ou uma
tarefa que um agente escreveu pode trazer palavras feitas para conduzir quem as lê, e o Nest mostra
tudo isso.

<a id="decision"></a>

## Decisão

- As Configurações continuam uma janela própria, criada quando aberta e que some quando fechada. A
  engrenagem do Nest e a da ilha a abrem, numa página quando pedida (`open_settings(section)`).
- Só essa janela pode:
  - escrever a config de um agente ([0005](0005-agent-configs-backup-diff-click.md));
  - salvar ou apagar uma chave;
  - criar ou retirar uma política ([0014](0014-consent-before-autonomy.md));
  - parear, revogar ou alcançar um celular, cujas respostas assinadas valem como um clique
    ([0017](0017-a-signed-phone-decision-is-a-click.md));
  - ligar um pedido feito em nome do usuário, como a web, a memória e o sono
    ([0019](0019-requests-on-the-users-behalf.md)).
- As páginas dela levam os títulos do protótipo Living Flock: General, Agents, Chat & voice,
  Approvals, Projects, Activity, Connectors, Phone, Flock & Zeca, About.

<a id="consequences"></a>

## Consequências

- Um Nest aberto com as Configurações abertas são duas webviews ao lado da ilha, não uma.
- Uma janela que mostra texto de agentes nunca ganha esses poderes por ser juntada às
  Configurações; uma superfície nova que precise de um deles abre as Configurações na página dele.
