# Configurações e arquivos
<!-- source: 5983e889fa7b -->

**Ditado > Diagnóstico** oferece um teste explícito de microfone de seis segundos,
somente na memória, estado atual do áudio/desktop/modelos, contadores desde o início do app e tempos
retidos por etapa. Não acrescenta configuração persistente nem pede permissão ao desktop. Veja
[Diagnóstico](../guide/dictation.md#diagnostics).

<a id="where-things-live-linux"></a>

## Onde cada coisa fica (Linux)

| Caminho | O quê |
|---|---|
| `~/.config/vults/settings.json` | As configurações do app. Um campo que ele não consegue ler (tipo errado) volta ao padrão e o resto é mantido; antes, o arquivo como estava é copiado para `settings.json.bad-<time>`. Um arquivo de uma release mais nova é lido até onde esta entende, e antes é copiado para `settings.json.v<version>-<time>` (veja `version` abaixo) |
| `~/.local/share/vults/bin/vults-hook` | O relay de hook que seus agentes executam |
| `~/.local/share/vults/bin/statusline-previous.json` | O seu próprio `statusLine` do Claude Code, salvo quando os hooks foram instalados; o hook o executa, e remover os hooks o coloca de volta |
| `~/.local/share/vults/inbox/` | Cópias dos arquivos soltos na ilha, apagadas depois de uma semana |
| `~/.local/share/vults/chat/` | A pasta vazia que os chats usam quando não há sessão em primeiro plano |
| `~/.local/share/vults/connectors/` | O que cada conector viu por último |
| `~/.local/share/vults/vults.sqlite` | O banco local: o histórico dos turnos dos agentes (só contagens), para Settings → Activity, o log de auditoria das respostas aos cards (90 dias), as políticas que você aceitou (guardadas até você revogá-las, e depois), os telefones que você pareou (as chaves públicas deles e o último contador com que cada um assinou; os revogados ficam, para que um telefone revogado nunca volte), os traces do Zeca (30 dias), as conversas dele (as 200 mais recentes), o diário e os resumos dos dias (90 dias), os vetores do recall, as propostas de memória (as decididas ficam 90 dias), as rodadas de sono e o que os agentes custaram por dia e por roost (veja [Segurança](../safety.md)). O **Clear history** esvazia quase tudo ([o que ele remove e o que fica](../guide/activity.md#the-history)). O `history.jsonl` e o `days.json` das versões anteriores entram nele na primeira abertura |
| `~/.local/share/vults/memory/` | A memória do Zeca: um repositório git de pequenos arquivos Markdown, cada mudança um commit (modo da pasta conforme o seu umask). O **Clear history** não mexe nela; apague a pasta para apagar os fatos dele ([A memória do Zeca](../guide/memory.md)) |
| `~/.local/share/vults/voice/` | Os modelos de voz que você baixou e o detector de fala |
| `~/.local/share/vults/meeting-audio/` | Áudio temporário privado das verificações explícitas de gravação/transcrição de um minuto; mantido durante a transcrição local, removido ao concluir/cancelar e limpo na inicialização após interrupções. Não é um arquivo de gravações |
| `~/.local/share/vults/backups/` | Backups datados dos arquivos que o Vults edita dentro de um projeto, como um `CLAUDE.md` que ele fez ler o `AGENTS.md` (nunca guardados no repositório) |
| `~/.local/share/vults/zeca-mcp.json` | A configuração de MCP que o Claude Code do chat recebe com `--mcp-config`: este app rodando como o servidor do Zeca (`vults --zeca-mcp`); modo `0600` |
| `~/.local/share/vults/zeca-flock.json` | As sessões da ilha (redigidas) para o `flock_list` desse servidor, reescrito quando mudam e apagado quando o app fecha; modo `0600` |
| `$XDG_RUNTIME_DIR/vults.sock` | O socket com que o hook conversa (modo `0600`) |
| `~/.config/autostart/` | A entrada que **Start with the desktop** adiciona |
| Chaveiro do sistema, serviço `io.github.rogeriojunior31.vults`, conta `<provider>-api-key` (`anthropic-api-key`, `openai-api-key`, …) | As chaves de API do chat, uma por provedor para o qual você deu uma (nunca em arquivo) |
| Chaveiro do sistema, serviço `io.github.rogeriojunior31.vults`, conta `dictation-keyboard-restore-token` | O token de restauração de teclado RemoteDesktop do ditado, quando há persistência; consumido uma vez, renovado após uma autorização e removido ao desligar a inserção. Nunca fica em arquivo nem é exposto a uma webview |
| `~/.local/state/vults/logs/` | O log: um arquivo por dia, os últimos cinco são mantidos. Ele registra o que aconteceu (nomes de eventos, decisões, erros), nunca comandos, caminhos ou texto do chat |

O dicionário, as regras de substituição e os trechos mantidos pelo usuário para o ditado usam
`vults.sqlite` (esquema 22). São preferências, separadas das transcrições gravadas: o prazo,
a exclusão de transcrições e **Clear history** os preservam. A exclusão individual remove a
entrada escolhida. Cada gravação independente captura essas entradas para o processamento local
do texto final. O vocabulário opcional do modelo usa apenas palavras do dicionário, conforme
as combinações de modelo/idioma medidas abaixo. Edite entradas em **Ditado > Dicionário / Trechos** e confira regras salvas com **Testar uma frase**. Veja o [processamento das suas
palavras](../guide/dictation.md#your-words-processing) para ordem, conteúdo literal dos trechos
e limites de tamanho.

A migração 25 acrescenta um registro de admissão da limpeza sem conteúdo (id da tentativa, recurso e
horário). Clear history o preserva para que apagar transcrições/rastros não renove a cota móvel de
requisições. O vínculo à identidade nas configurações impede que um banco ausente ou recriado renove
o consentimento ou a cota. Admissões vencidas são removidas na próxima tentativa admitida; horários futuros continuam
contando após atrasar o relógio.

A migração 26 acrescenta preferências por aplicativo em `dictation_styles`, preservadas pela retenção
e por Clear history, e `output_language` opcional no histórico. Apenas limpeza bem-sucedida define
o idioma de saída; falha mantém o estilo local e o idioma falado sem alteração.

`$XDG_CONFIG_HOME` e `$XDG_DATA_HOME` substituem `~/.config` e `~/.local/share` quando estão definidas.
**Settings → About** mostra a versão do app e os caminhos reais.

As teclas globais de ditado aparecem em **Dictation > Keyboard shortcuts**.
**Ctrl+Alt+V** sempre inicia o ditado independente; nunca abre o Chat. O botão de microfone do
Chat preenche seu rascunho separadamente.
**Comandos de voz > Tecla de comandos** mostra a combinação real do desktop (sugestão **Ctrl+Alt+Z**); segure e solte
para controlar o Vults sem digitar em outro aplicativo. Usa o modelo local e o microfone compartilhados,
com gestos de apenas segurar e um caminho separado; transformações de texto e inserção não se aplicam.
**Alterar no KDE…** abre o editor de atalhos instalado do KDE; falhas ao iniciar permitem tentar
novamente. Em outros desktops, altere os atalhos globais nas configurações do próprio desktop.
Abrir esse editor não grava preferências do Vults nem configurações de agentes.

<a id="dictation-settings"></a>

## Configurações do ditado

**Configurações de reuniões > Modelos de falantes para reuniões (experimental)** reúne a instalação opcional de
modelos/runtime com revisão. Mostra progresso e cancelamento e não ativa a análise. Os arquivos
ficam em `~/.local/share/vults/voice/meetings/`; nenhuma preferência persistente de ativação é
adicionada. Veja [Artefatos dos modelos de reuniões](meeting-models.md) para fontes, licenças e
limites de requisições.

A aba **Ditado** reúne suas páginas de configuração ao lado de Histórico, Comandos de voz,
Dicionário, Trechos e Reuniões. **Voz** reúne o modelo local e idioma falado compartilhados; **Chat e voz**
oferece um link. A configuração usa a webview privilegiada de Configurações, sem mostrar transcrições
nem mudar a autoridade dos comandos. A lista global de Configurações deixa de duplicar essas páginas. Durante a execução do app, a aba
Ditado restaura sua última seção de configuração; Comandos de voz abre diretamente Configurações
dos comandos.
Mudanças de microfone, gesto, processamento, vocabulário e música valem para a próxima gravação;
um turno ativo mantém essas escolhas capturadas. Preferências de legenda/som continuam ativas,
o prazo entra em vigor imediatamente e desligar a inserção revoga as capacidades pendentes.

| Grupo | Controles disponíveis |
| --- | --- |
| Atalhos de teclado | Atalhos atuais do desktop, editor de atalhos do KDE, segurar/toque duplo, Espaço/Esc no X11 e cancelamento falado |
| Configurações dos comandos | Ligado por padrão, combinação real, nomes/aliases, aviso local ou envio explícito ao Chat, frases compatíveis |
| Voz | Editor de modelo/idioma compartilhados, microfone e atualização, remoção de repetições, nova tentativa com Base, música, aviso inicial e legendas |
| Vocabulário | Contexto opcional do dicionário e vocabulário do projeto; ambos desligados por padrão, limitados à combinação qualificada Turbo/inglês fixo |
| Colagem | Pontuação de código falado, inserção opcional em editores/tmux, permissão real de teclado e nova tentativa explícita |
| Estilos | Consentimento separado da limpeza, provedor/dados/cota, estilo/idioma de saída, inglês nas sessões e escolhas por aplicativo |
| Privacidade | Prazo do Histórico e exclusão confirmada |
| Diagnóstico | Teste explícito de microfone, estado atual do sistema, contadores da execução e tempos retidos por etapa |

Edite palavras do dicionário, regras de substituição e trechos literais nas páginas de trabalho
de **Ditado** no Nest; **Testar uma frase** mostra as regras locais salvas sem usar modelo, área
de transferência ou histórico. As regras locais de texto funcionam com qualquer idioma de
transcrição; só o contexto opcional de vocabulário do Whisper tem a restrição modelo/idioma.
O Diagnóstico lê o estado atual; só a ação explícita **Testar microfone** grava, e esse teste
para se você mudar o microfone. Ele não acrescenta preferências salvas. Modos adicionais de entrega chegam em fases futuras. Veja o [guia de ditado](../guide/dictation.md)
para destinos compatíveis, comportamento da área de transferência e limites de privacidade.

## settings.json

```json
{
  "version": 39,
  "connectors": { "github": true },
  "sounds": true,
  "volume": 50,
  "fold_after": 15,
  "api_provider": "openrouter",
  "api_models": { "openrouter": "anthropic/claude-opus-5.5" }
}
```

| Chave | Padrão | Significado |
|---|---|---|
| `version` | `41` | Versão do esquema, para que releases futuras possam migrar o arquivo. Se for mais nova que a do app (você voltou para uma release mais antiga), o app (a partir da 0.1.1) usa as chaves que conhece, guarda o arquivo como estava em `settings.json.v<version>-<time>` e avisa no log; a próxima mudança que você fizer salva só as chaves que ele conhece, na versão dele. Até lá, cada início guarda mais uma cópia. Se nenhuma cópia puder ser guardada, as mudanças não são salvas. Para voltar às configurações da release mais nova, restaure essa cópia. Um arquivo mais antigo é lido com os padrões para as chaves que faltam nele, e é salvo na versão do app |
| `connectors` | `{}` | Id do conector → ligado |
| `sounds` | `true` | Sons 8-bit |
| `volume` | `50` | O volume dos sons, em porcentagem (0 a 100); `50` é o volume em que a 0.1.0 os tocava |
| `voice_model` | ausente | O modelo de voz compartilhado pelo Chat e pelo ditado (`base`, `small`, `turbo`), baixado em `~/.local/share/vults/voice/` (junto com o detector de fala, `ggml-silero-v6.2.0.bin`); ausente deixa a voz desligada. Cada gravação do Chat guarda até um minuto de áudio; o ditado independente guarda até dez minutos |
| `voice_language` | ausente | O idioma que o usuário fala para a voz: um código (`pt`, `en`), `auto` para detectar a cada vez, ausente para seguir o idioma do sistema |
| `dictation_insert` | `false` | Desligado até ativar em **Dictation**. Cola em editores simples compatíveis no X11 nativo ou Plasma Wayland, reconferindo o foco e usando uma reserva condicional de texto simples da área de transferência (até 64 KiB); sem Enter. O Wayland solicita apenas teclado pelo portal e guarda um token de restauração somente no chaveiro do sistema; recusa/revogação mantém a cópia até tentar novamente em Settings. Painéis tmux exatos do bando no XTerm/UXTerm ou Kitty podem receber uma linha sem controles de até 8 KiB após verificar a sessão e seus cartões atuais, sem alterar a área de transferência; outros destinos de terminal ficam apenas na cópia. Imagens e formatos ricos não são restaurados; a exclusão do histórico depende do desktop. Veja [Ditado](../guide/dictation.md#opt-in-x11-insertion) (a partir da versão 24) |
| `dictation_caption` | `true` | Mostra linhas cortadas do ditado ou a transcrição final confirmada em uma legenda de duas linhas que deixa cliques passarem. Cards e espaço limitado têm prioridade; a legenda termina após dois segundos, mesmo trocando de superfície. Configurada em **Dictation** (a partir da versão 25) |
| `dictation_cleanup` | `false` | Consentimento separado para uma requisição de limpeza de texto sem ferramentas pelo provedor de API selecionado em Chat & voice após o ditado local. O prazo de quatro segundos (oito para tradução, doze para Prompt) preserva o texto local; desligar ou mudar provedor/modelo/chave cancela a limpeza. Cota do portão: 2.000 tentativas em 24 horas móveis, incluindo chamadas locais, falhas e interrupções. Capturado ao pressionar a tecla; sem caminho por CLI. Configurado em **Ditado > Estilos** (a partir da versão 37) |
| `dictation_cleanup_budget` | ausente | Identidade interna do registro local salva na primeira ativação explícita da limpeza; preservada enquanto desligada e após Clear history. Estado de orçamento ausente, divergente ou ilegível impede requisições de limpeza; nunca enviada ao provedor (a partir da versão 37) |
| `meeting_summary` | `false` | Consentimento separado para o backend de resumos de reuniões por meio do provedor/modelo escolhido do chat por API; não herdado da limpeza do ditado. Configurado em **Ditado > Configurações de reuniões**; as abas Resumo/Pendências ficam no Ninho. Um pedido explícito do Ninho exige transcrição concluída e reserva durável de uma das 20 tentativas por 24 horas móveis (chamadas locais e falhas incluídas), com prazo total de 120 segundos. Desligar ou mudar provedor/modelo/chave cancela o trabalho; falhas preservam o texto local (a partir da versão 39) |
| `meeting_summary_budget` | ausente | Identidade do registro local vinculada na primeira ativação explícita dos resumos; preservada ao desligar e após Clear history. Uma identidade ausente/divergente ou um registro ilegível impede chamadas; substituir o banco nunca renova silenciosamente o consentimento (a partir da versão 39) |
| `voice_realtime` | `{on:false, ledger:null, limit_microusd:1000000, model:"gpt-realtime-2.1-mini"}` | Consentimento independente de voz só no backend (versão 41). Apenas a webview de Configurações salva a escolha; ainda sem controle visual/captura. Zero desativa voz remota. A identidade do registro permanece ao desligar/apagar histórico e precisa coincidir antes da admissão monetária; responsáveis exclusivos no Linux conciliam reservas abandonadas pelo custo desconhecido integral; responsáveis ativos continuam recusando a entrada. Recuperação local devolve rascunho/áudio original sem envio automático. Veja [Contabilidade realtime](realtime-accounting.md) |
| `dictation_style` | `"natural"` | Padrão global para aplicativos identificados sem categoria/escolha explícita: `"verbatim"`, `"natural"`, `"polished"`, `"prompt"`. Escolhas por aplicativo ficam em `dictation_styles`; sessões mantêm Prompt. Foco ausente mantém texto local; estilos nunca ativam a limpeza (a partir da versão 38; controles em **Ditado > Estilos**) |
| `dictation_write_in` | ausente | Idioma de saída opcional: `"en"`, `"pt-BR"`, `"es"`, `"zh"`; ausente mantém o idioma falado. Como falado não faz requisição nem traduz. Capturado ao pressionar a tecla, com idioma de saída aplicado guardado separado do idioma ouvido (a partir da versão 38) |
| `dictation_sessions_english` | `false` | Usa inglês na saída apenas em sessões reconhecidas do bando, com Prompt. Outros aplicativos mantêm Escrever em; contexto ambíguo de sessão nunca ativa inglês. Requer o consentimento separado da limpeza (a partir da versão 38) |
| `dictation_commands` | `true` | Ativa a tecla separada de comandos, apenas segurando. Desligar cancela captura/envios e contexto de comandos, preservando ditado comum e Copiar novamente (a partir da versão 36) |
| `dictation_command_media` | `false` | Consentimento independente para comandos locais de reprodução. Exige comandos de voz e controles de mídia ligados; atua só no reprodutor selecionado, sem alterar notificações de projetos. Desativar cancela comandos pendentes. Configure em **Ditado > Configurações dos comandos** |
| `dictation_command_chat` | `false` | Envio explícito de fala desconhecida ao Chat atual livre, com Zeca ligado e sem texto/arquivos/referências por enviar. Usa seu modelo, portão, orçamento, rastros e regras de aprovação; recusas/ambiguidades/comandos desatualizados ficam locais. Alterações somente em Settings invalidam tickets antigos (a partir da versão 36) |
| `dictation_sound` | `true` | Aviso curto ao iniciar uma gravação, respeitando a opção de sons, o volume e o Não Perturbe. Trocas de superfície e configurações não repetem o aviso. Configurado em **Dictation** (a partir da versão 26) |
| `dictation_music` | `"lower"` | **Ditado > Voz > Música enquanto você fala**: `"lower"` limita players MPRIS que já tocam a 25%, sem elevar os mais baixos; `"pause"` os pausa; `"leave"` não abre uma conexão de controle de mídia. Restaura o volume/reprodução de cada player não alterado após captura, cancelamento ou falha. Preserva mudanças do usuário e players que fecharam/reiniciaram. Somente controles locais, sem metadados de faixas/rede, separado de Tocando agora; o Chat e futuras reuniões não usam isso. Capturado na próxima gravação (a partir da versão 33) |
| `dictation_cancel_by_voice` | `true` | **Dictation > Keyboard shortcuts > Cancelar por voz**: descarta a transcrição final independente composta de, ou terminada com, uma frase inteira de cancelamento em inglês, português ou espanhol. Executado após encerrar a gravação, antes da conversão/entrega, no X11 e Wayland. Não copia, insere ou envia nada nem salva histórico ou totais; mantém o último ditado bem-sucedido. A reutilização e o Chat não passam por ele. Capturado na próxima gravação (a partir da versão 32) |
| `dictation_spoken_code` | `true` | **Dictation > Terminais e sessões > Código falado**: converte pontuação falada em inglês/português/espanhol entre partes de identificadores apenas para destinos reconhecidos como terminal/sessão. Preserva maiúsculas, minúsculas, pontuação existente e caracteres de controle; nunca acrescenta Enter. Aplicativos de prosa e destinos desconhecidos mantêm seu texto. Capturado para a próxima gravação; o Histórico mantém o texto ouvido e escrito, sem reformatar na reutilização (a partir da versão 31) |
| `dictation_cut_repeats` | `true` | **Dictation > Voice > Remover repetições em sequência**: reduz sequências longas de repetição na prévia e no texto final do ditado independente. Desligado mantém as repetições; marcações de som conhecidas e frases isoladas associadas a silêncio continuam sendo removidas. O Chat mantém seu filtro existente. Capturado no início de cada gravação (a partir da versão 30) |
| `dictation_retry_base` | `true` | **Dictation > Voice > Tentar com Base quando o ditado vier vazio**: permite uma única nova tentativa visível para texto vazio quando houve fala detectada e um modelo Base diferente está instalado. Desligado pula essa verificação extra de fala e a nova tentativa; erros não geram nova tentativa. Nunca baixa um modelo. Capturado no início de cada gravação independente (a partir da versão 30) |
| `dictation_vocabulary` | `false` | Contexto local opcional do dicionário para o ditado independente, limitado a 224 tokens incluindo o contexto de programação e entradas inteiras. Qualificado atualmente só para Turbo com inglês fixo (escolhido ou resolvido pelo sistema), em testes sintéticos de WER/silêncio; Base, Small, português, outros idiomas e detecção automática mantêm seus prompts anteriores. Prévia/final compartilham a visão capturada na gravação; a nova tentativa com Base verifica seu modelo real e fica sem contexto de dicionário. Nunca inclui conteúdo de substituições/trechos nem altera o Chat. Configurado em **Ditado > Vocabulário > Ajudar o modelo com estas palavras** (a partir da versão 34) |
| `dictation_project_vocabulary` | `false` | Opção independente para usar nomes locais do projeto no ditado, com a mesma restrição Turbo/inglês fixo e limite compartilhado de 224 tokens. Lê árvores de trabalho Git conhecidas ao iniciar a sessão/atualizar o repositório: nomes de pacotes e dependências Cargo/npm, membros próximos do workspace, módulos/arquivos e branch; sem conteúdo do código nem comandos. Prefere a pasta da sessão em foco; caso contrário exige um projeto conhecido sem ambiguidade no título de um terminal/editor. Palavras do dicionário pessoal têm prioridade. Cache em memória, worktrees separadas, contexto da primeira decodificação fixo para prévia/final; contexto ausente/tardio/ambíguo não acrescenta nada. Configurado em **Ditado > Vocabulário > Vocabulário do projeto** (a partir da versão 35) |
| `dictation_microphone` | ausente | O nome da entrada escolhida em **Dictation > Voice** para o ditado independente. Ausente segue o padrão do sistema. Se a entrada salva não for encontrada, usa o padrão ou outra entrada sem mudar a preferência; uma entrada listada que não puder ser aberta mostra um erro. O Chat mantém seu próprio padrão. Listar/atualizar não grava; mudanças valem a partir da próxima captura (a partir da versão 29) |
| `dictation_double_tap` | `true` | **Dictation > Modo de gravação**: permite toque duplo para gravar com as mãos livres. Desligado, toques curtos são descartados imediatamente; segurar por pelo menos 300 ms continua funcionando. Independente da trava por Espaço. Mudanças valem a partir da próxima gravação (a partir da versão 28) |
| `dictation_space_lock` | `true` | Espaço mantém uma gravação segurada com as mãos livres no X11. Desligado, Espaço segue o cancelamento por digitação comum e não reserva o atalho de Abrir a ilha. Indisponível no Wayland; exige um listener de teclas X11 reconhecido. Mudanças valem a partir da próxima gravação (a partir da versão 28) |
| `dictation_escape_cancel` | `true` | Esc cancela a captura, sua cauda ou a transcrição no X11. Desligado, Esc deixa o ditado continuar. O atalho separado Cancelar voz sempre funciona. Indisponível no Wayland. Mudanças valem a partir da próxima gravação (a partir da versão 28) |
| `dictation_keep` | `"30"` | Prazo das transcrições de ditado: `"0"` (não guardar), `"7"`, `"30"`, `"365"` ou `"all"`. Configurado em **Dictation**. Reduzir o prazo apaga textos antigos imediatamente; abrir o histórico e gravar também aplica o prazo. Totais por dia/aplicativo permanecem após apagar ou escolher Não guardar. Áudio e clipboard anterior nunca são gravados (a partir da versão 27) |
| `now_playing` | `false` | Mostra na ilha a música que seus players estão tocando (MPRIS no Linux), com tocar, pausar e pular |
| `codex_usage` | `false` | Lê o uso do plano do Codex para a ilha a cada 5 minutos (e meio minuto depois que uma janela reinicia), pelo codex em que você fez login (cada leitura chega à OpenAI, pelo portão de rede do app). Desligado numa instalação nova; um arquivo de antes da versão 16 o recebe ligado quando há um `codex` no `PATH`, para que uma atualização mantenha os medidores que mostrava (a partir da versão 16) |
| `web` | `false` | As ferramentas de web do Zeca no chat por API: uma busca e a leitura de uma página que o modelo pode pedir, cada uma uma requisição pelo portão de rede. Desligado, elas nunca são oferecidas |
| `web_engine` | `"brave"` | Para onde as buscas vão: `brave`, `tavily` (cada um com a chave no chaveiro) ou `searxng` |
| `searxng_url` | ausente | O endereço do seu próprio SearXNG (`https://…`), quando ele é o serviço |
| `phone` | `false` | Responder aos cards por um telefone pareado: cada card vai para ele e as respostas dele são lidas de volta, pelo portão de rede. Definido em **Settings → Phone** (a partir da versão 17) |
| `phone_transport` | `"relay"` | Como o telefone chega a este computador: `relay` (por `phone_relay`) ou `direct` (este computador escuta em `phone_listen`) |
| `phone_relay` | ausente | O endereço do relay (`https://…`; `http://` só numa rede em que você confia) |
| `phone_listen` | ausente | Em `direct`, o endereço deste computador na sua rede local ou tailnet e uma porta (`100.64.0.2:8787`) |
| `chat_picks` | ausente | O modelo e o esforço do chat por pasta (`""` para a pasta vazia dele), depois por motor: `{ "/home/me/site": { "claude": { "model": "sonnet", "effort": "high" } } }`. `model` é um nome que a CLI aceita (`opus`, `gpt-6.1-sol`), `effort` um de `low`, `medium`, `high`, `xhigh`, `max`; qualquer um ausente é o padrão da CLI. Definido pelo seletor do chat; uma conversa nova naquela pasta começa com ele |
| `native_chat` | `true` | O `claude` e o `codex` do chat leem a sua própria configuração: seu `CLAUDE.md`, skills, servidores MCP, hooks e regras de permissão; as configurações de um projeto só numa pasta em que você já confiou no Claude Code. Os cards ainda perguntam o que suas regras não liberam. Desligado, o Claude não lê nada disso e o Codex deixa de fora os seus hooks (ele sempre lê o próprio `config.toml`). Vale a partir da próxima conversa |
| `chat_access` | `"ask"` | Quem responde os cards do Zeca ([ADR 0004](../adr/0004-a-human-answers-permissions.md)): `ask` (um card toda vez), `policies` (as políticas que você aceitou respondem o que cobrem) ou `full` (tudo roda sem card, menos comandos destrutivos e o que uma política nega ou manda perguntar; os cards voltam depois que uma conversa lê algo de fora). Muda em **Settings → Chat & voice**; um valor que ele não consegue ler pergunta. Vale na hora |
| `memory` | `false` | A memória e o diário do Zeca; desligada, nada dela chega a um modelo ([A memória do Zeca](../guide/memory.md)) (a partir da versão 18) |
| `journal_summary` | `false` | Com `memory` ligada, **A summary of each day** do diário, pelo menor modelo do Claude Code ([A memória do Zeca](../guide/memory.md)) (a partir da versão 18) |
| `local_embeddings` | `false` | Com `memory` ligada, **Recall by meaning**, por um Ollama em `127.0.0.1` ([A memória do Zeca](../guide/memory.md)) (a partir da versão 18) |
| `memory_suggest` | `false` | Com `memory` ligada, **Suggest what to remember** a partir da conversa que você deixou ([A memória do Zeca](../guide/memory.md)) (a partir da versão 18) |
| `sleep` | `false` | Com `memory` ligada, **Let Zeca sleep on it**: o sono noturno dele também pede a um modelo junções e um resumo da volta ([A memória do Zeca](../guide/memory.md)) (a partir da versão 18) |
| `journal_summary_since` | ausente | O dia em que `journal_summary` foi ligado (`YYYY-MM-DD`): o diário guardado antes nunca vai para o modelo (a partir da versão 18) |
| `fold_after` | `15` | Segundos que a ilha aberta espera, depois que o ponteiro sai, antes de se recolher (5 a 120) |
| `open_on_hover` | `false` | Deixar o ponteiro parado sobre a pílula abre a ilha, sem clique; aberta assim, ela se recolhe assim que o ponteiro sai, a não ser que você tenha clicado nela. Não vale no painel. Definido em **Settings → General** (a partir da versão 11) |
| `monitor` | ausente | A tela em que a ilha fica, como fabricante e modelo (`"Samsung Electric Company LS27AG32x"`); ausente deixa o desktop escolher. Duas telas idênticas têm o mesmo nome, e a primeira vence |
| `nest_size` | ausente | A janela do Nest como você a deixou por último, `[largura, altura]` em pixels lógicos; ausente, ela abre no tamanho padrão (a partir da versão 21) |
| `rules` | `[]` | Regras de permitir sempre: `{ "agent", "cwd", "tool", "target" }`, cada uma comparada exatamente |
| `zeca_species` | `"atratus"` | A espécie do Zeca, pelo id (`atratus` é o urubu-de-cabeça-preta; os ids estão em `ui/src/character/flock/species.ts`); um desconhecido desenha o urubu-de-cabeça-preta |
| `zeca_look` | `"auto"` | O que o Zeca usa: `auto` (o visual do calendário: `witch-hat` de 1º de outubro a 1º de novembro, `santa-hat` de 1º a 26 de dezembro, `party-hat` de 31 de dezembro a 2 de janeiro, `bunny-ears` da Sexta-feira Santa à segunda-feira depois da Páscoa), `none`, ou um desses ou um visual do ano todo (`sunglasses`, `west-coast`, `fitted-cap`, `mountain-hat`, `headband`, `dreads`, `front-knot`, `durag`, `crown`, `bucket-hat`, `clock-chain`, `headphones`, `shutter-shades`, `chrome-chain`, `eye-patch`). Um desconhecido vira `auto` |
| `flock` | `"brazil"` | De onde vêm as aves das outras sessões: `brazil` (os urubus do Brasil), `americas` (com os dois condores) ou `world` (todos os urubus). Cada projeto sorteia a espécie dele daqui, a mesma a cada início; mudar isto sorteia todos os projetos de novo. O urubu-rei vem pelo papel de qualquer jeito |
| `presence` | `"island"` | A predefinição de presença: `island` (o bando no topo da tela), `panel` (o Zeca na bandeja do painel; a ilha abre perto do painel quando você clica nele ou quando um card precisa de você), `quiet` (nada em repouso; um card ainda abre a ilha com o seu som) ou `paused` (os cards vão na hora para os terminais dos agentes, os conectores param, sem notificações). Arquivos da versão 4 guardam `island` ou `panel` e são lidos como estão. Um valor desconhecido vira `island` |
| `zeca` | `true` | O Zeca, o companheiro: desligado, sem Chat ou microfone do Chat, e a bandeja fica sem *Chat…*; o ditado local independente, o bando, os cards, as notificações e os conectores continuam funcionando |
| `widget` | ausente | O canto do widget de canto: `top-left`, `top-right`, `bottom-left` ou `bottom-right`; ausente (o padrão) para nenhum widget. Um valor desconhecido é nenhum widget |
| `projects` | ausente | Escolhas por pasta de projeto, definidas pelas ações rápidas de uma sessão ou em **Settings → Projects**: `{ "/home/me/site": { "pin": true }, "/home/me/x": { "mute": true, "hide": true } }`. `mute`: sem sons nem notificações das suas sessões em repouso (um card mantém os dois); `pin`: as suas sessões primeiro; `hide`: as suas sessões fora da ilha, da bandeja e do widget, exceto enquanto uma delas tem um card esperando; `species`: a ave das suas sessões, um id de espécie do renderer (`"vultur"`), qualquer uma menos a do urubu-rei (`"papa"`), ignorada como uma desconhecida (a partir da versão 12). Só as escolhas ligadas são escritas, e um projeto sem nenhuma é removido (a partir da versão 8) |
| `budget_daily_cents` | ausente | O orçamento diário, em centavos de dólar (`500` é $5,00), definido em **Settings → Activity**: em 80% uma notificação, e quando acaba outra, e o Vults não inicia nenhuma sessão nova naquele dia. Nunca para nem bloqueia um agente. Ausente, sem orçamento (a partir da versão 20) |
| `budget_told` | ausente | O passo do orçamento já avisado hoje (`2026-10-11:1` para 80%, `:2` quando acabou), para que um reinício não avise de novo (a partir da versão 20) |
| `dnd_until` | ausente | Não perturbe até esse momento, em segundos desde a época Unix: sem sons e sem notificações em repouso, sem lembretes; um card ainda abre a ilha com o seu som (e, no *Panel*, a sua notificação). Definido em **Settings → General**; um horário que já passou significa desligado (a partir da versão 9) |
| `history` | `true` | Guardar um histórico local dos turnos dos agentes para **Settings → Activity**: as contagens de cada turno terminado (12 semanas) e os totais de cada dia (um ano), no `vults.sqlite` da pasta de dados. Desligado, nada novo é guardado e o que existe fica; **Clear history** apaga tudo (a partir da versão 13), e o log de auditoria junto |
| `recap_shown_week` | ausente | A segunda-feira da última semana cujo card de resumo você leu na ilha, para que ele venha uma vez só (a partir da versão 14) |
| `theme` | `"black"` | **Settings → General → Theme**: `black` ou o tema neutro `light`, compartilhado por todas as janelas. Um valor desconhecido usa Black (a partir da versão 23) |
| `language` | ausente | O idioma do app: `en`, `pt-BR`, `es` ou `zh` (chinês simplificado), definido em **Settings → General**. Ausente segue o sistema (`LC_ALL`, `LC_MESSAGES` ou `LANG`; o locale do usuário no Windows), e um idioma que o Vults não fala vira inglês (a partir da versão 15) |
| `notifications` | `true` | Notificações do desktop, só em *Panel* (no topo da tela a ilha já mostra tudo): uma sessão terminou, falhou ou ficou quieta, e um card esperando, na hora. A única ação delas abre a ilha |
| `visitors` | `true` | De vez em quando, enquanto há sessões abertas, um urubu de fora do bando cruza o céu uma vez, sem nunca pousar |
| `api_provider` | `"anthropic"` | O provedor do chat por API: `anthropic`, `openai`, `google`, `openrouter`, `groq`, `deepseek`, `mistral`, `xai`, `ollama`, `lmstudio` |
| `api_models` | `{}` | Provedor → o modelo escolhido para ele (as chaves nunca ficam aqui) |

<a id="agent-configs-vults-edits"></a>

## Configurações de agentes que o Vults edita

Só quando você clica em **Write the file**, depois de um backup datado e de um diff que você revisou:

| Agente | Arquivo | O que é adicionado |
|---|---|---|
| Claude Code | `~/.claude/settings.json` | Uma entrada de hook por evento, executando `vults-hook --agent claude`, e um `statusLine` executando `vults-hook --agent claude --statusline` (se você tem um seu, só o `command` dele muda, e o hook continua executando o seu) |
| Codex | `~/.codex/hooks.json` | Uma entrada de hook por evento, executando `vults-hook --agent codex` |
| Antigravity | `~/.gemini/config/hooks.json` | Um hook de nível superior chamado `vults`, com um handler por evento executando `vults-hook --agent antigravity <Event> \|\| exit 0` (timeouts em segundos). Os outros hooks nomeados no arquivo nunca são tocados, **Remove hooks…** apaga só a chave `vults`, com tudo o que estiver dentro dela, e desligá-lo com `"enabled": false` no Antigravity é respeitado. Um hook `vults` seu que não executa o nosso nunca é sobrescrito: a instalação é recusada |
| OpenCode | `~/.config/opencode/plugins/vults.js` (sob `$XDG_CONFIG_HOME` quando definido) | Um plugin nosso, o arquivo inteiro, que repassa os passos, as permissões e as perguntas do OpenCode pelo `vults-hook --agent opencode`. Um arquivo ali sem a primeira linha *Generated by Vults* é seu e nunca é escrito |
| Qwen Code | `~/.qwen/settings.json` | Uma entrada de hook por evento, executando `vults-hook --agent qwen` |
| Copilot CLI | `~/.copilot/hooks/vults.json` | Um arquivo nosso, um hook por evento executando `vults-hook --agent copilot`. O Copilot lê todos os arquivos dessa pasta; os outros nunca são tocados |
| Cursor | `~/.cursor/hooks.json` | Um hook por evento, executando `vults-hook --agent cursor <event> \|\| exit 0`; nenhum dos hooks que rodam antes de um passo, cuja saída o Cursor lê como uma decisão |
| Factory Droid | `~/.factory/settings.json` | Uma entrada de hook por evento, executando `vults-hook --agent droid` |
| Kimi Code | `~/.kimi-code/config.toml` | Uma tabela `[[hooks]]` por evento, executando `vults-hook --agent kimi`; os seus comentários e o seu layout ficam |

O Vults não suporta mais o Gemini CLI. Enquanto os hooks que um Vults anterior instalou em
`~/.gemini/settings.json` continuarem lá, eles não mandam nada, e **Settings → Agents** mostra um
card do Gemini CLI cujo único botão é **Remove hooks…** (o mesmo diff, backup datado e clique).

A variável de pasta de cada agente move o arquivo dele, lida do jeito que o agente a lê:
`CLAUDE_CONFIG_DIR` (Claude Code, no lugar de `~/.claude`), `CODEX_HOME` (Codex, no lugar de
`~/.codex`, o `config.toml` também), `QWEN_HOME` (Qwen Code, no lugar de `~/.qwen`), `COPILOT_HOME` (Copilot CLI, no lugar de
`~/.copilot`), `FACTORY_HOME_OVERRIDE` (Factory Droid, no lugar da home: a `.factory` dele fica
dentro), `KIMI_CODE_HOME` (Kimi Code, no lugar de `~/.kimi-code`) e
`GEMINI_CLI_HOME` (o Gemini CLI guarda a sua `.gemini` dentro dela; lida só para achar os hooks
antigos a remover). Aberto pelo desktop, o app
não vê o que os arquivos de inicialização do seu shell exportam, então ele pergunta ao seu shell de
login uma vez, quando abre, e guarda só essas sete; um shell que leva mais de dois segundos é
deixado de lado, e valem as pastas de sempre. O Claude Code e o Codex que o próprio app abre (o chat,
a leitura de uso do Codex) também as recebem, então usam as mesmas pastas que no seu terminal. Cada
card em **Settings → Agents** mostra o arquivo que ele edita. O `OPENCODE_CONFIG_DIR` do OpenCode ainda não é seguido.

Uma escrita nunca deixa meio arquivo: o texto novo vai para um arquivo próprio ao lado, é gravado
no disco e só então toma o lugar do antigo. Um arquivo com outros nomes (um hard link, como fazem
alguns gerenciadores de dotfiles e ferramentas de backup) é escrito no próprio lugar, então todos
os nomes veem a mudança. Um arquivo salvo com marca de ordem de bytes (BOM) ou com quebras de linha
do Windows (CRLF) continua com elas.

Entradas de outras ferramentas são mantidas, e **Remove hooks…** tira só as nossas. Se você colocar o hook
de outra ferramenta no mesmo grupo que o nosso, esse grupo fica: uma atualização muda só o nosso hook nele,
e Remove tira só o nosso hook. No Codex, um hook ou grupo que vinha depois de um dos nossos que
removemos sobe uma posição, então o Codex pode pedir que você confie nele de novo em `/hooks`. Uma atualização
muda as nossas entradas onde elas estão: o arquivo mantém a ordem das chaves, então o diff mostra só o que mudou.
Hooks que executam outra cópia do `vults-hook` (de outra pasta de dados) avisam isso em **Settings →
Agents**, e **Update hooks…** os aponta para o deste app (se o seu próprio status line estiver salvo ao lado
daquele outro hook, o card não oferece atualização e avisa: remova os hooks primeiro, veja abaixo).

Uma configuração tem um único `statusLine`, e o Claude Code informa o uso do plano só para ele. Se você
tem um seu, a instalação salva o objeto inteiro dele em `statusline-previous.json` ao lado do
hook e muda só o `command` dele para o nosso (o seu `padding` e os outros campos ficam); o diff mostra
os dois arquivos. O nosso então entrega o uso ao app, executa o seu comando com a mesma entrada (via
`sh -c`, por até 10 segundos) e imprime exatamente o que ele imprime (até 64 KiB), cores incluídas. Se
o seu comando sumiu, não consegue iniciar ou demora mais, o status line fica em branco daquela vez, o que
ele tiver iniciado em segundo plano também é parado, e o Claude Code segue em frente. **Remove hooks…** devolve o seu objeto
como estava; um `statusLine` que você mudou desde então não é mais nosso e fica como está. Hooks instalados
por outra pasta de dados mantêm a sua linha ao lado do hook deles: remova-os antes de instalar a partir desta.

<a id="environment"></a>

## Ambiente

| Variável | Efeito |
|---|---|
| `VULTS_NO_LAYER_SHELL` | Usa uma janela comum sempre no topo mesmo onde existe layer-shell |
| `VULTS_LOG` | Filtro do log, por exemplo `debug` (padrão `info`) |
| `VULTS_LEAN` | Renderiza a ilha em software (WebKit sem compositing): cerca de 44 MB a menos, mas o WebKitGTK 2.54 e mais novos deixam partes da ilha sem pintar. Desligado por padrão |
