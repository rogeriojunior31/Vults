# Regras de design — Mini, Compact e Nest

Revisão 05 · 13/10/2026 · **proposta em validação**.

O [DESIGN.md da raiz](../../DESIGN.md) define as regras comuns para novos protótipos e mudanças de UI. Este documento mantém as adaptações e o histórico do HTML desta pasta. Não alteram o produto e ainda não representam decisões aprovadas. O [README.md](README.md) registra cada ajuste e seu critério de validação. Quando uma decisão mudar, atualize ambos os documentos.

## 1. Papel de cada superfície

| Superfície | Finalidade | Tamanho proposto | Alternativas para validar |
| --- | --- | --- | --- |
| Mini | Perceber o bando e uma pendência; abrir Compact. | **680 × 56 px** | 560 × 56; 760 × 56; revisão 02 em 460 × 64. |
| Compact / Quick | Ler estado, responder pedidos, conversar e consultar resumos. | **680 × 600 px** | 560 × 600; 760 × 600; revisão 02 em 600 × 760. |
| Nest | Inspecionar detalhes e configurar o produto. | **1180 × 820 px** | 760 × 640; mínimo 480 × 360; Full 1440 × 900. |

As medidas são pixels CSS no protótipo. A ilha atual tem 440 px internos com zoom 1,14, aproximadamente 502 px físicos. Implementar a ampliação exige reconciliar largura interna, zoom, tamanho da janela, posicionamento e região de entrada no Tauri; não copiar 680 para uma constante e manter o zoom sem verificar.

O modo escolhido mantém aba, rascunho e filtros. Mini e Compact compartilham uma extensão horizontal de 560, 680 ou 760 px. Alterar a largura de um atualiza a do outro; o Mini tem 56 px de altura e o Compact 600 px. Nest mantém tamanho independente. Os tamanhos anteriores continuam disponíveis para comparação e regressão. Mini abre Compact por clique ou Enter. O comparador usa o tamanho real da captura atual, sem esticá-la.

## 2. Mini ampliado

- Manter uma linha: pássaros, quantidade de sessões, estado curto e indicação de abertura.
- Não adicionar comandos de aprovação nessa superfície. Uma pendência conduz ao Compact para expor comando, pasta e alcance antes da decisão.
- Mostrar pendência por texto e marca, além da cor. Sem pendências, reduzir o destaque da borda.
- A ampliação solicitada é horizontal: aumentar a largura sem transformar o Mini em um painel alto. Quatro pássaros cabem no padrão desta revisão; o protótipo não representa uma solução definitiva para dezenas de sessões. Uma futura implementação precisa agrupar o excedente com contagem acessível.
- Sem sessões, mostrar convite curto para abrir um agente. O nome acessível descreve o estado real da demonstração.
- Preservar pixel art nítida e o comportamento da ave; não esticar suas proporções para preencher a largura.

## 3. Compact ampliado

- Usar o espaço extra para leitura e ação: margens de 18–20 px, cartões de 16 px e decisões com altura mínima de 40 px.
- A partir de 550 px de largura, mostrar estados abaixo dos nomes do bando e aves em caixas de 76 × 43 px. Em dimensões anteriores, usar a composição mais curta.
- Seis abas com ícone e nome. A antiga entrada Tarefas torna-se Agentes, com Studio, Tasks e Agents conforme a proposta original. Essa navegação ainda é uma evolução proposta.
- Flock tem uma coluna: pedido ou estado, faixa curta da sessão, debrief quando relevante. Não esticar “Nesta sessão” em uma sidebar. Histórico detalhado abre sob demanda; alterações e contexto completos continuam no Nest. Não repetir cabeçalhos de cenário e avisos que o pedido já torna claros.
- Em Atividade, quatro métricas podem ficar na mesma linha nas novas larguras. Preservar quebra em duas colunas nas menores.
- No Chat, manter retrato interativo, histórico e compositor em linhas independentes. No Chat Compact, o campo usa 56 px para duas linhas confortáveis, com anexos e controles na linha inferior; em janelas mínimas do Nest, reduz para 28 px. Enviar fica inteiro. Texto longo continua editável pela rolagem interna do campo, sem aumentar toda a janela.
- Conexões usa navegação entre lista e detalhe nessa largura, com Voltar e foco no título selecionado.
- Ajustes rápidos cobrem presença, bando e histórico. Configuração de agentes, chaves, políticas e telefone continua no Nest.

## 4. Nest responsivo

- Em 1180 px, apresentar lista e detalhe quando ajudam a consulta. Abaixo de 900 px, Conexões abre o detalhe em sua própria vista.
- Abaixo de 650 px, substituir a navegação lateral de Ajustes por seletor e permitir controles empilhados.
- Até 470 px de altura, retirar o cenário do bando para priorizar conteúdo. Aprovação pendente precisa ter os botões visíveis na primeira dobra em 480 × 360.
- Deixar conteúdo longo rolar na região central. Cabeçalho, navegação e compositor não devem depender da altura do histórico.
- Nenhum controle pode sair horizontalmente da janela. Tabelas que realmente precisam de largura usam sua própria região de rolagem.

## 5. Hierarquia e linguagem

1. Pendência que exige uma decisão.
2. Projeto, agente, comando e alcance dessa decisão.
3. Ação principal e resposta alternativa.
4. Histórico, métricas e contexto complementar.

Usar uma ação principal por decisão. Em pedidos destrutivos, Negar recebe destaque e permissão permanente não aparece. Regras permanentes precisam de uma confirmação com escopo explícito. Textos de configuração descrevem a consequência da escolha; detalhes extensos ficam em disclosure.

## 6. Espaçamento, cor e tipografia

| Token / papel | Valor de referência no protótipo |
| --- | --- |
| Fundo nativo / chrome | `#000000` / `#070809` |
| Painel / float / raised | `#141518` / `#0e0f11` / `#1c1e22` |
| Borda / linha do chrome | `#1d1f23` / `#202127` |
| Texto / secundário / muted | `#f5f6f8` / `#9398a1` / `#7a7f88` |
| Ação principal / seleção de aba | `--accent: #f5f6f8` / `--mark: #e2e6da` |
| Aviso / erro / resultado | `#f5a524` / `#ff8d97` / `#22c55e` |
| Identidade dos agentes | Tokens `--agent-*`, arte e species renderer atuais. |
| Espaços | 4, 6, 8, 12, 16, 20 e 24 px. |
| Bordas arredondadas | 7 px em controles; 9–10 px em cartões; 16 px no Mini. |
| Corpo | `--font: Inter, system-ui, sans-serif`, 12–14 px; auxiliares 10–11 px; estados das aves 9 px. |

O build lê [ui/src/themes.css](../../ui/src/themes.css) diretamente e injeta os mesmos papéis no shell e no Studio. Não usar verde sage em fundos, botões ou seleção como identidade alternativa. Verdes permanecem estados ou cor de agente; a arte original mantém a própria paleta. Fontes locais seguem o fallback do produto, sem download externo. Os overrides neutralizam as superfícies do esboço do Studio; esta é fidelidade de tokens e componentes, não certificação de equivalência pixel a pixel.

As cores indicam estado e identidade, mas não substituem texto. O Studio mantém o idioma inglês de sua fonte; esta revisão não é uma tradução completa do produto. Uso de quota deve ficar em aviso a partir de 70% e em atenção crítica a partir de 90%, alinhado ao Flock existente. Contraste precisa de verificação específica antes da aplicação no produto; esta proposta não declara conformidade completa.

## 7. Movimento

Reutilizar a arte e os clips de [docs/ANIMATIONS.md](../../docs/ANIMATIONS.md), exportados pelo renderer original. Nenhuma ave nova foi desenhada para esta revisão. A animação comunica estado, sem mover comandos, rótulos ou áreas de clique.

| Situação | Regra |
| --- | --- |
| Ave em repouso | Clip `idle`, com fase própria por sessão. |
| Trabalho | Clip compatível com a ação; a demonstração usa `edit` e `run`. |
| Pedido pendente | Clip `approval`; texto e indicador permanecem legíveis. |
| Concluído | Clip `done` termina e segura a pose final; não repete celebração indefinidamente. |
| Troca de conteúdo | Entrada de 160 ms, deslocamento de 4 px, sem bloquear a interação. |
| Memória | Entrada de 200 ms, deslocamento lateral de 10 px; Esc retorna ao Chat. |
| Diálogo | Entrada de 160 ms, deslocamento de 6 px; foco inicial em Cancelar. |
| Mini ↔ Compact ↔ Nest | Largura e altura em 280 ms; conteúdo preservado, sem atraso artificial. |
| Pendência no Mini | Brilho discreto por três ciclos de 2,8 s; indicador estático continua depois. |
| Preferência do sistema | `prefers-reduced-motion: reduce` tem prioridade; imagens em pose fixa, sem transições. |
| Controle da revisão | Caixa Animações permite pausar os clips e as transições. |
| Eficiência | Um relógio de animação por vista ativa, limitado a 30 fps; o renderer do bando pausa enquanto o Chat/Studio está aberto e o Studio pausa sem retrato ou editor animado; parar fora de tela, em documento oculto, comparação ou análise. |

O atlas inclui `idle`, `think`, `read`, `search`, `edit`, `run`, `approval`, `question`, `done`, `fail` e `listen` para atratus e aura. Exportar um clip não significa que todos os estados estejam demonstrados no HTML. Voos, scouts, visitantes e o diretor completo continuam fora deste protótipo; suas regras existentes permanecem em docs/ANIMATIONS.md.

## 8. Teclado, foco e acessibilidade

- Cada controle tem nome acessível, incluindo ações só com ícone.
- As abas usam seleção acessível, um único ponto de tabulação e setas/Home/End.
- Abrir detalhe ou diálogo posiciona o foco na nova região; fechar Memória devolve ao botão que a abriu.
- O desenho dos pássaros é decorativo; o nome da sessão e seu estado ficam no controle que o contém. Não anunciar cada frame.
- Manter foco visível. O estado de um botão não depende apenas de cor.
- Movimento reduzido mantém todas as ações disponíveis e comunica os mesmos estados.

## 9. Como promover uma proposta a regra aprovada

Validar a linha correspondente do README no HTML, registrar aprovação ou ajuste por ID, atualizar este documento e só então planejar implementação no produto. Registrar dimensões testadas e limitações nativas. Os checks cobrem a proposta em Chromium; não substituem teste em Tauri, WebKit ou compositor.


## 10. Rolagem no Compact

Não usar uma página longa como composição padrão. A largura adicional distribui informação; não é compensada por cartões mais altos. Não esconder conteúdo obrigatório com overflow sem um caminho de acesso.

| Área | Regra |
| --- | --- |
| Flock | Pedido, botões e resumo visíveis em 680 × 600; etapas abrem em diálogo. Uma permissão permanente expandida ou comando longo pode fazer sua região rolar. |
| Semana | Período, quatro métricas, gráfico e ação de exportar cabem no padrão. |
| Ajustes rápidos | Página Geral cabe no padrão; páginas completas continuam no Nest. |
| Studio / Overview | Três objetivos e continuação da execução inicial cabem nas extensões. Mais execuções podem exigir rolagem da região de conteúdo. |
| Chat vazio | Retrato, sugestões, contexto e compositor cabem; nenhuma rolagem é necessária no estado inicial do Compact. |
| Chat com mensagens | Somente o histórico rola à medida que a conversa cresce. Compositor e identidade permanecem disponíveis. |
| Tarefas, perfis, skills, integrações e registros | Listas rolam quando excedem o espaço; navegação principal e seletor de workspace ficam fixos. |
| Detalhe, editor ou diálogo | Rolagem quando o conteúdo exigir; sempre há retorno explícito à vista anterior. |

Os checks incluem ausência real de overflow vertical nos resumos iniciais de Flock, Semana, Geral, Overview, Profiles e Tasks em 560, 680 e 760 px. Isso não promete ausência de rolagem com qualquer quantidade de dados.

## 11. Zeca como companheiro no Chat

No Compact e no Nest, o retrato é um controle acessível: clique ou Enter provoca o clip startle; o ponteiro por 550 ms provoca preen. As reações não enviam mensagens nem concedem permissões. O mesmo tratamento visual pode acompanhar outro perfil sem compartilhar a memória privada de Zeca.

Ao enviar uma mensagem, mostrar think enquanto a resposta ilustrativa está pendente. Manter Parar e tentar novamente, sem perder o rascunho seguinte. Anexar mantém os metadados originais da conversa e toca swallow. Arrastar um arquivo sobre a conversa indica gape e uma área de destino; o drop usa o mesmo fluxo de anexos. Voz é uma demonstração local: listen enquanto ativa; finalizar coloca uma frase de exemplo no rascunho. Nenhum microfone é acessado.

Atalhos The flock now e Plan the next step preenchem o rascunho; o usuário decide enviar. Contexto e Memória são acessíveis em diálogo nos tamanhos menores. O Nest amplo conserva contexto lateral. Em 480 × 360, recolher atalhos repetidos para reservar pelo menos 40 px ao histórico e manter o compositor inteiro.

## 12. Adaptação do Agent Studio

Referência: [design.md original](../agent-studio/design.md) e [AGENTS.md original](../agent-studio/AGENTS.md). A adaptação compila as fontes originais com CSS de apresentação e uma ponte; não edita seus arquivos nem reproduz os importadores ou os modelos de execução.

- Studio organiza Overview, Skills, Integrations e Validation. As 187 operações são referência avançada, nunca ferramentas anunciadas como instaladas.
- Tasks e Chat mantêm a mesma execução e seu contexto. Abrir a tarefa no Chat não cria uma cópia da execução.
- Agents distingue quatro presets Vults e perfis do usuário. Copiar cria um perfil próprio; editar não substitui o preset.
- Importação ZIP/GitHub mantém descoberta, seleção de bundles, arquivos de origem, reimportação controlada e alterações locais. Criar conexão MCP mantém um rascunho sem executar servidor.
- Conversas preservam identidade, revisão de perfil e seleção de skills fixadas no início. Outra persona recebe conversa própria. Memória privada de Zeca não é implicitamente compartilhada.
- O shell da revisão fica responsável por tamanho, navegação, atenção e rodapé. O Studio embutido conserva seu estado ao mudar de aba ou modo; não reinicializar a conversa para mudar o layout.
- No Compact, o Overview vira uma síntese; listas e configuração usam regiões de conteúdo. No Nest, trilho lateral e contexto aparecem quando há largura.

A lista AgentRun do Studio é uma demonstração de execuções. A implementação futura precisa preservar TaskRow, seus critérios, execução em worktree, revisão e comandos existentes em ui/src/surfaces/nest/tasks.ts. Esta adaptação não migra essas entidades, não muda ADRs e não projeta os runs ilustrativos em Flock. A integração nativa continua pendente, incluindo ADR 0018, permissões e persistência.


## 13. Diretório de Agentes e tarefas

Agentes abre em Profiles: mostrar quem é cada especialista, o que faz e qual executor utiliza. Quatro presets devem caber no Quick inicial; novas cópias e buscas extensas podem rolar. Não usar a altura do avatar como estrutura do cartão. Copy/Edit e Chat permanecem diretos; instruções, skills e escopo ficam no detalhe. No editor, Salvar/Descartar ficam em barra sticky; retirar o trilho vazio na largura pequena para não bloquear essa barra em janelas baixas. A partir de 950 px, mostrar o perfil selecionado ao lado; abaixo disso, usar diálogo com retorno e contexto preservado.

Tasks destaca objetivo, etapa e status antes de dados técnicos. A execução tem disclosure para pausa, encerramento e detalhes; não confundir controle de resposta do Chat com parar uma sessão observada. Nova tarefa começa como rascunho com projeto, responsável e critérios. Iniciar a demonstração conserva ID, critérios e vínculo com Chat. Isso não cria worktree, driver ou registro nativo; a implementação continua obrigada a preservar TaskRow, roteamento, revisão e continuidade atuais.

Studio é configuração e investigação: Skills, Integrations, Validation e Agent support. Distinguir executor (Claude/Codex), identidade do especialista e provedor de ferramentas; nenhum catálogo planejado deve parecer uma instalação concluída.

## 14. Próximas features e seus estados

Referência temporal: roadmaps reescritos em 12/10/2026. Cada demo informa os IDs reais e permanece local. K1–K8, K10, L3–L5, M7, W2/W9b e Every agent são abertos; W1–W11 já concluídos não voltam ao backlog. Itens Dropped não são retomados.

- Debrief ocupa uma faixa no Quick e detalha sob demanda. Cartão pendente vem primeiro; limite de atenção não pode ocultá-lo. Em foco/tela cheia, falas espontâneas são silenciosas, não pedidos.
- Explicação de risco conserva comando e pasta. Pasta externa à solicitada precisa aparecer explicitamente. Leituras em lote mantêm IDs, seleção e decisão por pedido; itens não selecionados continuam pendentes.
- Pergunta textual não vira aprovação. Requer resposta não vazia; texto é escapado na apresentação.
- Rotina começa desligada. Editar alvo, gatilho, escopo ou orçamento volta a rascunho; ativação pede revisão. Pode ser preparada no Chat e editada em Tasks. Sem agendamento real neste HTML.
- Aprendizado e skill são propostas de diff, com caminho e fluxo backup → revisão → clique. Nunca escrever instruções silenciosamente.
- Humor usa clips atuais para estudar K7; olhar direcionado e atenção nativa continuam pendentes. Voz K8 é opt-in com modelo/orçamento; não responde permissões nem acessa o microfone da demo.
- Android representa aprovação biométrica forte, negação sem biometria e expiração. O desktop é a autoridade; UI sem assinatura e attestation real nunca conta como validação do protocolo.
- Custos observados começam desligados e identificam a origem OTLP. Receptor local, configuração por diff/backup/clique; estimativa de API não equivale à cobrança de assinatura.
- Suporte de agente separa Watch / Answer / Drive. Observe-only informa terminal; planejados ficam não verificados. G1/G2 não habilita ferramentas genéricas antes da aprovação de ADR 0027.
- O Flock board é um storyboard dos 12 IDs. Não anunciar os clips atuais como novos sprites finalizados: decolagem, fio, sexo/idade/anilha, temperamento, trabalho, retorno e visitantes precisam da arte aprovada.

Os critérios de validação e as limitações de cada ajuste estão em D01–D13 do README. Não alterar produção ao validar uma direção visual.


## 15. Chat Compact — revisão 05

Usar um cabeçalho com destinatário, executor, Nova conversa e Histórico. A faixa do companheiro mantém origem, estado e um único conjunto de Contexto/Memória; não repetir as mesmas ações junto do compositor. No estado vazio, os prompts existentes preenchem o rascunho. Ao crescer a conversa, o histórico ocupa essa área e rola independentemente do campo.

O Histórico abre as conversas do mesmo owner, sem duplicar dados ou perder rascunhos. No Compact, usar campo de 56 px com controles separados e preservar enviar/parar, anexos, voz e skills. A rotina no protótipo continua uma demonstração de K4; o ajuste nativo de layout não deve anunciar uma rotina funcional antes de sua entrega.

A execução está em [plan-ui.md](../../docs/dev/plan-ui.md): o lab UX0 vem primeiro. As regras gerais daqui permanecem subordinadas ao DESIGN.md da raiz e aos ADRs de comportamento; este arquivo não é outro backlog.
