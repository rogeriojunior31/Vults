# Ditado
<!-- source: 9a3d354e189d -->

A versão de desenvolvimento da N3 oferece ditado local para a área de transferência, com colagem
opcional em editores de texto compatíveis no X11 nativo ou Plasma Wayland e painéis tmux exatos
do bando. Inclui indicações medidas na ilha, Histórico com busca, Dicionário/Trechos,
estatísticas na Atividade e Configurações com Diagnóstico. Nada é enviado automaticamente. Recursos presentes apenas no protótipo aprovado separado
não estão habilitados aqui.

<a id="voice-available-today"></a>

## Voz disponível hoje

Use a [entrada de voz do Chat](chat.md#speaking-instead-of-typing) depois de escolher um modelo local em
**Settings > Chat & voice**. Cada gravação do Chat guarda até um minuto de áudio na memória.
A prévia e a transcrição final usam o mesmo modelo local; você revisa o rascunho antes de enviar.

A captura de áudio compartilhada agora converte a entrada para mono a 16 kHz, mantém um buffer
limitado e informa falhas de dispositivo ou captura, em vez de retornar áudio incompleto
silenciosamente. Essa mudança não adiciona downloads automáticos de modelos.

A detecção de fala filtra ruídos breves e mantém pausas curtas dentro de uma frase. No modo
de toque para falar, o Chat continua esperando cerca de 1,5 segundo de silêncio depois da
fala antes de encerrar; o silêncio antes de você falar não encerra a gravação.

Por padrão, a prévia e a transcrição final compartilham um filtro local de texto. Sequências de seis ou
mais repetições de palavras ou frases são reduzidas a uma ocorrência (unidades de até 60
caracteres); repetições mais curtas são mantidas. Marcações de som conhecidas são removidas,
enquanto outros textos entre parênteses ou colchetes permanecem intactos. Um resultado isolado
associado a silêncio, como "Thank you.", "Obrigado." ou "Gracias.", é omitido, mesmo quando dito
intencionalmente; frases maiores que contenham essas palavras são mantidas. Você pode editar o
rascunho antes de enviar. Esse filtro não usa modelo nem faz requisições de rede.

<a id="global-dictation-first-slice"></a>

## Ditado global (primeira etapa)

Para o primeiro ditado, escolha um modelo, coloque o foco no destino, segure o atalho de fala,
fale e solte. Aguarde **Copiado** e cole ali. A inserção automática é uma escolha separada e
só funciona nos destinos abaixo; nunca envia Enter.

Escolha e instale um modelo local de voz em **Settings > Chat & voice**. O desktop pede os novos
vínculos de atalhos globais; **Settings > Dictation > Keyboard shortcuts** mostra as teclas reais e
é atualizado quando você as altera nas configurações do desktop. **Alterar no KDE…** abre
Configurações do sistema do KDE > Atalhos pela entrada instalada do desktop. Se ela estiver
ausente ou não puder iniciar, a página permite tentar novamente; outros desktops usam suas
próprias configurações de atalhos. O Vults não edita os vínculos por conta própria. Teclas sugeridas:

- **Ctrl+Alt+V**: fora do Vults, segure por pelo menos 300 ms, fale e solte. A captura mantém os
  últimos 600 ms antes da transcrição local. Dois toques curtos, com o segundo até 400 ms após
  soltar a tecla no primeiro, mantêm a gravação com as mãos livres; o próximo toque encerra.
  Um toque curto sozinho é descartado.
- **Ctrl+Alt+X**: cancela o ditado ou a voz do Chat, inclusive um ditado que já esteja transcrevendo.
- **Ctrl+Alt+B**: copia novamente o último ditado concluído. Não interrompe uma gravação ativa.

Um ditado ativo mantém seu dono se o foco mudar. O Chat do Vults em foco usa seu caminho de
rascunho existente; o atalho de fala não inicia gravação a partir de Configurações. O teste separado de microfone
só começa com **Testar microfone** no Diagnóstico. Com o Zeca desligado, o ditado independente
continua funcionando. Em **Settings > Dictation > Keyboard shortcuts**, escolha **Segurar +
duplo toque** (o padrão) ou **Só segurar**. Só segurar descarta toques curtos imediatamente;
um segundo toque inicia uma nova gravação segurada. A trava por Espaço continua sendo uma
escolha independente mesmo nesse modo. **Espaço mantém a gravação** e **Esc cancela o ditado**
vêm ligados e podem ser alterados no X11; os controles ficam indisponíveis no Wayland. Com a
trava desligada, Espaço age como outra digitação e pode descartar uma gravação segurada. Com o
cancelamento por Esc desligado, Esc deixa o ditado continuar; o atalho global Cancelar voz sempre
funciona. As escolhas ficam salvas localmente e valem a partir da próxima gravação, inclusive
após reiniciar; alterá-las nunca interrompe uma gravação ou transcrição ativa.

Com as escolhas padrão no X11 com XI 2.1 disponível,
pressione **Espaço** enquanto segura o atalho de fala para continuar gravando com as mãos livres.
**Esc** cancela durante a gravação, a cauda da captura ou a transcrição. Outras teclas que não são
modificadoras descartam uma gravação segurada ou um toque único pendente; digitar normalmente não
cancela uma gravação já travada com as mãos livres. O listener não consome essas teclas no aplicativo
de destino. As combinações reais de Falar e Copiar novamente ficam fora do cancelamento por outra
tecla; a repetição automática é ignorada. A trava por Espaço impede que o atalho padrão de Abrir a
ilha mova o foco durante a gravação. Se Abrir for remapeado para outra tecla, mantém sua ação usual.

No Wayland, use o toque duplo e o atalho separado de cancelamento; a trava por Espaço e Esc global
não estão disponíveis. Nenhuma tecla bruta é observada pelo XWayland. Se um listener X11 ou seu
atalho real não puder ser reconhecido, os controles do portal para segurar, toque duplo e cancelar
continuam funcionando. A observação de teclas termina com a gravação e seu processamento, sem
guardar texto digitado nem histórico de teclas.

**Settings > Dictation > Voice > Microfone do ditado** escolhe uma entrada para o ditado local
independente; o Chat continua usando o padrão do sistema. A página lista as entradas sem gravar,
e **Atualizar microfones** atualiza essa lista explicitamente. Carregamento, falhas de leitura,
listas desatualizadas e ausência de entradas têm feedback separado. Um microfone salvo que foi
desconectado continua selecionado como indisponível; se não for encontrado ao iniciar a captura,
o fallback existente escolhe o padrão do sistema ou outra entrada. Isso não reescreve a preferência
salva. Uma entrada listada que não puder ser aberta mostra um erro. Escolha **Microfone padrão do
sistema** para limpar a preferência. A escolha fica salva neste computador e é usada pela próxima
gravação; uma gravação ativa mantém sua entrada. Abrir a página ou atualizar nunca inicia uma gravação.

**Settings > Dictation > Voice** também reúne **Remover repetições em sequência** e **Tentar
com Base quando o ditado vier vazio**; as duas opções vêm ligadas. Desligar a remoção mantém as
repetições na prévia e no texto final. Marcações de som conhecidas e frases isoladas associadas
a silêncio continuam sendo removidas. Desligar a nova tentativa com Base pula a verificação extra
de fala e a segunda transcrição de um resultado vazio. As escolhas ficam salvas localmente e são
capturadas ao iniciar uma nova gravação independente; alterá-las nunca muda um ditado ativo nem
o processamento do Chat. Nenhuma dessas escolhas baixa ou contata um modelo.

O status temporário fica na ilha ou no Nest quando ele está aberto. Mini e Quick reservam uma
faixa neutra acima do bando, com ícone e texto para escuta, captura com mãos livres,
finalização, transcrição, nova tentativa e cada resultado de entrega. Linhas longas são cortadas
dentro dessa faixa. Ela não pega o foco. Um card aguardando resposta ou a tela bloqueada oculta
o feedback imediatamente; um feedback expirado não volta quando o card sai. A ilha mantém sua
superfície nativa de tamanho fixo e contabiliza a faixa antes de aplicar o zoom do Quick e a
ancoragem superior ou inferior.

Durante a escuta e a captura com mãos livres, nove bandas acompanham o áudio medido do microfone.
Em uma GPU compatível, a prévia local de voz existente também mostra, na mesma faixa, as palavras
ouvidas até aquele momento. Essas palavras provisórias podem mudar antes da transcrição final;
a captura apenas pela CPU mantém as bandas e o estado, seguidos do resultado final de cópia ou
inserção. O texto da prévia fica na memória, é descartado quando a captura termina e nunca vai
para o log ou histórico do aplicativo. Cancelar ou iniciar outro ditado rejeita atualizações
atrasadas de áudio e prévia. O movimento reduzido oculta as bandas móveis e mantém o estado e as
palavras.

**Settings > Dictation > Voice > Som de início do ditado** controla um aviso curto;
ele começa ligado e acompanha a opção de sons, o volume e o Não Perturbe do aplicativo. Somente
uma nova gravação toca o aviso. Abrir outra superfície, mudar essa opção ou sair do silêncio
não repete o som. Falhas e cancelamentos ficam silenciosos.

**Settings > Dictation > Voice > Legendas do ditado** controla a legenda opcional;
ela começa ligada. A legenda expande uma linha cortada ou mostra o texto final depois de uma
cópia ou entrega confirmada, em até duas linhas. Ela usa o espaço livre abaixo da ilha ancorada
no topo ou acima da ilha ancorada na parte inferior; um Quick que ocupa toda a altura deixa a
legenda oculta. No Nest ou em Settings, ela fica acima da linha temporária de feedback, com
espaço reservado na página para manter os controles visíveis. Cards e bloqueio de tela a ocultam
imediatamente. A legenda não recebe cliques nem foco e não amplia a
região da ilha que recebe o mouse. Ela some em até dois segundos após o término; trocar de
superfície ou alternar a opção de legendas não renova esse prazo. O texto fica na memória. Ditado tem sua própria categoria em Settings.

**Copiado** significa que o texto foi relido e conferido na área de transferência; use o
comando Colar do aplicativo de destino (geralmente Ctrl+V, ou Ctrl+Shift+V em terminais).
Nada é enviado automaticamente.

A entrega confirmada também mostra o total de palavras do texto final, com rótulo localizado
no singular ou plural. A contagem usa limites Unicode entre palavras: pontuação e emojis não
contam, e ideogramas chineses contam individualmente. O total acompanha a linha de entrega mesmo
depois que a legenda some. Gravação, falhas e cancelamentos não reutilizam uma contagem anterior.
O total de correções aguarda a etapa de métricas; ainda não é exibido.

No Wayland, a cópia em segundo plano exige suporte do desktop a `ext-data-control` ou
`wlr-data-control` para a área de transferência. Se indisponível, o ditado informa um erro em vez
de dizer Copiado; ele nunca toma o foco nem grava silenciosamente apenas no XWayland. O acesso
à área de transferência usa texto na memória, sem arquivo de transcrição. O log do aplicativo
registra as fases do ditado sem registrar o texto.

No primeiro pressionamento do atalho de fala, o Vults solicita a classe, o título, o identificador
de processo e a identidade da janela em foco, sem ativar uma janela. No X11, ele verifica novamente
a janela em primeiro plano depois da leitura. No Plasma Wayland, usa um script de leitura do KWin;
essa observação assíncrona pode ocorrer logo depois do pressionamento, por isso serve como contexto,
e não como prova do destino exato. Observações atrasadas ou que falham permanecem desconhecidas.
No GNOME e no wlroots Wayland, o aplicativo também fica desconhecido, mesmo com XWayland disponível.
Quando conhecido, o status de cópia informa o aplicativo observado e sugere Ctrl+Shift+V para um
terminal reconhecido. Uma correspondência de título ou processo não comprova qual painel do terminal
está em foco nem permite inserção. Processos de terminal compartilhados por várias sessões do bando
deixam a associação com a sessão desconhecida. Títulos e identidades nativas de janelas ficam
na memória e não são enviados ao renderizador, gravados no histórico nem registrados em logs.
O aplicativo e a sessão reconhecidos podem acompanhar uma transcrição concluída, conforme o
prazo de retenção escolhido. Copiar novamente começa sem reutilizar esse contexto de aplicativo.

<a id="supported-destinations"></a>

## Destinos compatíveis

O ditado independente funciona com o Zeca ligado ou desligado. Os caminhos atuais de entrega
são limitados; reconhecer um aplicativo não concede autorização de inserção.

| Destino | X11 nativo | Plasma Wayland |
| --- | --- | --- |
| Campo de texto no Firefox | Copiado; cole manualmente | Copiado; cole manualmente |
| Kate | Copiado; cole manualmente | Copiado; cole manualmente |
| Konsole, inclusive tmux dentro do Konsole | Copiado; cole manualmente | Copiado; cole manualmente |
| KWrite, GNOME Text Editor, Mousepad, Leafpad | Colagem opcional após verificar foco e área de transferência | Colagem opcional com autorização de teclado pronta e verificações de foco/área de transferência |
| Sessão rastreada do bando em painel tmux exato hospedado no XTerm/UXTerm ou Kitty | Entrega opcional literal de uma linha após verificar sessão/painel atuais | Mesmas verificações, além da autorização de teclado pronta |
| Chat do Vults em foco com o Zeca ligado | Preenche o rascunho sem enviar | Preenche o rascunho sem enviar |

Firefox, Kate e Konsole mantêm a colagem manual mesmo com a inserção ligada. Painéis diretos
do Kitty ou WezTerm também ficam apenas na cópia sem o caminho tmux exato compatível. Uma
alternativa de cópia informa **Copiado**, em vez de **Colado** ou **Na sessão**. No Wayland, copiar
também exige o protocolo de área de transferência descrito acima; pode falhar explicitamente
quando ele não está disponível.

Com o Zeca desligado, a tecla de fala usa ditado independente até sobre o Nest ou a ilha; não
há rascunho do Chat a preencher. Configurações não inicia uma gravação. Um ditado independente
já ativo mantém seu dono original quando o foco muda; voltar o foco nunca restaura uma
autorização de inserção.

<a id="cancel-by-voice"></a>

## Cancelar por voz

**Settings > Dictation > Keyboard shortcuts > Cancelar por voz** vem ligado e funciona no X11
e no Wayland. Encerre a gravação normalmente; a transcrição final é verificada antes da conversão
de código falado e de qualquer cópia ou inserção. Estas frases inteiras cancelam quando ditas
sozinhas ou no final da transcrição:

| Idioma | Frases de cancelamento |
| --- | --- |
| Inglês | `cancel`, `never mind` |
| Português | `cancela`, `deixa pra lá` (também sem o acento) |
| Espanhol | `cancela`, `déjalo` (também sem o acento) |

A comparação ignora maiúsculas, minúsculas, espaços extras, os acentos agudos das frases e a
pontuação final. Uma frase no final precisa começar após um espaço; `cancel the meeting at noon`,
`path/cancel`, `cancel_by_voice` e `"cancel"` entre aspas continuam como texto comum. É uma regra
literal de sufixo: até `src slash cancel` cancela o ditado. Desligue a opção para ditar conteúdo
que termine com uma dessas frases.

Uma correspondência mostra **Cancelado**, não copia, insere ou envia nada e não acrescenta
histórico nem totais de uso. O último ditado bem-sucedido continua disponível para Copiar novamente.
A reutilização usa o texto já entregue literalmente, então reutilizar `cancel` não aciona esta
verificação. Isso vale só para o ditado independente; a entrada de voz do rascunho do Chat mantém
seu comportamento existente. A opção fica salva localmente e é capturada no início da gravação,
por isso uma mudança não afeta o ditado ativo. Não acrescenta escuta contínua de comandos por voz
nem baixa modelos.

<a id="your-words-processing"></a>

## Processamento das suas palavras

O ditado independente agora aplica as entradas locais de Suas palavras após a transcrição final.
Abra **Ditado > Dicionário** para adicionar/remover palavras e salvar regras de substituição.
Salvar a mesma origem atualiza seu texto sem mudar a ordem. **Trechos** adiciona gatilhos curtos
e conteúdo literal, inclusive espaços e quebras de linha. Excluir exige confirmação; falhas ao
salvar preservam o rascunho para tentar novamente. Essas páginas não alteram a voz do Chat.

**Configurações > Ditado > Suas palavras > Ajudar o modelo com estas palavras** fica desligado
por padrão. Quando ativado, só o Turbo com inglês
fixo recebe o contexto de programação mais as palavras do dicionário, limitado a 224 tokens
do modelo com entradas inteiras. O inglês pode ser escolhido diretamente ou resolvido pelo idioma
do sistema; a detecção automática fica excluída. Prévia e transcrição final usam a mesma visão capturada na
gravação. O conteúdo de substituições e trechos nunca entra nesse contexto, que não persiste no
modelo em cache nem no Chat. Uma nova tentativa com Base fica sem contexto de dicionário.
Base, Small, português, outros idiomas e detecção automática não recebem contexto do dicionário:
só Turbo/inglês
melhorou o WER e manteve silêncio/ruído vazios na comparação sintética local. Esses testes não
estabelecem a precisão com fala humana; a validação real pelo microfone continua pendente.
As regras locais após a transcrição abaixo funcionam com qualquer idioma de transcrição,
independentemente do vocabulário do modelo. Nenhuma das opções de vocabulário precisa estar
ligada para usar substituições ou trechos.

**Configurações > Ditado > Suas palavras > Vocabulário do projeto** fica desligado por padrão,
independentemente da outra opção. Quando ativado, o Vults mantém em cache nomes de projetos Git conhecidos: pacotes e dependências Cargo/npm, membros próximos do
workspace, nomes de módulos, arquivos da raiz e branch. Lê manifests pequenos e nomes de arquivos
localmente; não lê o conteúdo do código, executa scripts de pacotes nem baixa nada. A atualização
ocorre no início da sessão e na atualização existente do repositório (no máximo uma vez por minuto
em eventos posteriores dos agentes).

A pasta da sessão capturada seleciona a árvore de trabalho. Caso contrário, o título de um terminal
ou editor reconhecido precisa nomear exatamente um projeto em cache; navegadores e títulos
desconhecidos ou ambíguos não acrescentam nada. Esse contexto aproximado de primeiro plano nunca
autoriza digitar. A primeira decodificação de prévia/final fixa os nomes, inclusive um contexto
ausente, para toda a gravação. Os nomes do projeto vêm depois das palavras do dicionário pessoal,
sob o mesmo limite de 224 tokens e restrição Turbo/inglês. Nunca viram regras salvas do dicionário
nem alteram o fluxo de grafia. Outros modelos/idiomas, novas tentativas com Base e Chat mantêm seu
comportamento existente. Isso reutiliza a qualificação da V2.3; não estabelece a precisão para
todo nome de projeto nem substitui a validação real pelo microfone.

**Testar uma frase** no Dicionário executa as regras locais realmente salvas. Marca as partes
alteradas; **Contexto de terminal** inclui código falado se a opção em Configurações estiver
ligada. O cancelamento falado também usa a opção salva atual e ocorre antes das suas regras.
Cada teste lê uma nova visão das regras, sem iniciar microfone/modelo, alterar a área de
transferência ou salvar no histórico. Mudar a frase ou as regras descarta um resultado antigo.
Isso mostra o processamento de texto, não o reconhecimento de fala nem permissão de destino.
**Como é aplicado** explica a ordem e os limites.

No início da gravação, o Vults lê uma visão consistente do dicionário, das substituições e dos
trechos. Edições feitas durante essa gravação valem para a próxima. Se não puder ler essa visão,
a gravação falha explicitamente antes da captura, em vez de ignorar as entradas silenciosamente.

A ordem é: proteger gatilhos de trechos, aplicar substituições na ordem salva, corrigir grafias
simples do dicionário, converter código falado num terminal/sessão reconhecido e restaurar o
conteúdo literal dos trechos. Esse conteúdo mantém espaços, quebras de linha, cifrões e pontuação;
nenhuma regra posterior ou conversão de código falado o reescreve. Texto parecido com um marcador
interno continua sendo texto comum. Substituições inserem texto literal, sem expansões de expressão
regular; regras posteriores podem corresponder ao resultado da regra anterior, seguidas pela
grafia do dicionário.

A comparação ignora caixa com chaves Unicode e exige limites de palavras inteiras. Entre as partes
de uma frase, espaços, sublinhados, hifens e outros separadores que não formam palavras são
intercambiáveis; letras, dígitos e marcas combinantes continuam dentro das palavras. Caracteres de
controle, inclusive tabulações e quebras de linha, são barreiras e nunca são consumidos por uma
correspondência. Grafias do dicionário só se aplicam a entradas com pelo menos duas letras/dígitos
e compostas apenas de letras, dígitos, espaços, hifens ou apóstrofos retos. Outras entradas ainda
podem ser usadas como vocabulário opcional do modelo.

Por exemplo, substituir `cloud code` por `Claude Code` também corresponde a `CLOUD-CODE`; uma
entrada `Vults` corrige `vults` sem mudar `prevults`. O conteúdo de um trecho nunca é executado
como código nem enviado. Saídas com quebras de linha ou controles continuam sujeitas à cópia
alternativa e às restrições existentes de entrega ao terminal.

O cancelamento falado é verificado no texto ouvido **antes** dessas regras. Expandir um trecho
para `cancel`, portanto, escreve esse texto literal; dizer `cancel` ainda descarta antes da
expansão. O texto ouvido original e o escrito final continuam separados no Histórico. Copiar
novamente e reutilizar pelo Histórico usam o texto final sem reaplicar as regras. Os números de
correções continuam indisponíveis até sua etapa de métricas. O processamento para com um erro
explícito acima de 256 KiB de entrada ou saída intermediária, sem cópia ou inserção parcial.

<a id="check-your-words-locally"></a>

## Confira Suas palavras localmente

1. No **Dicionário**, adicione `Vults` e uma substituição de `cloud code` por `Claude Code`.
   Uma palavra duplicada é recusada; salvar a mesma origem atualiza a substituição existente.
2. Em **Trechos**, adicione `signature` com este conteúdo exato:

   ```text
   Regards,
   Ada
   ```

3. De volta ao **Dicionário**, teste `cloud code vults signature`. O resultado deve conter
   `Claude Code Vults Regards,`, seguido de `Ada` na próxima linha. As partes alteradas são marcadas;
   a quebra de linha do trecho salvo fica literal. Teste `source slash main` com **Contexto de
   terminal** desligado e depois ligado: só vira `source/main` se **Código falado** estiver ligado
   em Configurações.
4. Com **Cancelar por voz** ligado, teste `cancel`: informa cancelamento antes das regras.
   Esses testes não copiam, inserem nem acrescentam Histórico. Exclua uma entrada de teste,
   cancele a confirmação uma vez e depois confirme: só a confirmação explícita a remove.
5. Para validar pelo microfone, dite no seu aplicativo externo habitual e confira o texto final,
   o resultado real Copiado/Colado e, quando o histórico for guardado, os textos ouvido/escrito no Histórico. Mantenha as
   [restrições de destino](#supported-destinations) existentes; trechos nunca autorizam enviar texto.
   Uma mudança de regras durante a captura vale para a próxima gravação; Copiar novamente reutiliza
   o texto final.

As primeiras quatro verificações testam o processamento local sem baixar nem habilitar um recurso
de modelo. Se também testar o vocabulário do Whisper, use um Turbo instalado com inglês escolhido
diretamente ou resolvido pelo sistema e ative apenas o contexto desejado. Base/Small, português e
detecção automática mantêm seus prompts anteriores mesmo com as opções ligadas. Nomes de projeto
exigem um projeto Git conhecido em cache após início da sessão/atualização do repositório;
contexto de primeiro plano desconhecido ou ambíguo não acrescenta nada. Compare com as opções
desligadas; isso não promete melhora para um nome ou gravação de microfone. Restaure as preferências
que alterou para o teste.

<a id="music-while-you-speak"></a>

## Música enquanto você fala

**Settings > Ditado > Voz > Música enquanto você fala** oferece **Baixar para 25%** (o padrão),
**Pausar** e **Não alterar**. A próxima gravação independente mantém essa escolha até parar,
inclusive no cancelamento. A voz do Chat e a futura captura de reuniões não usam esse controle.

No Linux, o Vults controla temporariamente apenas players MPRIS que já estão tocando. A redução
limita o volume do próprio player a 25%; players mudos ou mais baixos ficam como estão. Ao parar
a captura, restaura o volume original de cada player ou retoma aqueles pausados por esta gravação.
Um player cujo estado relevante ou volume mudou nesse intervalo não é restaurado, mesmo que depois
tenha voltado a 25% ou Pausado. Players que fecharam e processos substitutos nunca são retomados
ou restaurados.

Isso usa controles locais de mídia sem ler metadados das faixas nem fazer requisições de rede.
Funciona independentemente de **Tocando agora**, que controla a música exibida no aplicativo.
**Não alterar** não abre uma conexão de controle de mídia. Players ou o barramento de sessão
indisponíveis nunca impedem o ditado, e a falha de um player não impede restaurar os outros.
A restauração ocorre em segundo plano; a captura do microfone não espera por um player.

<a id="spoken-code"></a>

## Código falado

**Settings > Dictation > Terminais e sessões > Código falado** vem ligado. Para um terminal
ou sessão reconhecido e capturado para o ditado, une partes de identificadores em torno destes termos:

| Idioma | Termos falados para `.`, `/`, `_`, `-`, `:` |
| --- | --- |
| Inglês | `dot`, `slash`, `underscore` ou `underline`, `dash`, `colon` |
| Português | `ponto`, `barra`, `underline` ou `underscore`, `traço` ou `traco`, `dois pontos` |
| Espanhol | `punto`, `barra`, `guion bajo` ou `guión bajo`, `guion` ou `guión`, `dos puntos` |

Por exemplo, `src slash main dot rs` vira `src/main.rs`. Maiúsculas, minúsculas e pontuação
existente são preservadas: não acrescenta capitalização nem ponto final. Um marcador precisa
ter partes semelhantes a identificadores dos dois lados. Espaços fora de uma sequência convertida
ficam intactos; quebras de linha, tabulações e caracteres de controle nunca são removidos para
criar uma sequência. É uma sintaxe literal de pontuação, sem interpretação de prosa: desligue
quando quiser essas palavras literalmente em um terminal.

Aplicativos de prosa, destinos desconhecidos e o Chat do Vults em foco mantêm seu texto. Mudar a
opção vale para a próxima gravação. O fallback de um terminal para a área de transferência mantém
o texto convertido; a formatação nunca autoriza inserção, envia Enter ou contorna uma sessão
aguardando. Copiar novamente e o Histórico reutilizam o texto já entregue sem formatá-lo outra vez.
Quando há retenção, **Mostrar o que foi ouvido** no Histórico mantém a transcrição anterior à
conversão de código falado junto do resultado escrito. Não acrescenta áudio nem conteúdo anterior
da área de transferência ao histórico.

<a id="opt-in-x11-insertion"></a>

## Inserção opcional no X11

Em **Settings > Dictation > Paste**, ative **Inserir ditado em editores e painéis tmux compatíveis** apenas
se quiser que o Vults insira texto. A opção começa desligada, inclusive após atualizar.
Esta primeira implementação aceita KWrite, GNOME Text Editor, Mousepad e Leafpad. Outros aplicativos e
IDEs mantêm o caminho de cópia; a entrega no terminal exige as verificações exatas de tmux abaixo. O X11 nativo é descrito abaixo; o Plasma Wayland usa
a autorização de teclado separada na próxima seção. Não há desvio por XWayland.

No X11 nativo, o callback do atalho captura um destino separado antes de enfileirar a gravação.
Ele confere a janela ativa, o filho exato com foco de teclado, a classe, o processo e sua hora de
início; depois observa a perda de foco até a entrega. Mudar o foco e voltar também impede a inserção.
O contexto assíncrono do aplicativo descrito acima nunca autoriza colar. Imediatamente antes de
Ctrl+V, o Vults reconfere esse destino, verifica o dono da área de transferência e exige teclas
soltas. Ele não ativa outro aplicativo, solta seus modificadores pressionados nem envia Enter.
Uma verificação que falha deixa o texto copiado. Terminais nunca recebem Ctrl+Shift+V automaticamente;
o caminho tmux abaixo usa bytes literais.

A inserção assume temporariamente a área de transferência de texto. Após o despacho, o Vults espera
cerca de 700 ms e restaura o texto anterior apenas enquanto esta reserva exata ainda for a dona.
Uma cópia mais recente prevalece mesmo quando o texto é igual ao ditado. Ditados sobrepostos herdam
o texto original; uma reserva antiga não restaura por cima da nova. Se o resultado for apenas uma
cópia, o ditado fica disponível. O conteúdo anterior fica na memória e nunca vai para histórico,
logs ou arquivos.

A reserva restaura apenas texto simples, de até 64 KiB; não restaura imagens nem formatos ricos.
Texto anterior ou ditados maiores seguem o caminho de cópia comum. A indicação de exclusão do
histórico depende do suporte do desktop. Settings explica esses limites antes de ativar a inserção.
**Colado** confirma o despacho para o destino em primeiro plano validado; revise o resultado no
editor. Isso não confirma que o editor consumiu, enviou ou salvou o texto.

<a id="plasma-wayland-insertion"></a>

## Inserção no Plasma Wayland

A mesma opção de inserção, desligada por padrão, aceita Plasma Wayland com `ext-data-control`
e um único assento de área de transferência. Ativá-la solicita **apenas entrada de teclado** pelo
portal RemoteDesktop. O desktop concede a capacidade de digitar em outros aplicativos; o Vults
limita seu uso à colagem nos editores compatíveis acima e à entrega exata em tmux abaixo. Não solicita controle do ponteiro nem
compartilhamento de tela.

Settings mostra se o acesso ao teclado está aguardando, pronto, temporário ou indisponível.
Ditados seguintes reutilizam a sessão aberta. Uma recusa, portal ausente, falha na chamada de
entrada ou autorização revogada mantém o ditado copiando. O próximo ditado não abre outro diálogo;
**Solicitar acesso ao teclado** em Settings é a tentativa explícita. Outros desktops Wayland
mantêm o caminho de cópia.

Um token reutilizável de restauração fica apenas no chaveiro do sistema, na conta do aplicativo.
Ele é removido antes do uso e substituído pelo novo token do portal. Sem chaveiro disponível,
uma sessão aprovada funciona nesta execução sem persistir um token. Ao reiniciar, o Vults tenta
restaurar apenas quando consegue ler e consumir um token existente; caso contrário, espera sua
solicitação em Settings. O desktop pode perguntar novamente se não aceitar mais o token.

Desligar a inserção invalida imediatamente as entregas pendentes, fecha a sessão do portal e
remove o token salvo. Se o chaveiro não conseguir removê-lo, a inserção continua desligada e
Settings mostra a falha com **Tentar remover o token novamente**. Sair fecha a sessão e mantém um
token utilizável para o próximo início. Nenhum token vai para configurações, histórico, logs,
renderizador ou arquivo.

Um monitor KWin somente de leitura funciona enquanto a autorização estiver ativa, antes do
atalho de fala. Ao pressionar, o Vults captura a identidade atual da janela e a geração de foco.
Um checkpoint do compositor antes da entrega precisa confirmar identidade, geração, classe,
processo do mesmo usuário e hora de início do processo; mudar o foco e voltar também impede a
inserção. Uma janela do Vults ganhar foco invalida o destino capturado. Checkpoints ausentes ou
tardios deixam o texto copiado. Esse monitor não guarda títulos nem histórico de teclas e nunca
ativa uma janela. O caminho tmux usa essa mesma capacidade de janela.

A reserva de texto no Wayland usa o mesmo limite de 64 KiB e a retenção em memória do X11. A
restauração muda os bytes fornecidos pela própria fonte, sem reassumir a seleção: uma cópia mais
recente prevalece mesmo se seu evento de cancelamento ainda não chegou. A indicação de exclusão
do histórico depende do desktop. Vários assentos ou falta de `ext-data-control` levam à cópia
comum. O despacho validado de Ctrl+V bem-sucedido mostra **Colado**, com os mesmos limites acima.

A captura é limitada a dez minutos e fica na memória. Áudio muito baixo ou muito curto não gera
texto. Com **Tentar com Base quando o ditado vier vazio** ligado (o padrão), se o modelo selecionado
não retornar texto, mas o Silero detectar fala, um modelo Base já instalado pode tentar uma vez,
com indicação visível; o Base não é baixado para essa tentativa.
Ditados independentes concluídos agora atualizam a página de Histórico e os totais de uso locais.
O último texto concluído também fica na memória para Copiar novamente até desligar a voz,
bloquear a tela ou sair do Vults. O texto copiado pode permanecer
na área de transferência do sistema ou no gerenciador dela.

<a id="exact-tmux-session-delivery"></a>

## Entrega exata em sessões tmux

A opção de inserção também aceita uma sessão do bando em **tmux dentro do XTerm/UXTerm ou Kitty**.
O Kitty precisa do CLI `kitten` e já deve permitir consultas somente de leitura em seu socket Unix
do mesmo usuário; o Vults não edita a configuração nem ativa o controle remoto. Outros terminais,
metadados ausentes de socket/painel, clientes compartilhados ou ambíguos e entrega direta em
Kitty/WezTerm ficam apenas na cópia. A [implementação de send-text do Kitty](https://github.com/kovidgoyal/kitty/blob/master/kitty/rc/send_text.py)
não confirma a entrega, portanto seu resultado de sucesso não basta. A
[camada de cópia/pesquisa do WezTerm](https://github.com/wezterm/wezterm/blob/main/wezterm-gui/src/overlay/copy.rs)
compartilha o ID do painel original; seu CLI não permite confirmar qual campo de entrada o possui.
A entrega direta, portanto, continua na cópia. **Abrir terminal** pode navegar para a instância
exata de WezTerm registrada pelo hook, mas isso não autoriza digitar nela.

Ao pressionar o atalho, o Vults captura o painel tmux ativo exato, o cliente conectado e o processo
do agente. O socket do mesmo usuário, a ancestralidade dos processos e suas horas de início os
ligam à janela capturada. O Kitty também informa o painel exato em foco. A ancestralidade do
processo filho, o PTY compartilhado e o grupo de processos em primeiro plano precisam confirmar o
cliente tmux. Antes da entrega, o Vults os reconfere, verifica a capacidade da janela e consulta o core
sobre o estado **atual** da sessão. Permissões, perguntas e qualquer cartão pendente, incluindo um
cartão explicitamente adiado, mantêm o texto copiado. A indicação aponta para o cartão ou terminal;
responder não libera ditado enfileirado. Mudança de sessão, processo, painel, foco ou autorização
também leva à cópia.

A entrega envia bytes UTF-8 literais pelo caminho de teclas hexadecimais do tmux e aguarda a
confirmação do servidor. O servidor reconfere cliente conectado, processo, janela/painel ativos, modo de cópia e
existência do painel na sua fila de comandos. Nenhuma transcrição é interpolada em shell ou
comando tmux, e nenhum Enter é enviado. Só é aceita uma linha de até 8 KiB sem caracteres de
controle; texto com várias linhas ou controles fica copiado. O modo de cópia também impede a
entrega. **Na sessão** significa que esse transporte aceitou a inserção, não que o agente enviou
ou executou o texto. Uma entrega bem-sucedida não altera a área de transferência; **Copiar
novamente** continua sendo uma ação explícita apenas de cópia.

O Plasma Wayland exige a autorização de teclado existente mesmo neste caminho; recusa ou revogação
nunca desvia para IPC do terminal. O Chat do Vults em foco continua preenchendo o rascunho atual
sem enviar. Os antigos compositores de execuções Tasks em Lab/Quick foram removidos na N0 e não
são recriados.

<a id="coming-in-n3"></a>

## O que chega na N3

Histórico, escolhas de gesto/entrada/processamento, indicações medidas, legendas, código falado,
cancelamento por voz e controle de música estão disponíveis nesta versão. O Chat mantém o limite
de um minuto por gravação; o ditado independente guarda até dez minutos. O processamento local
de Suas palavras está conectado com edição de Dicionário/Trechos, controles de vocabulário e prévia
de frases. As estatísticas de Ditado na Atividade mostram o uso medido, e Configurações > Ditado
inclui Diagnóstico. Estilos, Reuniões e comandos de voz continuam sendo etapas futuras. Nenhuma página provisória executa gravação ou requisição de
modelo simulada.

Limpeza por modelo, resumos de reuniões e voz em tempo real continuam sendo recursos separados,
desligados por padrão. A transcrição comum permanece local. Ditado nunca responde a uma
permissão de agente; veja as [regras de inserção](../adr/0032-dictation-into-other-apps.md) e as
[regras de voz e gravação](../adr/0028-spoken-replies-through-a-realtime-model.md).

<a id="dictation-history"></a>

## Histórico de ditado

Abra **Ditado** no cabeçalho do Nest ou escolha **Abrir Ditado** na paleta. A página continua
disponível com o Zeca desligado. Ela agrupa os 500 ditados guardados mais recentes por dia local e
busca nos textos ouvido/escrito, aplicativo e sessão sem reler o banco. Textos e valores longos
quebram linha dentro da coluna rolável. **Mostrar o que foi ouvido** expande a transcrição
original. Ela pode diferir do texto escrito quando há aplicação de Suas palavras ou conversão
de código falado.

Os números de hoje usam a duração medida do áudio e o total de palavras entregue. Um dia sem
registro mostra **Desconhecido**, em vez de um zero medido. A velocidade é desconhecida abaixo de
um segundo de áudio. O tempo economizado estima digitação a 45 palavras por minuto e desconta
o tempo de ditado, sem ficar negativo. Correções desconhecidas não aparecem. Os números
permanecem depois de excluir as transcrições.

Entregas retidas também guardam tempos locais por etapa, em milissegundos: transcrição (incluindo
carregamento do modelo e eventual nova tentativa com Base), processamento do texto e entrega
confirmada. O total começa ao soltar a tecla, pressionar para encerrar a gravação sem as mãos ou
atingir o limite da captura; inclui a cauda do microfone e a espera pelo worker. A entrega termina
quando a cópia ou o envio nativo retorna, sem medir a renderização do aplicativo de destino.
A API de histórico
fornece p50/p95 por etapa pela ordem de classificação mais próxima entre as 50 entregas retidas
mais recentes, ignorando medições desconhecidas. Entradas antigas e a etapa de limpeza ainda não
utilizada ficam desconhecidas. Cancelamento, falha, resultado vazio e reutilização não acrescentam
amostras. Exclusão, expiração e **Não guardar** removem esses tempos junto das entradas; os totais
de uso por dia/aplicativo continuam independentes. [Atividade > Ditado](activity.md#dictation-statistics)
mostra os números de uso; **Configurações > Ditado > Diagnóstico** mostra os tempos por etapa.

**Copiar** usa a área de transferência com indicação de sucesso/falha. **Usar novamente** pede
ao worker de Copiar novamente que copie a entrada guardada e a lembre como último texto.
Acompanhe o estado real de entrega na ilha e volte ao destino para colar. A ação não digita na
busca do Nest, não envia Enter, não reutiliza uma autorização antiga de janela e não conta outro
ditado. Gravação/transcrição ativa rejeita a ação; entradas excluídas ou expiradas não podem voltar.

**Excluir…** e **Apagar histórico de ditado…** exigem confirmação própria. Apagar remove as
transcrições em cache mesmo se a próxima leitura falhar. Limpar todo o histórico ou mudar a
retenção invalida gravações anteriores ainda em andamento no histórico; a duração existente do
último texto de Copiar novamente na memória continua separada. Áudio e clipboard anterior nunca
são escritos no histórico.

**Configurações do ditado** leva a **Settings > Dictation**. Os grupos Atalhos de teclado, Voz,
Colagem, Terminais e sessões, Suas palavras e Privacidade reúnem os controles existentes; Geral e Chat e voz têm links para cá.
Mudanças de modelo e idioma continuam compartilhadas em **Chat e voz**. **Privacidade > Guardar
histórico de ditado** oferece a escolha de prazo. Escolha **Não guardar**, **7 dias**, **30 dias**
(o padrão), **1 ano** ou **Sempre**. Reduzir o prazo apaga os textos antigos imediatamente;
abrir o histórico e gravar também aplica o prazo. **Não guardar** não escreve transcrição nem
sessão no histórico, mas os totais medidos por dia/aplicativo continuam atualizados. Dicionário/
Trechos são páginas funcionais. **Estatísticas** abre Atividade > Ditado; Estilos, reuniões e
comandos de voz continuam como etapas futuras.
**Privacidade > Apagar histórico de ditado…** usa a mesma exclusão confirmada da página de trabalho,
com falha e nova tentativa explícitas. Modo de gravação, microfone, remoção de repetições, nova
tentativa com Base, código falado, cancelamento por voz, música e vocabulário são controles funcionais.
Opções adicionais de entrega continuam como etapas futuras. Gravações ativas
mantêm as escolhas capturadas no início.

<a id="diagnostics"></a>

## Diagnóstico

Abra **Configurações > Ditado > Diagnóstico** no fim da página. **Atualizar diagnóstico** lê o estado
atual do áudio, desktop e modelos locais sem iniciar captura nem pedir uma permissão ao desktop.
A entrada selecionada usa a alternativa normal do ditado quando um dispositivo está ausente; o nome e a
**taxa nativa de entrada** indicam o formato do dispositivo antes da conversão para 16 kHz. Uma entrada
indisponível fica desconhecida. GlobalShortcuts pronto significa que o atalho de fala está registrado.
RemoteDesktop informa a permissão de teclado existente; KWin informa se essa permissão tem seu script
de foco verificado ativo. Nenhum dos dois é necessário para inserção no X11. GPU disponível não comprova
velocidade de transcrição, e VAD pronto apenas indica que seu modelo local está instalado.

**Testar microfone** abre a entrada selecionada por seis segundos. Fique em silêncio pelos primeiros
dois segundos e fale pelos quatro seguintes. Nove bandas de fala mostram o sinal ao vivo. O resultado
estima o ruído de fundo do silêncio inicial em **dBFS**, um nível digital, não o volume acústico do
ambiente, e indica sinal fraco/ausente, ruído de fundo excessivo ou saturação. O silêncio digital tem
piso de -120 dBFS. **Parar teste**, sair da página/janela, bloquear a tela ou começar uma gravação no
Chat/ditado encerra o teste. Perder contato com a página de Configurações também encerra a captura.
Um novo teste espera a captura anterior fechar. O áudio é descartado da memória; o teste não transcreve,
copia, insere, salva histórico nem busca modelos.

Os contadores somam sequências repetidas removidas nas decodificações finais do ditado, novas tentativas
com Base instalado e resultados **Copiado** confirmados desde o início do Vults. Prévia e Chat não somam;
**Copiar novamente** pode somar um resultado Copiado. Esses contadores ficam somente na memória e zeram
ao sair. A tabela de etapas mostra a quantidade de amostras retidas de entregas e p50/p95 em milissegundos;
uma etapa sem medição fica **Desconhecida**. Falha ao atualizar preserva as últimas leituras com aviso.
Falha na leitura dos tempos é diferente de histórico vazio, em que cada etapa tem zero amostras e
durações desconhecidas.

![Diagnóstico de ditado no app nativo](../../assets/dictation-diagnostics.png)

Exemplo de validação nativa isolada: entrada e atalhos indisponíveis aparecem explicitamente.

O resumo de tempos combina as entregas retidas mais recentes, mesmo quando o modelo, idioma ou destino
mudou. **p50** é a duração observada central; **p95** é a duração observada na posição do percentil 95,
sem interpolação. O total inclui a cauda e a fila do worker; as outras etapas não precisam somar esse
valor. Um resultado Copiado mede a entrega na área de transferência, não a inserção em outro app.
GPU **Pronto** e algumas amostras rápidas não comprovam uma garantia de latência para determinado
modelo ou destino.

<a id="check-statistics-and-diagnostics-locally"></a>

## Confira estatísticas e Diagnóstico localmente

1. Em **Configurações > Ditado > Diagnóstico**, use **Atualizar diagnóstico**. Confirme que isso não
   grava nem pede uma permissão ao desktop. Uma sondagem indisponível deve continuar indisponível/
   desconhecida; X11 não precisa de RemoteDesktop nem do script de foco do KWin.
2. Escolha **Testar microfone**, fique em silêncio por dois segundos e fale em seguida. Confira as
   bandas, a estimativa de ruído e o resultado. Comece outro teste e use **Parar teste**; depois tente
   sair da página durante um teste. A captura deve terminar sem acrescentar transcrição, alterar a
   área de transferência ou somar amostras de uso/tempos.
3. Dite uma frase curta no app que costuma usar. Confira o resultado real **Copiado**, **Colado** ou
   **Na sessão** e o texto final. Em **Atividade > Ditado**, confira as novas palavras, a barra diária
   e a atribuição do aplicativo. Um destino que não pôde ser identificado fica em Desconhecido.
4. Volte ao Diagnóstico e atualize. Um ditado retido e entregue deve acrescentar uma amostra de tempo;
   a limpeza por modelo não utilizada fica Desconhecida. **Copiar novamente** deve manter o uso e as
   amostras de tempo; uma cópia bem-sucedida pode incrementar o contador separado de resultados Copiado.
   Cancelar um novo ditado não acrescenta amostras.
5. Se guardou uma transcrição descartável de teste, exclua apenas essa entrada do Histórico. Sua
   amostra de tempo desaparece após atualizar, enquanto os totais de uso permanecem. Com **Não guardar**,
   novos ditados entregues acrescentam uso, mas não amostras retidas de tempo. Escolha o prazo de forma
   consciente; reduzi-lo também remove textos antigos retidos imediatamente.
6. Confira as páginas em Black e Light, no tamanho habitual e numa janela estreita e baixa. Alcance
   Parar, Atualizar, a escolha de período e a tabela de etapas pelo teclado. Alterne Room/Full e reabra
   o Nest; o período das estatísticas deve continuar selecionado. Reinicie o Vults: o uso permanece,
   os tempos retidos acompanham seus textos e os contadores na memória zeram.

Ao relatar um problema, inclua a versão do app, desktop/sessão e escala, microfone selecionado,
modelo/idioma, resultado real de entrega, quantidade de amostras dos tempos relevantes e os passos
que falharam. Não é preciso compartilhar o texto ditado. Essas verificações cobrem as telas atuais
da V4; comandos de voz, Estilos, Reuniões e voz em tempo real continuam como etapas separadas.
