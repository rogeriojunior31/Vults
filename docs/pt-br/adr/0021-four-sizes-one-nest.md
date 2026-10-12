# 0021. Quatro tamanhos de atenção, um Nest com cinco abas
<!-- source: 34afb111d5cb -->

**Status:** Aceita, 2026-10-11. Emenda a [0020](0020-0.2.0-at-wave-4-then-0.2.x.md): a 0.2.0 sai
com este layout.

<a id="context"></a>

## Contexto

A onda 4 levou o Nest a nove abas (Chat, Tasks, Memory, Roosts, History, Usage, Traces, Audit,
Agents). As Configurações são uma janela própria, o resumo da semana é uma das páginas delas e o
card do GitHub abre a partir de uma aba da ilha. Nove abas não cabem numa janela estreita, e os mesmos registros
ficam em três lugares. O Nest só abre com o Zeca ligado, então os registros do bando e as tarefas
somem junto com ele.

O protótipo Living Flock do dono (`design/explorations/08-living-flock.html`) redesenha o app como
uma identidade em quatro distâncias, com o bando num céu compartilhado em todos os tamanhos.

<a id="decision"></a>

## Decisão

- **Quatro tamanhos**:
  - **Mini** é a pílula, 340 x 38, com o bando e nenhum texto. Qualquer novidade chega a ela só como corpo, cor ou movimento.
  - **Quick** é a ilha, com 440 px de largura, dentro da superfície de camada fixa de hoje.
  - **Panel** é a janela do Nest, redimensionável de 720 a 1280 px.
  - **Full** é o Nest maximizado ou em tela cheia; Esc volta ao tamanho anterior.
  - A sobreposição de layer-shell nunca passa do seu tamanho fixo por causa do Panel ou do Full.
- **O Nest tem cinco abas**, Flock, Tasks, Chat, Activity e Connections, e as Configurações atrás de uma engrenagem:
  - Memory vira um painel lateral do Chat.
  - Activity reúne Week (com *By project*, que substitui Roosts), History, Cost (que substitui Usage), Traces e Audit.
  - A tabela de capacidades dos agentes vai para Configurações › Agents.
  - As Configurações deixam de ser uma janela própria.
- **Trocar de tamanho mantém a seleção.** A sessão selecionada é o `focus` do core, como hoje. A aba, os filtros e a conexão escolhida são aparência: o app os guarda e entrega ao Nest quando ele abre.
- **O Nest abre com o Zeca desligado.** Sem ele não há aba Chat nem *Ask Zeca*. O bando, as tarefas, os registros, as conexões e as configurações continuam.
- **Os cards ficam onde estão.** A ilha continua sendo o único lugar do desktop onde um card aparece ([0008](0008-one-core-many-surfaces.md)). Panel e Full levam até ela e nunca mostram Allow ou Deny. A paleta de comandos também nunca responde a um card.

<a id="consequences"></a>

## Consequências

- O Nest ganha um segundo motivo para existir além do Zeca, então a [0010](0010-zeca-is-optional.md) vale nos dois sentidos: as superfícies do próprio bando nunca dependem dele.
- Em repouso continua havendo uma única webview viva, a da ilha (mais o widget de canto, enquanto ele estiver escolhido). As Configurações deixam de abrir uma webview própria, então abrir o Nest acrescenta uma webview, não duas.
- Quase todas as capturas da ilha mudam, então as referências visuais são renovadas passo a passo (`docs/dev/plan-living-flock.md`, U0 a U13).
- O portão da 0.2.0 é o teste R3 neste layout: uma vez depois do U1, e de novo depois do U13. A tag vem depois do segundo teste.
