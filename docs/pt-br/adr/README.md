# Decisões
<!-- source: d75d99b7aebd -->

Seis registros, um por regra em que o projeto inteiro se apoia. Os registros 0001 a 0004 guardam os
motivos das quatro regras do `CLAUDE.md`, com os mesmos números; 0005 e 0006 guardam os dois limites
que todo recurso atravessa. O código e os guias dizem *o que* o app faz; estes dizem *por quê*.
Planos ficam no [roadmap](../../dev/ROADMAP.md), o que já saiu no [changelog](../../../CHANGELOG.md).

| # | Registro | Regra no `CLAUDE.md` | Antes de 2026-11-15 |
|---|---|---|---|
| [0001](0001-hook-never-blocks.md) | O hook nunca bloqueia um agente | 1 | 0003 |
| [0002](0002-permissions.md) | Quem responde a uma permissão, inclusive por voz | 2 | 0004, com as regras de voz da 0028 e da 0032 |
| [0003](0003-agent-configs.md) | As configurações dos agentes, e os agentes que suportamos | 3 | 0005 |
| [0004](0004-network-and-secrets.md) | Rede, segredos, telemetria, voz e áudio | 4 | 0019, com as regras de privacidade da 0028 e da 0032 |
| [0005](0005-zeca.md) | Zeca: opcional, separado do bando, regras antes de modelos | | 0018 |
| [0006](0006-surfaces.md) | Superfícies: um núcleo, quatro tamanhos, um card respondido onde você está; só ações que funcionam | | 0021, com a 0011 |

<a id="old-numbers"></a>

## Números antigos

Os registros foram renumerados em 2026-11-15. Um commit, PR ou entrada do changelog de antes dessa
data usa os números antigos: leia-os pela última coluna acima. Dentro dos registros, *antiga NNNN*
sempre quer dizer um número antigo. Aposentados sem sucessor, com o texto no histórico do git: 0001
(como escrever um registro, abaixo), 0002 (Rust, sem sidecars, sem GPL: agora *Stack* no
`CLAUDE.md`), 0020 (releases: [Releasing](../../dev/releasing.md)), 0027 (uma resposta aberta para
qualquer ferramenta: G1/G2 do roadmap) e 0031 (remoção de Tasks: o changelog). Números mais antigos,
incorporados antes (0006 a 0030), estão no *Histórico* de cada registro.

<a id="writing-one"></a>

## Como escrever um

Um registro só existe para uma regra em que o projeto inteiro se apoia: uma regra do `CLAUDE.md`, ou
um limite que todo recurso precisa respeitar. Uma escolha que pertence a um recurso não é registro;
vai no guia ou na referência desse recurso. Um plano, uma fase, uma meta de release ou uma remoção
também não é decisão: isso vai no roadmap ou no changelog.

- **Uma decisão nova é uma edição do registro da sua regra**, num PR próprio, com uma linha datada
  no seu *Histórico*. Um registro novo precisa antes de uma regra nova no `CLAUDE.md`; ele ganha o
  próximo número e uma linha aqui.
- **O histórico registra decisões, não entregas.** Uma linha datada diz qual regra mudou e por quê.
  Qual PR ou etapa a implementou vai no changelog.
- **Uma regra do `CLAUDE.md` muda primeiro no seu registro**, nunca só no `CLAUDE.md`.
