# 0001. Registrar decisões como ADRs
<!-- source: a343baf8f24b -->

**Status:** Aceita, 2026-10-04. Emendada em 2026-10-12: registros de um mesmo tema podem ser
consolidados.

<a id="context"></a>

## Contexto

As escolhas estavam espalhadas pelo `CLAUDE.md`, pelos planos internos em `docs/dev/` e por threads
de pull request. Os planos são apagados quando terminam, então os motivos iam junto. Depois,
registros que emendavam uns aos outros se acumularam no mesmo tema, e achar a regra em vigor
significava ler quatro deles.

<a id="decision"></a>

## Decisão

Toda escolha que molda o projeto ganha um registro curto em `docs/adr/`, numerado, com status,
contexto, decisão e consequências. Os planos em `docs/dev/` apontam para os registros; os registros
duram mais que os planos.

Registros de um mesmo tema podem ser consolidados num **registro vivo**: a regra atual, inteira,
com um *Histórico* curto dos registros incorporados a ele. Uma consolidação não decide nada novo e
só descarta o que um registro posterior já tinha substituído. Cada registro incorporado continua
como um esboço que diz onde foi incorporado, para que os links continuem funcionando; o histórico
do git guarda o texto antigo.

Daí em diante um tema cresce no lugar: uma nova decisão sobre um tema que já tem registro vivo é uma
edição desse registro, feita num PR próprio com uma linha datada no seu *Histórico*. Um número novo
é só para um tema novo.

<a id="consequences"></a>

## Consequências

- Quem contribui pode entender por que o app funciona assim sem conhecer o histórico.
- Uma mudança numa regra começa no registro dela (o registro vivo do tema, ou um novo registro para
  um tema novo), não com uma edição só no `CLAUDE.md`.
- Um tema, um lugar para ler a regra em vigor.
