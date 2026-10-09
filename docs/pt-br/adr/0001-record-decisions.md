# 0001. Registrar decisões como ADRs
<!-- source: 98b9d543f107 -->

**Status:** Aceita, 2026-10-04.

<a id="context"></a>

## Contexto

As escolhas estavam espalhadas pelo `CLAUDE.md`, pelos planos internos em `docs/dev/` e pelas
discussões de pull requests. Os planos são apagados quando concluídos, e os motivos iam junto.

<a id="decision"></a>

## Decisão

Toda escolha que molda o projeto ganha um registro curto em `docs/adr/`, numerado, com seu status,
contexto, decisão e consequências. Os planos em `docs/dev/` apontam para os registros; os registros
duram mais que os planos.

<a id="consequences"></a>

## Consequências

- Quem contribui pode entender por que o app funciona assim sem precisar do histórico.
- Mudar uma regra começa com um novo registro que substitui o antigo, e não só com uma edição no
  `CLAUDE.md`.
