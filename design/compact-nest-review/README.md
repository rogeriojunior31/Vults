# Validação de design — Compact, Mini e Nest

Esta pasta contém a proposta interativa e a análise do design do Vults. As mudanças estão no protótipo; o produto continua com o comportamento original. Nenhum ajuste foi considerado aprovado pelo usuário ainda.

## Abrir e comparar

Abra [index.html](index.html) no navegador. O arquivo contém a proposta, os sprites e as 88 capturas, funciona offline e não depende de servidor.

1. Selecione **Mini**, **Compact / Quick** ou **Nest**.
2. Use **Janela** para comparar tamanhos. Mini: 560 × 56; Compact: 560 × 600 são os padrões atuais. Nas extensões de 560, 680 ou 760 px, a largura acompanha a troca de Mini para Compact.
3. Em Agentes, consulte Profiles, Tasks e Studio. Use busca, detalhes e Chat; os botões de roadmap abaixo da janela abrem 13 demonstrações.
4. Alterne os cenários e a caixa **Animações**. Se o sistema pedir movimento reduzido, essa preferência prevalece.
5. Use **Comparar com o atual**: a captura atual mantém seu tamanho original, sem ser esticada para simular a ampliação.
6. Em **Análise completa**, filtre os achados e abra cada evidência ou destino da proposta.

As regras compartilhadas estão em [DESIGN.md](../../DESIGN.md), com adaptação e histórico em [Design.md](Design.md). As operações de agentes e GitHub são demonstrações locais.

## Revisão 05 — Chat Compact e plano de implementação

O [DESIGN.md da raiz](../../DESIGN.md) passa a definir as regras compartilhadas de UI. Este README mantém o histórico específico do protótipo; [plan-ui.md](../../docs/dev/plan-ui.md) destrincha a implementação dentro do roadmap atual.

| ID | Ajuste no protótipo | Como validar | Decisão |
| --- | --- | --- | --- |
| E01 | Chat Quick com cabeçalho curto: destinatário, executor, Nova conversa e Histórico. Contexto e Memória aparecem uma vez, perto do Zeca e da origem da conversa. | Abra Chat em 560/680/760 × 600; confirme o agrupamento e a preservação do contexto ao mudar para Nest. | Direção solicitada; implementação nativa em UX4/UX8 |
| E02 | Campo com duas linhas confortáveis, mais espaço para histórico e prompts concentrados no estado vazio. Nas três extensões medidas, o histórico ganhou 42 px em relação à revisão 04. Retira a faixa de atalhos repetidos no Compact; anexos, voz, skills, rotina demonstrativa e enviar/parar continuam acessíveis. | Digite duas linhas, anexe, envie e pare a resposta. Campo e ações não se sobrepõem; o Nest mínimo continua utilizável. | Layout em UX2/UX4; rotina com K4 |
| E03 | Histórico do Compact reutiliza as conversas originais, com nome, especialista e origem. Escolher uma conversa conserva seu ID e rascunho; fechar retorna ao Chat. | Abra Histórico com um rascunho, selecione a mesma conversa e retorne. O conteúdo permanece intacto. | UX4; identidade especializada com UX8 |

O plano prioriza **UX0: transformar o lab existente na nova UI**, como base de desenvolvimento para os próximos steps. A versão antiga pode permanecer em uma entrada separada chamada `old`; o lab novo não reúne as duas UIs em uma tela de comparação. UX1–UX5 cobrem ajustes nas funções atuais; UX6–UX7 tratam continuidade e extensão nativa; UX8–UX10 são features futuras; UX11–UX12 acompanham os donos K/M/W/L/G já presentes nos roadmaps. As etapas concluídas continuam concluídas.

A migração do lab é a primeira tarefa planejada, ainda não implementada. O protótipo offline serve como referência de design; os labs atuais continuam disponíveis pelos comandos de reprodução abaixo até a migração. As próximas etapas evoluem o lab novo, sem exigir manutenção paralela do antigo. A referência da revisão 04 está arquivada em [verification-v4.json](evidence/verification-v4.json).

## Revisão 04 — Quick, Agentes, identidade visual e roadmap

Revisão de 13/10/2026. Referências verificadas: [Plan Zeca](../../docs/dev/plan-zeca.md) e [Every agent](../../docs/dev/plan-every-agent.md), reescritos em 12/10 no commit `592f6a6`; [planejamento do Studio](../agent-studio/design.md); componentes de `ui/src/island` e `ui/src/surfaces/nest`; tokens de [themes.css](../../ui/src/themes.css). A direção anterior de uma sidebar alta em “Nesta sessão” foi substituída.

| ID | Ajuste no protótipo | Como validar | Decisão |
| --- | --- | --- | --- |
| D01 | **Quick Flock enxuto:** remove a coluna “Nesta sessão”; pedido em largura completa, contexto em faixa curta e detalhes sob demanda. Remove o título redundante do cenário. | Em 560/680/760 × 600, pedido e respostas ficam visíveis sem rolar. Alterne as aves: etapa, projeto e branch acompanham a seleção. | Pendente |
| D02 | **Tema fiel:** incorpora os tokens reais de themes.css em relatório, Quick, Nest e Studio; fundo preto, painéis #141518, chrome #070809, texto e botões neutros. Retira o sage das superfícies e dos controles principais. | Compare com as capturas atuais; confira Flock, perfis, Skills e Chat. O check compara os valores computados do fundo, painel e ação principal. | Pendente |
| D03 | **Agentes abre nos especialistas:** Profiles / Tasks / Studio em navegação simples com sublinhado; elimina cabeçalhos e cartões de entrada repetidos. | Abra Agentes, navegue nos três destinos e volte. A aba e seus dados continuam preservados ao expandir. | Pendente |
| D04 | **Diretório de perfis:** linhas com nome, função, executor, Copy/Edit e Chat; busca por nome/função e filtro de propriedade. Nest amplo tem detalhe lateral; Quick abre o perfil em diálogo. | Os quatro presets cabem sem rolagem no Quick inicial. Buscar Reviewer e filtrar presets encontra Lia. Presets continuam imutáveis; copiar cria perfil próprio. | Pendente |
| D05 | **Tarefas mais claras:** objetivo antes de responsável, etapa e status; detalhes de execução agrupados. Nova tarefa guarda projeto, especialista e critérios em rascunho, antes de iniciar a demonstração. | Crie um objetivo com dois critérios; marque um, inicie a execução ilustrativa e abra Chat. O ID e os critérios permanecem os mesmos. O editor de perfil mantém Salvar/Descartar visíveis, inclusive em 480 × 360. | Pendente |
| D06 | **Debrief K1/K2 e risco K3/W2:** resumo ao retornar ocupa uma faixa; pedido mantém precedência. Explicação do alcance e pasta externa ficam no cartão. | Abra Ao voltar, depois o resumo e o Chat vinculado. A permissão permanece pendente. Em Risco, simule ../shared e confira comando e aviso. | Pendente |
| D07 | **K3 e M7:** leituras selecionáveis em lote, com clique humano; pergunta textual com resposta explícita. | Desmarque uma leitura e permita a outra: o pedido restante continua pendente. Negue o restante. Uma resposta vazia não avança a pergunta. | Pendente |
| D08 | **K4 Rotinas:** modelos de brief, CI, dependências e página; editor de alvo, gatilho, escopo e orçamento. Criar pelo Chat ou Tasks; salvar começa desligado e ativar exige revisão. | Crie, ative, edite e desligue. Editar volta a rascunho desligado. Não existe agendador ou requisição externa nesta demonstração. | Pendente |
| D09 | **K5/K6:** propostas de AGENTS.md e SKILL.md no Chat com diff, backup e aceite; dispensar não escreve nada. | Abra as duas propostas; revise o diff e aceite ou dispense. Nenhum arquivo de instruções ou skill é instalado. | Pendente |
| D10 | **K7/K8:** quatro humores com clips atuais; voz com contexto, ativação explícita e orçamento ilustrativo. | Alterne calm/alert/worried/celebrating e observe as poses. Voz só entra em listen após a revisão. Nunca responde a uma permissão. | Pendente |
| D11 | **L3–L5 Android:** três vistas no telefone: pedido, tarefa e GitHub. Aprovação, aceite e transferência passam pela etapa biométrica; o aceite cria um rascunho vinculado no desktop; CI tem proposta com prazo de cinco minutos. | Simule expiração: respostas ficam indisponíveis. Aceite a tarefa e confira o mesmo registro no desktop; autorize CI e confira a indisponibilidade de repetir a decisão. A validação real da assinatura permanece fora do HTML. | Pendente |
| D12 | **W9b e cobertura de agentes:** custos observados só aparecem após revisar ativação; distingue Watch, Answer e Drive, integrações disponíveis, pendentes e terminais sem resposta. | Revise o diff de OTLP local e ative no exemplo. Cursor permanece Terminal; Goose fica Not verified; G1/G2 não pode ser habilitado. | Pendente |
| D13 | **Animações do Flock:** storyboard dos 12 itens ainda abertos, agrupados por comportamento e identificados pelos IDs do board. | Abra Animações planejadas. Os clips são os atuais; as poses futuras não são apresentadas como arte pronta ou aprovada. | Pendente |

### O que o roadmap representa

| Origem / status em 12/10 | Representação proposta |
| --- | --- |
| K1, K2 abertos | Debrief em Quick e Nest; orçamento de até 4 falas espontâneas por hora e silêncio em foco explicitados no detalhe. Cartão nunca é adiado. |
| K3 e W2 abertos | Risco, pasta de execução/busca e leitura em lote; decisão individual preservada. |
| M7 follow-up | Pergunta com texto; instalação do MCP para Codex permanece trabalho de configuração, não uma conexão falsa. |
| K4 aberto | Rotinas no Chat e em Tasks, com interruptor, orçamento, escopo e revisão. |
| K5, K6 abertos | Propostas revisáveis para instruções e skill; o protótipo não aprende nem grava arquivos. |
| K7, K8 abertos | Humores e voz opt-in no Chat; clips atuais e transcrição de demonstração. Direção do olhar e realtime real pendentes. |
| K10 aberto | Linha de Zeca por ACP para editores na matriz de suporte, marcada Planned. |
| L3, L4, L5 abertos | Telas Android e sequência de decisão; resumo de tarefas/handoff e serviço GitHub. Criptografia, relay, attestation e Live Updates não executados. |
| W9b aberto | Proveniência OTLP das sessões observadas e revisão de configuração local. Valores ilustrativos, não cobrança real do plano. |
| B4, C1, C5, C6, F3, R2 abertos em Every agent | Matriz separa suporte atual e futuro; não anuncia agentes planejados como instalados. |
| G1/G2 propostos, ADR 0027 pendente | Ferramentas nomeadas e VERSION 9 em estado indisponível. |
| Board #03/#09/#11/#44/#34/#35/#36/#29/#30/#40/#47/#42 abertos | Seis estudos de comportamento cobrem os 12 IDs. Nenhum frame novo substitui o sprite aprovado. |
| W1–W11 já concluídos | Base de Tasks, revisão, handoff, continuidade, custos e ACP; não tratados como novos lançamentos. A UI ainda precisa integrar seus donos nativos. |
| R3/tag, F4 e follow-ups internos | Smoke/release, issues, testes reais, formatos, ACK atrasado e processo WebKit são validações/backend; não geram uma aba nova. Z6/Z8 dependem de validação de provider e conteúdo externo. |
| X6/X7/X9 e distribuição, Later | Navegador headless, outros sistemas/desktops e empacotamento ficam como contexto futuro, sem prometer telas prontas. |
| X1/X2/X3/X4/X5/X8 e Gemini, Dropped | Não foram reintroduzidos no protótipo. Voz K8 não ressuscita o dicionário/cloud transcription X5. |

A proposta não altera cronograma nem aprova ADRs. Funcionalidades existentes e conceitos do Studio continuam separados de integrações planejadas. O texto de status e a fonte de cada demonstração permitem validar um ajuste de cada vez.

## Revisão 03 — extensão, menos rolagem e Agent Studio

A direção de largura compartilhada vem da correção do usuário: ampliar horizontalmente o Mini e aproveitar a extensão no Compact. Os tamanhos da revisão 02 ficam como comparação, não como padrão atual. O Studio original foi lido junto de sua especificação, instruções e componentes nativos de Tasks/Chat; as fontes em design/agent-studio permanecem preservadas.

| ID | Ajuste implementado no protótipo | Como validar | Decisão |
| --- | --- | --- | --- |
| C01 | Mini 680 × 56 e Compact 680 × 600 compartilham largura; alternativas 560 e 760. | Escolha a extensão 760, recolha em Mini e abra pelo teclado. Ambos devem manter 760 px; Nest tem tamanho independente. | Pendente |
| C02 | Flock Compact distribuiu decisão e contexto em colunas na revisão 03; substituído por D01. Histórico em diálogo. | Pedido e duas respostas cabem sem rolar nos três tamanhos novos. Abra Etapas e retorne. Expandir uma regra longa pode exigir rolagem da própria região. | Pendente |
| C03 | Semana e Ajustes rápidos reorganizados para caber na janela. | Veja período, quatro métricas, gráfico e exportação na Semana; em Geral, alcance todos os controles. | Pendente |
| C04 | Tarefas vira Agentes, com Studio, Tasks e Agents reutilizados da fonte original. | Abra os três destinos. Overview inicial cabe sem rolar; listas e formulários rolam apenas quando exigirem. | Pendente |
| C05 | Studio mantém Skills, Integrations, Validation e os fluxos de importação/cópia. | Importe um ZIP seletivamente; reimporte uma revisão; copie Atlas; crie um rascunho MCP. Nenhuma dessas ações instala ou executa ferramentas. | Pendente |
| C06 | Chat usa os modelos originais de conversa e execução, com estado preservado ao mudar modo/aba. | Digite um rascunho no Compact, abra Nest e volte. Abra a mesma tarefa no Chat e confira seu ID e contexto. | Pendente |
| C07 | Retrato interativo do Zeca: toque/Enter, ponteiro, pensar, receber anexo e voz de demonstração. | Observe startle, preen, think, swallow e listen. Com movimento reduzido, o texto mantém o mesmo feedback. | Pendente |
| C08 | Atalhos para o bando/próximo passo e acesso explícito a Contexto/Memória. | Clique no atalho, confira o rascunho, abra e feche Memória. A ação nunca envia o rascunho sozinha. | Pendente |
| C09 | Compositor fixo e rolagem só no histórico; Chat vazio cabe no Compact. | Faça várias mensagens e confirme que Enviar permanece disponível. No Nest mínimo, histórico mantém pelo menos 40 px. | Pendente |
| C10 | Regras de integração documentadas: identidade, fonte, revisão, presets, rascunhos e limites nativos. | Confira Design.md, seção 12. Perfil não é executor, catálogo não é ferramenta instalada e memória de Zeca não vira memória do outro agente. | Pendente |

O estado inicial do Overview foi verificado nas três extensões; execuções adicionais podem exigir rolagem da região. As listas de agentes e skills mantêm seu conteúdo completo.

A UI do Studio conserva o inglês de seu protótipo original. A adaptação visual não é uma tradução completa.

## Revisão 02 — histórico de espaço e animações

Cada linha pode ser aprovada, ajustada ou descartada separadamente. Marque o resultado somente depois de experimentar o HTML.

| ID | Ajuste implementado no protótipo | Como validar | Decisão |
| --- | --- | --- | --- |
| A01 | Mini ampliado de 340 × 38 para **460 × 64**, com alternativas 400 × 56 e 520 × 72. | Compare os três novos tamanhos e o anterior; confira se quatro pássaros, resumo e acesso cabem sem cortes. | Dimensões substituídas por C01 |
| A02 | Mini mostra quantidade de sessões e estado por texto, com indicador de pendência. Clique ou Enter abre Compact. | Alterne Pedido pendente, Em andamento e Sem dados; confira textos e teste Enter com foco no Mini. | Pendente |
| A03 | Compact ampliado de 502 × 640 da revisão 01 para **600 × 760**, com alternativas 560 × 680 e 640 × 820. | Percorra todas as abas nos três novos tamanhos; volte ao tamanho anterior para comparar. | Dimensões substituídas por C01 |
| A04 | Compact usa o espaço extra: pássaros maiores com estado escrito, navegação maior, cartões com respiro, quatro métricas lado a lado e campo de mensagem mais alto. | Flock: leia os quatro estados e o comando. Atividade: confira métricas. Chat: digite duas linhas e confirme que Enviar fica inteiro. | Pendente |
| A05 | Sprites originais agora animados por estado; fase diferente por sessão. Espécies atratus e aura, 11 clips exportados por espécie. | No Flock veja aprovação, edição e repouso; mude para Em andamento e observe a sessão principal. Concluído termina e mantém a pose final. | Pendente |
| A06 | Entrada de conteúdo 160 ms, Memória 200 ms, diálogo 160 ms e troca de tamanho 280 ms. Indicador do Mini tem brilho por três ciclos. | Troque abas e modos; abra Memória e Nova tarefa. O foco deve permanecer utilizável, sem atraso para clicar. | Pendente |
| A07 | Controle de animações, preferência do sistema, reprodução limitada a 30 fps e pausa fora de tela, em aba oculta, na análise e na comparação atual. | Pause Animações; emule movimento reduzido no navegador. Saia da proposta e confira que os checks de pausa passam. | Pendente |

## Revisão 01 — cada bug e refinamento

Os 29 achados abaixo pertencem à análise inicial. A coluna de direção descreve a solução proposta; não significa que a correção tenha sido aplicada ao produto. A evidência completa e a implementação demonstrada estão no HTML. As operações reais continuam simuladas.

| ID | Tipo / prioridade | Problema e direção proposta | Validação esperada | Decisão |
| --- | --- | --- | --- | --- |
| B01 | Bug / P1 | **Chat: O compositor se sobrepõe ao rodapé no Nest mínimo.** Reduzir o cabeçalho em janelas baixas; permitir que a conversa encolha com min-height: 0; reservar espaço independente para compositor e rodapé. Simplificar os seletores em uma única linha. | Em 480 × 360, compositor e rodapé não se intersectam, o botão Enviar fica inteiro e a conversa mantém altura útil. | Pendente |
| B02 | Bug / P1 | **Ajustes: Controles de Configurações ultrapassam a borda direita.** Substituir a sidebar por um seletor de página abaixo de 650 px. Empilhar descrição e controle largo; usar largura máxima de 100%. | As dez páginas cabem horizontalmente em 480 × 360, inclusive controles abaixo da primeira dobra. | Pendente |
| B03 | Bug / P1 | **Ajustes: Interruptores não têm o nome da opção na árvore acessível.** Associar cada controle ao título no helper compartilhado; usar label para checkbox e for/id ou aria-labelledby nos demais. | Todo controle tem nome acessível específico, preservado após redesenho e mudança de idioma. | Pendente |
| B04 | Bug / P1 | **Tarefas: Status e responsável têm seletores sem nome acessível.** Identificar a função e a tarefa no nome acessível; mostrar rótulos curtos junto aos seletores. | Status de [tarefa] e Responsável por [tarefa] são anunciados separadamente. | Pendente |
| B05 | Bug / P1 | **Superfícies: A troca Nest → Quick perde a área fora do Chat.** Transportar a seleção e subárea para a ilha; manter um resumo de Tarefas no compacto ou explicitar o destino ao trocar. Guardar rascunho, filtros e sessão em foco. | Activity e Connections mantêm a área nas duas direções; Chat mantém rascunho. Confirmar a integração no app Tauri. | Pendente |
| B06 | Bug / P2 | **Custos: O uso de plano em 91% aparece com barra verde.** Aplicar os mesmos limiares e acrescentar um texto (“quase no limite”) para não depender só da cor. | 69% neutro; 70–89% âmbar; 90% ou mais vermelho com texto. | Pendente |
| B07 | Bug / P2 | **Navegação: A tablist principal não responde às setas.** Aplicar roving tabindex, setas, Home/End e aria-controls/aria-labelledby na navegação principal; manter Alt+1–5. | Há um único ponto de Tab na tablist; setas mudam foco e área, ignorando o Chat quando desligado. | Pendente |
| B08 | Bug / P2 | **Navegação: Abas da ilha não anunciam qual está ativa.** Usar aria-pressed nos botões da ilha ou o mesmo padrão de tablist do Nest, com rótulos visíveis curtos. | A área ativa é anunciada e se mantém ao trocar o tamanho. | Pendente |
| B09 | Bug / P2 | **Conexões: Um botão da ilha de Conexões não tem nome acessível.** Nomear a ação pelo objeto, por exemplo “Dispensar aviso de checks de me/vults #56”. | Nenhum botão visível e interativo é exposto com nome vazio. | Pendente |
| R01 | Refino / P1 | **Flock: O pedido de decisão deve vir antes do cenário em janelas baixas.** Priorizar o cartão pendente. Reduzir ou recolher o cenário abaixo de 470 px de altura; preservar os pássaros em janelas maiores. | Ao abrir a sessão pendente em 480 × 360, o pedido e pelo menos uma ação estão visíveis sem rolar. | Pendente |
| R02 | Refino / P2 | **Flock: O Flock usa muito espaço para representar a mesma sessão.** Manter céu e identidade, mas reduzir a altura e usar rótulos de 11–12 px; separar “quem está no bando” de “o que exige ação”. | Nomes continuam legíveis com quatro sessões e com nomes longos; detalhes usam a área liberada. | Pendente |
| R03 | Refino / P2 | **Tarefas: Cada tarefa oferece ações demais no mesmo nível.** Exibir objetivo, progresso e próximo passo; agrupar ações secundárias em uma área expansível. Manter exclusões com confirmação e alcance explícito. | A execução/revisão tem uma ação principal clara; apagar tarefa e remover pasta não parecem equivalentes. | Pendente |
| R04 | Refino / P2 | **Tarefas: Uma visão de Tarefas no compacto ajudaria a continuidade.** Proposta de evolução: incluir um resumo de tarefas no compacto, com objetivo, status e critérios, levando ao Nest para os detalhes. | A inclusão preserva leitura e alvos de clique; requer integração futura, pois essa aba ainda não existe na ilha. | Pendente |
| R05 | Refino / P2 | **Atividade: Semana põe explicações antes dos números.** Mostrar período e indicadores primeiro. Mover a explicação sobre registro e privacidade para uma área expansível; manter o contexto dos números. | A primeira dobra mostra ao menos dois indicadores úteis no tamanho mínimo. | Pendente |
| R06 | Refino / P3 | **Atividade: Por projeto pode mostrar resumo antes do conteúdo completo.** Usar resumo com estado, tempo e pendências; abrir memória, custos e worktrees quando solicitado. Preservar o diff antes de compartilhar configuração. | Dois projetos são comparáveis sem ler todo o conteúdo de memória. | Pendente |
| R07 | Refino / P3 | **Atividade: Histórico precisa de busca e leitura por registro.** Adicionar busca por agente/projeto/resultado e agrupamento por dia; manter o conteúdo como contagens. | Buscar “Codex” ou “Falhou” retorna o turno correspondente; limpar a busca restaura a lista. | Pendente |
| R08 | Refino / P3 | **Atividade: Traces pode explicitar a relação turno → ferramenta.** Agrupar ferramentas dentro do turno com disclosure; dar rótulos a entrada, saída, duração e custo. | O turno mostra sua ferramenta filha e cada número tem contexto. | Pendente |
| R09 | Refino / P3 | **Atividade: Auditoria deve destacar autor, decisão e alvo.** Manter três níveis: quando, quem/decisão, comando/alvo. Mostrar consentimento e escopo ao lado da política e permitir busca. | É possível distinguir uma decisão humana, pelo telefone e por política sem depender de cor. | Pendente |
| R10 | Refino / P2 | **Conexões: Na janela estreita, selecionar um item deixa o detalhe abaixo da lista.** Abaixo de 900 px, abrir o detalhe no lugar da lista, mover foco ao título e oferecer “Voltar à lista”. Acima, manter as duas colunas. | Ao selecionar uma linha, o título do detalhe fica visível; o retorno restaura a lista e o item focado. | Pendente |
| R11 | Refino / P2 | **Chat: Memória precisa de um retorno explícito no Nest pequeno.** Adicionar “Voltar ao Chat” dentro da Memória e permitir Escape. Manter rascunho e histórico intactos. | Abrir e fechar Memória preserva o rascunho e devolve foco ao botão que a abriu. | Pendente |
| R12 | Refino / P2 | **Superfícies: Pontos de sessão e troca de tamanho têm alvos pequenos.** Ampliar área clicável sem aumentar o desenho dos pontos; usar controles de 28–32 px e texto útil de 11–12 px no desktop. | A área interativa supera o desenho do ponto; não há sobreposição entre alvos vizinhos. | Pendente |
| R13 | Refino / P2 | **Superfícies: Rótulos coloridos pequenos precisam de cores de texto mais claras.** Preservar a cor original em pontos e pássaros, mas usar variante clara para nomes; elevar a tipografia auxiliar. Não tratar esta observação como certificação de contraste. | Nomes de agentes continuam legíveis em fundo de card e de cenário, inclusive no estado desabilitado. | Pendente |
| R14 | Refino / P3 | **Ajustes: General e Chat & voice têm descrições extensas na linha do controle.** Uma frase explica a consequência; detalhes avançados ficam expansíveis. Preservar informações materiais sobre permissões, cobranças e retenção. | Título, efeito e controle são identificáveis em uma leitura curta; detalhes continuam disponíveis. | Pendente |
| R15 | Refino / P3 | **Ajustes: Agentes pode reduzir a repetição das ações de instalação.** Resumo com status e caminho; “Revisar conexão” revela diff, reinstalação e remoção. Dar prioridade ao agente que precisa de setup. | Status instalado/não conectado é visível antes das ações secundárias; diff e backup continuam obrigatórios. | Pendente |
| R16 | Refino / P3 | **Ajustes: Aprovações precisa tornar o escopo da regra imediato.** Mostrar agente, projeto e comando na própria regra, com revogação explícita; revisar o diff antes de aceitar uma política. | É claro se uma autorização vale para um comando idêntico ou para uma política mais ampla. | Pendente |
| R17 | Refino / P3 | **Ajustes: Atividade aparece como consulta e configuração em dois lugares.** Na proposta, métricas ficam na aba Atividade; Ajustes guarda retenção e exclusão e oferece um link para os números. | O caminho para consultar números e o caminho para alterar retenção são explícitos. | Pendente |
| R18 | Refino / P3 | **Ajustes: Telefone precisa de uma sequência visível de configuração.** Explicar a sequência ativar → escolher conexão → salvar endereço → gerar código; sinalizar validade de dois minutos e estado de pareamento. | O estado de conexão indica o próximo passo; pré-requisitos e expiração são claros. Backend de pareamento não foi exercitado. | Pendente |
| R19 | Refino / P3 | **Superfícies: Estados vazios podem direcionar melhor o primeiro passo.** Dar ao vazio uma ação principal direta e alternativa textual. Distinguir sem dados, carregando, falha e filtro sem resultado. | Um usuário com Zeca desligado cria tarefa pelo formulário; filtros vazios oferecem limpeza. | Pendente |
| R20 | Refino / P3 | **Superfícies: A pílula pode informar melhor a presença sem ganhar ruído.** Preservar a pílula sem texto persistente; usar nome acessível e tooltip com contagem/pendência e abrir a sessão certa ao revisar. | Mini permanece 340 × 38, abre por teclado e explica o total e a pendência no nome acessível. | Pendente |

## Reprodução

Revisão 05: **288 verificações de layout e 49 fluxos de interação**, sem falhas ou erros de página em Chromium. Resultado atualizado em [evidence/proposal-checks.json](evidence/proposal-checks.json); layout dos 13 cenários do roadmap e os novos fluxos do diretório, rotinas, critérios e tema entram no mesmo check.

Execução anterior, revisão 03: **200 verificações de layout e 31 fluxos de interação**, sem falhas ou erros de página na execução em Chromium. Os resumos iniciais cabem nas três extensões; Chat mínimo, perfis, contexto fixado, importação ZIP seletiva, reimportação, rascunho MCP e reações do Zeca foram exercitados. Resultado em [evidence/proposal-checks.json](evidence/proposal-checks.json).

Execução anterior, revisão 02: **171 combinações de layout e 31 interações**, sem falhas registradas e sem erros de página em Chromium. A reprodução confirmou mudança real dos pixels do clip, pausa manual, pausa na análise e na comparação, preferência de movimento reduzido, abertura do Mini pelo teclado e recuperação do tamanho do Compact. Resultado anterior arquivado em [evidence/verification-v2.json](evidence/verification-v2.json).

Regerar o HTML com os dados de evidência já presentes:

```sh
node design/compact-nest-review/build.mjs
node design/compact-nest-review/inspect.mjs --proposal
node design/compact-nest-review/build.mjs
```

O HTML funciona offline; ler um repositório GitHub exige rede após o clique explícito em Read repository.

O último build atualiza o rodapé com o resultado da validação. O check usa Playwright já instalado no projeto e Chromium; não acessa agentes ou GitHub.

Reexportar os sprites depois de mudar a arte original exige o Vite local:

```sh
node_modules/.bin/vite ui --host 127.0.0.1 --port 1430
# Em outro terminal:
node design/compact-nest-review/inspect.mjs --birds
node design/compact-nest-review/build.mjs
```

## Arquivos

| Arquivo | Papel |
| --- | --- |
| index.html | Entrega offline que deve ser aberta para validar. |
| Design.md | Regras e decisões visuais propostas. |
| README.md | Histórico de ajustes e critérios para decidir cada um. |
| index-template.html | Interface da revisão: modos, tamanhos, comparação, achados e galeria. |
| proposal.html | Fonte do protótipo; possui marcadores que o build preenche. Abra index.html para executá-lo. |
| findings.mjs | Nove bugs, vinte refinamentos e cobertura por área. |
| build.mjs / build-studio.mjs | Montam index.html e compilam as fontes originais do Studio com a adaptação. |
| studio-adapt.css / studio-bridge.js | Layout responsivo, companhia no Chat e comunicação com o shell; dados e fluxos continuam na fonte original. |
| theme.css / roadmap.js | Papéis visuais nativos e cenários locais do roadmap; build lê os tokens diretamente da produção. |
| birds.mjs | Reutiliza o renderer original para exportar espécies e atlas animados. |
| verify.mjs | Confere geometria, acessibilidade básica, interações e animações. |
| inspect.mjs / deep.mjs | Reproduzem a inspeção do design atual nos labs locais. |
| evidence/ | Capturas, medições, atlas e resultados dos checks. |

## Limites e próximos passos

A validação automática cobre Chromium e dados simulados. O aumento de janela ainda precisa ser validado no aplicativo Tauri, no posicionamento da ilha e na região de entrada do compositor. O protótipo não recria voos, scouts, visitantes, som, voz real ou o diretor completo de animações do produto. A voz demonstrada não acessa o microfone. Perfis, runs, importações e conversas do Studio continuam em memória; os exemplos não aparecem no Flock como sessões nativas. Esses comportamentos continuam definidos em docs/ANIMATIONS.md; aqui validamos espaço, clips em poleiro e transições.

Para registrar uma decisão, informe o ID (por exemplo, **D01 aprovado; D04 refinar os perfis; D08 aprovado**). Atualize a linha correspondente neste README e a regra em Design.md. Não transforme uma hipótese em regra aprovada sem essa validação.
