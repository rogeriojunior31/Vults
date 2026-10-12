# 0019. Requisições em nome do usuário: cada uma ligada, com orçamento e registrada
<!-- source: 05830d2a4800 -->

**Status:** Aceita, 2026-10-10. Emenda a [0006](0006-keyring-no-telemetry.md).

<a id="context"></a>

## Contexto

A [0006](0006-keyring-no-telemetry.md) e a [0016](0016-updates-asked-for-and-signed.md) permitem
uma requisição que o app faz por conta própria: a verificação de atualização. O plano do Zeca e do
celular (`docs/dev/plan-zeca.md`) traz outras que acontecem sem o usuário pedir naquele momento:
busca e leitura na web durante um turno do chat, o sono do Zeca (consolidação da memória por um
modelo), rotinas (um resumo da manhã, o CI da noite), o relay e o push do celular. Cada uma manda
para fora da máquina algo que importa ao usuário, custa dinheiro ou cota e poderia virar telemetria
por acidente.

<a id="decision"></a>

## Decisão

- Toda requisição que sai da máquina sem um clique naquele momento é um **recurso com sua própria
  chave** no Settings, desligado por padrão. Ligá-lo diz o que ele envia, para onde e quanto custa.
- Cada uma tem um **orçamento** (requisições, tokens ou dinheiro por dia) e para quando ele acaba.
- Cada requisição grava um **registro local** que o usuário pode ler (o quê, para onde, quando,
  custo). Nada sobre ela é enviado a nenhum outro lugar.
- Todas passam por **um único portão de rede, do `app`**, que confere a chave e o orçamento do
  recurso. Nenhum crate abre uma conexão para um desses recursos por fora dele.
- **Nunca telemetria:** nenhuma requisição leva uso, um id ou qualquer coisa sobre o usuário além do
  que o recurso precisa para fazer seu trabalho.
- Uma chamada de ferramenta que o modelo faz por conta própria durante um turno (uma busca ou
  leitura na web) é uma requisição em nome do usuário, mesmo que o usuário tenha iniciado o turno.
- Os conectores (o GitHub pelo `gh`, hoje) são recursos assim: cada um fica desligado até o usuário
  conectá-lo, e a consulta periódica dele passa pelo portão, com orçamento e registro.
- Uma requisição que o usuário inicia com um clique ou uma mensagem (enviar um turno do chat, um
  chat por API, uma transcrição em nuvem que ele escolheu) continua como diz a
  [0006](0006-keyring-no-telemetry.md): escolha do usuário, dita onde ela é ligada.

<a id="consequences"></a>

## Consequências

- A regra 4 do `CLAUDE.md` cita este registro ao lado da verificação de atualização.
- O portão é o `app/src/gate.rs`; um teste roda um dia com todos os recursos desligados e conta
  zero requisições.
- As ferramentas web (plan-zeca Z7), o sono (M8), o relay (L2) e a consulta dos conectores (S10) têm
  cada um sua chave, orçamento e registro; as rotinas (K4) acrescentam os seus no passo que as
  constrói.
