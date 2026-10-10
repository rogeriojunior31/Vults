# 0006. Segredos só no chaveiro do sistema; sem telemetria
<!-- source: 8bb564797626 -->

**Status:** Aceita, 2026-10-04 (a regra existe desde o início do projeto). Emendada pela
[0016](0016-updates-asked-for-and-signed.md) e pela [0019](0019-requests-on-the-users-behalf.md)
(requisições em nome do usuário: cada uma ligada, com orçamento e registrada).

<a id="context"></a>

## Contexto

O app vê comandos, nomes de arquivos, diffs e chaves de API. Os usuários o rodam ao lado de
trabalho privado.

<a id="decision"></a>

## Decisão

Um segredo é escrito apenas no chaveiro do sistema, nunca em um arquivo ou log. O app não envia
nada sobre seu uso para lugar nenhum. Tudo o que sai da máquina (um chat pela API, uma transcrição
na nuvem) é escolha explícita do usuário e avisa isso onde é ativado. A verificação de
atualizações é o único pedido que o app faz por conta própria, e só quando o usuário a ativou
([0016](0016-updates-asked-for-and-signed.md)).

<a id="consequences"></a>

## Consequências

- Nenhum número de uso para guiar o roadmap: aprendemos pelas issues e usando o app nós mesmos.
- Qualquer coisa que aprenderia com os dados dos usuários (um classificador treinado, um
  ranqueamento) tem que aprender na máquina do usuário ou não aprender.
