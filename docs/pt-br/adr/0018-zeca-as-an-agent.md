# 0018. Zeca: opcional, separado do bando, regras antes de modelos
<!-- source: ecda4c21e16d -->

**Status:** Aceita, 2026-10-10. Consolidada em 2026-10-12 a partir da 0010 e da 0012.
Alterada em 2026-10-16 pela [0031](0031-tasks-leave-vults.md): Tasks sai do Vults.

<a id="context"></a>

## Contexto

O app faz dois trabalhos: mostra o que os agentes do usuário estão fazendo (o bando) e oferece
um companheiro que conversa, escuta, lê o bando e lembra (o Zeca). Alguns usuários querem só o
primeiro, e o poder do segundo não pode vazar para ele. Também foi proposto um pequeno modelo
local de decisão para as escolhas do próprio app, mas os eventos que ele recebe já vêm tipados,
e modelos assim sem fine-tuning acertam menos que chutar a classe majoritária, com uma confiança
que não conseguimos calibrar sem coletar dados ([0019](0019-requests-on-the-users-behalf.md)).

<a id="decision"></a>

## Decisão

- **Opcional.** Tudo sobre o bando funciona com o Zeca desligado: sessões, cards, notificações,
  conectores, o Nest com seus registros e custos ([0021](0021-four-sizes-one-nest.md)). Com ele
  desligado: nada de chat, de microfone, de atalho de fala, de *Chat…* na bandeja, de aba Chat ou de
  *Ask Zeca*; a ilha tem a sua própria aparência ociosa, e o `zeca` não inicia processo, não escreve
  arquivo e não faz requisição.
- **Onde:** o cérebro dele é um crate `zeca` na camada Connect, ao lado do `chat`; o corpo dele
  (sprites, a ilha, o Nest) fica em Experience. Nenhum crate abaixo do `zeca` sabe que ele existe;
  o `app` o conecta.
- **O que ele lê:** o bando, por um `FlockReader` construído a partir da view, sem caminho até um
  `Intent`. O conteúdo do bando é entrada não confiável: pode trazer uma injeção.
- **O que ele faz:** ações tipadas próprias; o `zeca` não depende do `vults-core`, então o `app`
  mapeia cada uma para as classes de política do `core` (leitura, leitura externa, escrita,
  destrutiva, gasto, externa) e para o ledger, como as de qualquer agente. Ele nunca constrói
  `Decide` nem `DecideAlways`, por tipo, e nunca responde a uma permissão, sua ou de outro agente
  ([0004](0004-a-human-answers-permissions.md)).
- **O que ele lembra:** um repositório git próprio (via `gix`): toda escrita é um commit, desfeita
  por um revert.
- **Código é trabalho dos agentes:** no harness (uma chave de API ou um modelo local) ele nunca roda
  código por conta própria. Remover Tasks não cria um caminho substituto de execução. O Chat via
  CLI e `/continue` mantêm seus drivers e verificações de permissão; os comandos e edições dele
  no chat perguntam ao usuário como em qualquer chat.
- **O trabalho dele nunca é uma sessão:** os processos de CLI dele levam `VULTS_ZECA=1`; o hook
  marca os eventos deles e o `core` os manda para o estado do chat (taint e atividade), nunca para o
  bando. Qualquer processo poderia definir essa variável, então um evento marcado nunca esconde nem
  descarta um pedido de permissão: ele continua sendo perguntado ao usuário, pelo approver do chat.
- **Regras antes de modelos** nas decisões do próprio app:
  - O que pode ser decidido a partir de estado tipado é decidido por regras simples no `core`, com
    testes: notificações, atenção, a aparência do Zeca, qual agente está instalado e tem cota.
  - Texto livre do usuário (o que ele pede ao Zeca) vai para o modelo do chat.
  - Um classificador treinado pode vir depois só atrás da mesma interface, só para texto livre, só
    local, e só se um conjunto de avaliação neste repositório mostrar que as regras não bastam.
  - Nenhum modelo responde a uma permissão.

<a id="consequences"></a>

## Consequências

- O crescimento do companheiro nunca muda quem responde a uma permissão; as decisões continuam
  exatas, testáveis e explicáveis.
- O `scripts/check-layers.sh` põe o `zeca` em Connect; um teste de falha de compilação garante que
  o `FlockReader` não alcança um `Intent`.
- O protocolo ganhou um campo para o evento marcado (versão 6).

<a id="history"></a>

## Histórico

- 0010, 2026-10-04: o Zeca é opcional e fica separado do bando; o
  handle dele não consegue construir uma decisão.
- 0012, 2026-10-04: regras antes de modelos nas decisões do próprio
  app.
- 2026-10-16, [0031](0031-tasks-leave-vults.md): remover Tasks e a escolha de agentes; manter
  `/continue` e custos, e manter o bando independente do Zeca (D7).

- 2026-10-21: UX8 começa com um catálogo inerte de perfis integrados na camada Connect. Os perfis descrevem papéis, instruções e aparência, sem definir acesso. A leitura independe do Zeca e de sua memória; esta etapa não oferece execução nem acesso a contexto privado.
- 2026-10-21: A UX8 adiciona cópias dos perfis integrados pertencentes ao usuário, salvas como revisões imutáveis no banco local com rejeição de edições desatualizadas. Editar nome/instruções não concede execução, acesso à memória nem autoridade sobre permissões; origem e aparência do perfil integrado ficam fixadas.
- 2026-10-21: As cópias de perfis podem revisar a espécie e um acessório fixo usando o catálogo existente de personagens. Cada revisão mantém a própria aparência; revisões anteriores e perfis integrados imutáveis não mudam. Aparência sazonal, habilidades e execução ficam fora desta etapa.
- 2026-10-21: O armazenamento de conversas especializadas agora fixa a origem/revisão explícita do perfil e suas instruções/aparência completas na criação. Esses registros não podem carregar o contexto congelado do Zeca nem ser abertos/listados pela rota atual do Chat dele. A execução especializada e sua disponibilidade com o Zeca desligado continuam desabilitadas até a implementação do próprio contrato de isolamento.
- 2026-10-21: Adicionar um construtor de runtime especializado para Claude Code que mantém somente o briefing fixado, configurações isoladas e o fluxo existente de aprovação humana. Ele remove MCP do companheiro, diretórios extras e histórico de API, rejeita contexto/ferramentas do companheiro e bloqueia provedores não suportados antes de iniciar um turno. Essa base não habilita Chat de perfil, recuperação de memória, escrita no diário nem disponibilidade especializada com o Zeca desligado; o roteamento nativo continua fechado.
- 2026-10-21: O turno nativo agora fixa e verifica os metadados do destinatário antes da consulta à memória. Runtimes vinculados a perfis mantêm seu snapshot, origem e executor exatos, pulam os callbacks de consulta/contexto/MCP/diário do companheiro e não podem alimentar a extração de memória dele. Reiniciar/abrir/parar/excluir limpa ou substitui os metadados do destinatário junto com o runtime. O roteamento especializado para o usuário continua fechado até a interface de destinatário e as transições de comandos/voz.
