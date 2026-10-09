# 0015. Tudo o que está planejado chega até a 0.2.0, em pequenas releases 0.1.x
<!-- source: 68ab77f236da -->

**Status:** Aceita, 2026-10-04. Substitui a [0007](0007-versions-to-0.5.md).

<a id="context"></a>

## Contexto

A [0007](0007-versions-to-0.5.md) deu a cada versão até a 0.5 um tema (Experience, Control,
Platform, Operations). Isso deixava os usuários esperando meses entre releases por trabalho que
fica pronto em dias, e fazia um número de versão prometer um tema inteiro.

<a id="decision"></a>

## Decisão

- A primeira release continua sendo a **0.1.0**.
- Depois dela, as releases são pequenas e frequentes: uma **0.1.x** sai sempre que um ou dois
  passos planejados são integrados e uma verificação rápida (smoke check) passa. Correções urgentes
  também vão na próxima 0.1.x.
- Tudo o que está planejado (superfícies na área de trabalho, controle, voz, histórico, poleiros)
  chega em releases 0.1.x, na ordem das ondas do plano interno (`docs/dev/road-to-0.2.md`).
- A **0.2.0** é o marco Operations: o app completo, políticas e iniciar agentes. Ela só começa
  depois que a [0014](0014-consent-before-autonomy.md) for aceita.
- Linux, KDE Plasma primeiro, até a 0.2.0. Windows e macOS vêm depois.

<a id="consequences"></a>

## Consequências

- Os usuários recebem cada recurso quando ele fica pronto; o changelog é por 0.1.x.
- Aqui, um número de patch não significa mais "só correções": as releases 0.1.x trazem recursos.
  Os números de patch podem passar de 9.
- 1.0 continua significando "pronto para Linux"; a tradução pt-BR ainda espera por ela.
