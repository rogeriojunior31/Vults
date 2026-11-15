# Reuniões
<!-- source: a19b23b5c0e1 -->

Grave uma reunião, tanto o seu microfone quanto o que o computador toca, e receba uma transcrição
local. Até quatro horas, no Linux, com o Zeca ligado ou desligado. Gravar e transcrever não fazem
nenhuma requisição de rede; o resumo é um passo à parte e opcional.

<a id="record"></a>

## Gravar

1. Instale um modelo de fala em **Configurações > Ditado > Voz**.
2. Abra **Ditado > Reuniões > Nova gravação** e escolha **Conferir fontes de áudio**. Confira o
   microfone, o áudio do sistema, o modelo e o idioma.
3. **Avise os participantes de que você está gravando.** Marque a confirmação e escolha **Gravar e
   transcrever** (em até um minuto depois da conferência).
4. **Parar gravação** quando terminar. A transcrição roda em seguida, com o progresso na tela.

Mantenha Reuniões aberta enquanto grava e transcreve: sair, esconder a janela ou bloquear a tela
cancela a gravação. **Cancelar gravação** a descarta.

Durante a gravação, o áudio vai para arquivos temporários na pasta de dados do Vults (cerca de
220 MiB por hora), privados ao seu usuário. Eles são apagados quando a transcrição termina, falha ou
é cancelada, e na próxima abertura depois de uma queda. Nenhum áudio é guardado.

<a id="read-the-transcript"></a>

## Ler a transcrição

**Transcrição** mostra **Microfone** e **Áudio do sistema** separados, cada um com seus próprios
tempos. O texto só existe nesta página até você escolher **Salvar reunião**: nada é copiado, colado
ou adicionado ao histórico do ditado.

*Rótulos de quem fala* (**Voz 1**, **Voz 2**) são experimentais: precisam dos modelos opcionais em
**Configurações de reuniões** e de um build de desenvolvimento, e podem errar.

<a id="summary-and-to-dos"></a>

## Resumo e tarefas

**Configurações > Ditado > Configurações de reuniões > Ativar resumos de reuniões** (desligado por
padrão) deixa um modelo de **Configurações > Chat e voz** ler o texto da transcrição, nunca o áudio.
Depois de transcrever, abra **Resumo** e escolha **Gerar resumo**: título, resumo, pontos principais e
tarefas com responsáveis e prazos quando foram ditos. Compare com a transcrição; um modelo pode
entender errado um compromisso. São no máximo 20 resumos por dia; o provedor pode cobrar.

<a id="saved-meetings-and-export"></a>

## Reuniões salvas e exportação

Dê um título à reunião e **Salvar reunião**: a transcrição e as notas prontas são salvas como texto
neste computador. **Reuniões salvas** busca pelo título. Numa reunião salva você pode dar nome a
cada voz, exportar para **Markdown** (as duas fontes e as notas) ou para legendas **SRT/VTT** (uma
fonte) e **Excluir…**.

Os arquivos exportados são seus; excluir a reunião não os apaga. **Limpar histórico** na Atividade
também exclui todas as reuniões salvas.
