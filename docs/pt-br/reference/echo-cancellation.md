# Cancelamento de eco
<!-- source: d60e3a8baa87 -->

A V6.5.1 qualifica um caminho nativo no Linux com inventário somente de leitura e um experimento
sintético reproduzível. A V6.5.2 acrescenta propriedade duplex nativa explícita para essa rota e
uma escolha separada de fones. Ainda não há entrada do renderer nem ativação visível do microfone;
a aceitação física com fala e alto-falantes continua pendente antes da interface da V6.7.

<a id="candidate-inventory"></a>
## Inventário de candidatos

`vults_audio::echo` usa a feature existente `meeting-capture` e a conexão Pulse local limitada.
`discover()` não abre streams de áudio, altera padrões/rotas nem carrega módulos. Ela recusa
endpoints remotos e servidores incompatíveis e exige tudo a seguir:

- servidor Pulse do PipeWire e `module-echo-cancel` configurado explicitamente com `aec_method=webrtc`;
- exatamente uma fonte que não seja monitor e um destino pertencentes ao módulo;
- `node.group` não vazio correspondente, seriais válidos, nomes distintos e metadados de amostragem compatíveis.

Argumentos entre aspas seguem um subconjunto estrito do parser do PipeWire: as aspas começam logo
após `=`, o fechamento encerra o valor e escapes ou sintaxe ambígua recusam o candidato. Rótulos,
um método dentro de outro argumento entre aspas e inferência do motor padrão nunca estabelecem um
par. Módulos nativos sem metadados de propriedade Pulse ficam fora do subconjunto aceito.

O `Pair` opaco registra cookie do servidor, módulo, índices/seriais dos endpoints e grupo; sua
saída Debug omite nomes. `present()` compara um novo snapshot. Nem descoberta nem uma nova verificação
bem-sucedida concedem uma rota, comprovam a acústica ou autorizam interrupção. A enumeração pode
concorrer com criação de streams ou mudança de rota. Nomes/seriais são identificadores locais,
sem representar uma fronteira de confiança.

<a id="both-directions-are-required"></a>
## As duas direções são necessárias

A fonte de cancelamento fornece amostras limpas do microfone. O destino pareado recebe a saída
exata do assistente como referência remota e a encaminha ao alto-falante. Escolher apenas a fonte
enquanto o CPAL toca na saída padrão não estabelece esse caminho de referência.
[Contrato de quatro streams do PipeWire](https://docs.pipewire.org/page_module_echo_cancel.html).

A entrada CPAL padrão da V6.4 mantém a interrupção por voz desativada. A entrada separada `realtime::run_routed`
da V6.5.2 exige consentimento realtime explícito, rota escolhida e autoridade nativa atual. Um único
worker fixa captura e reprodução PCM16 mono a 24 kHz nos índices exatos dos dispositivos escolhidos,
confere formato negociado/confirmações de destino e define `DONT_MOVE`. Não há fallback para padrões.
`Route::echo` valida o par escolhido; `Route::headphones` é uma afirmação explícita do usuário, sem
inferência por rótulos. `Route::speakers` mantém a interrupção desligada.

<a id="routed-native-ownership"></a>
## Propriedade nativa das rotas

A assinatura precede o snapshot inicial do grafo. A rota registra identidades de servidor/nós,
propriedade do módulo, conexões internas de captura/reprodução do AEC, seus endpoints físicos e
portas ativas. Movimentos internos, mudanças/remoções físicas relevantes, alterações do módulo ou
eventos conservadores de placas revogam a sessão inteira. Mudanças dos dispositivos padrão não
redirecionam streams fixados. Outras alterações observadas suspendem a verificação imediatamente;
somente um snapshot correspondente e estável entre eventos a restabelece. Amostras capturadas durante
a suspensão da verificação são descartadas. O grafo é consultado pelo menos a cada 100 ms; ausência de
validação/entrada por 500 ms, tempo inválido, overflow ou falha do servidor encerram o proprietário.
A inicialização tem limite de três segundos, sessões de dez minutos e respostas de 120 segundos
de PCM de origem. Filas de entrada/saída e cópias de metadados são limitadas.

Um worker nativo mantém a concessão do app durante a limpeza e compartilha a exclusividade de saída
com CPAL. Ele verifica revogação antes de conectar/despausar e durante o processamento. O armazenamento
de callbacks continua sob propriedade até cancelar as operações nativas; uma saída encerrada mantém
sua referência nativa até uma confirmação de exclusão. A ordem de destruição de streams/contexto/laço
é explícita. Isso usa `libpulse` do sistema por `libpulse-sys`, sem protocolo Pulse reimplementado nem
carregador de módulos de produção. Compilar no Linux exige os headers do libpulse; instaladores
incluem a biblioteca em suas dependências.

A reprodução mapeia trechos de conteúdo para posições do stream, incluindo intervalos de silêncio.
O relógio bruto sozinho não conta palavras ouvidas: silêncio por underrun é excluído, a latência
informada do destino é subtraída e tempo corrompido/antigo encerra a saída. Fim do produtor, conteúdo
consumido e confirmação bem-sucedida de drenagem continuam separados. Parar congela o último cursor
de conteúdo confirmado e descarta a saída pendente; o stream antigo é encerrado antes de outra
resposta tocar. A estimativa continua na fronteira de destino informada; o tempo físico posterior
ainda exige validação acústica.

`run_routed` encaminha eventos `Input` limitados e apenas em memória ao chamador nativo. Para
rotas com fones/AEC escolhidas explicitamente, a V6.7.3 agora também consome esses pacotes por um
adaptador local de interrupção. Modelos instalados de transcrição/VAD são verificados antes de abrir
streams nativos. A detecção começa somente após reprodução real, mantém um buffer circular de oito
segundos e observa janelas recentes de fala. A máquina de turnos existente controla bloqueios de
reprodução e tokens exatos de verificação; uma transcrição local separada pode confirmar palavras
dentro do prazo da verificação. Seu texto nunca vira comando, permissão, rascunho do Chat ou entrada
do provedor. Resultados vazios, com falha, expirados ou revogados não confirmam interrupção.

Um único worker nativo limitado mantém a concessão do desktop durante inferência e destruição dos
modelos. Cancelamento, perda da rota, gerações substituídas e encerramento do proprietário suprimem
publicação atrasada. Após uma interrupção confirmada, `Replay` vem depois da nova ação `Listen` e
carrega o início retido da fala e seu trecho anterior nessa geração exata. `Command::Capture` nativo
agora aceita as operações `Begin`, `Finish` e `Cancel` vinculadas à geração nesse mesmo stream. Begin
inicia a captura com esse replay e acrescenta pacotes recebidos após Begin, mantendo até 30 segundos
de PCM16 de 24 kHz; não abre um segundo microfone.
Somente Finish inicia uma transcrição local final separada e cancelável. Palavras finais válidas
acompanham o PCM original até o roteador existente de turnos completos; palavras da verificação
nunca são reutilizadas. Decodificação vazia/com falha, cancelamento, substituição, perda da rota e o
prazo de 60 segundos impedem envio. O worker real mantém a concessão do desktop durante o
encerramento. Replay sozinho nunca grava/confirma um turno seguinte, e nenhum áudio é salvo ou
enviado ao Chat.

A política escolhida de fones/AEC continua separada da validade da rota ao vivo. A página de voz
comum do Nest ainda usa captura manual e o botão Interromper resposta; isso não ativa uma rota na
interface. Seleção/consentimento explícito na interface, conexão dos controles da rota e qualificação
física/com provedor real continuam na V6.7. A decisão de configuração de produção da Q6 segue aberta;
o Vults não carrega módulos do sistema silenciosamente.

<a id="reproduce-the-isolated-experiment"></a>
## Reproduzir o experimento isolado

Pré-requisitos: Linux com módulos Pulse e WebRTC de eco do PipeWire, WirePlumber 0.5, `pactl`,
`pw-cat` e `pw-dump`. O exemplo espera os dados originais instalados do WirePlumber em
`/usr/share/wireplumber`. Na raiz do repositório:

```sh
cargo run -p vults-audio --features meeting-capture --example echo-cancel-spike
```

O comando explícito de desenvolvimento inicia um core PipeWire privado e um WirePlumber apenas de
política, com sockets Unix, cookie Pulse e diretório temporário modo 0700 privados. Ele fixa as
configurações privadas de cliente PipeWire/Pulse, copia a configuração principal original instalada
do WirePlumber ao diretório e aplica somente suas próprias substituições de perfil. A busca de
scripts do WirePlumber fica fixada nos dados originais instalados. Caminhos privados de estado/cache,
D-Bus, monitores de hardware e restauração de estado desativados separam o experimento das rotas
normais do desktop. Ele nunca altera `HOME`. A busca exclusiva de configuração/scripts é necessária
porque caminhos XDG privados sozinhos não excluem todas as substituições do usuário; veja a
[busca de configuração do PipeWire](https://github.com/PipeWire/pipewire/blob/1.6.9/src/pipewire/conf.c)
e os [locais do WirePlumber](https://pipewire.pages.freedesktop.org/wireplumber/daemon/locations.html).

Somente destinos nulos privados e seus monitores alimentam o teste. Um dump limitado do grafo
recusa nós ALSA ou Bluetooth antes de iniciar streams. Os processos têm prazos; filhos são
encerrados/recolhidos e o diretório temporário é removido ao concluir normalmente ou tratar erros.
PCM float sintético fica na memória/pipes; não há microfone humano, alto-falante do desktop,
provedor, chave ou rede. Encerramento externo abrupto pode impedir a limpeza normal dos processos,
como em outros comandos locais de desenvolvimento.

A entrada fixa tem oito segundos de ruído com semente, mono a 48 kHz. Um microfone simulado recebe
essa referência atrasada em 20 ms e atenuada por 0,5, mais dois tons locais de amplitude 0,03 a
300/700 Hz a partir do sexto segundo. Controle de ganho, supressão de ruído e filtro passa-altas são
desligados explicitamente; `aec_args` precisa manter suas aspas externas quando `pactl` une os
argumentos. Isso separa a medição de AEC da supressão adicional. A versão instalada do processamento
WebRTC é registrada sem inferir implementação/versão de AEC pelo rótulo de um dispositivo.

Antes da medição, o limite sintético foi definido em pelo menos 15 dB de redução de eco após o
aquecimento e cada tom local entre -3 e +3 dB da amplitude de entrada. A pontuação usa 3-4,5 segundos
para eco e 6,3-7,5 segundos para os tons, a partir do início capturado. O JSON informa medições e
`synthetic_gate_passed`; uma medição negativa válida ainda termina com sucesso. Erros de processo,
captura, inventário ou metadados fazem o comando falhar. `physical_acceptance` é sempre false.

<a id="recorded-evidence-and-limits"></a>
## Evidências registradas e limites

A [fixture de 2026-10-19](../../../crates/spike/fixtures/echo-cancellation-2026-10-19.json) registra três
execuções consecutivas, fingerprints dos fontes e versões instaladas: PipeWire 1.6.9, WirePlumber
0.5.18 e processamento de áudio WebRTC 1.3. As três passaram no limite sintético fixo. O resultado
sustenta continuar com o motor nativo do PipeWire; o experimento não justifica outra crate de AEC.

| Métrica | Faixa nas três execuções | Limite fixo |
|---|---|---|
| Redução de eco | 20,233 a 20,299 dB | Pelo menos 15 dB |
| Ganho local a 300 Hz | -1,218 a -0,998 dB | -3 a +3 dB |
| Ganho local a 700 Hz | -0,523 a -0,317 dB | -3 a +3 dB |

Trata-se de um único caminho sintético estacionário de eco, dois tons locais e um relógio virtual
compartilhado. Isso não qualifica fala humana simultânea, acústica de microfone/alto-falantes,
caminhos de eco móveis, mudanças de ganho, hotplug, estabilidade longa, latência física nem a rota
completa do app. Nenhuma opção de produção ou aceitação física/real é marcada como concluída.
Os fingerprints da V6.5.1 se referem ao commit `5099a43d`; a evidência atual de rotas está abaixo.

<a id="routed-validation"></a>
## Validação das rotas

Execute a verificação nativa dentro do mesmo grafo privado:

```sh
cargo run -p vults-audio --features meeting-capture --example echo-cancel-spike -- --duplex
```

Ela altera somente padrões/rotas/módulos privados e testa captura/confirmações reais de destino,
drenagem completa, cursor congelado na interrupção, outra resposta após parar, recusa de movimentos
manuais, revogação ao mover uma conexão interna do AEC, política explícita de fones/alto-falantes,
remoção do módulo, recusa de seleção antiga e liberação da concessão nativa. A
[fixture de 2026-10-20](../../../crates/spike/fixtures/duplex-routing-2026-10-20.json) registra três
execuções finais consecutivas bem-sucedidas, com fingerprints do commit `86d6b80d`. Regressões puras
e do proprietário nativo também cobrem
silêncio entre trechos, latência/relógio corrompido, revogação de admissão, referências de workers e
evidência antiga de interrupção.

O backend da V6.5 está entregue. A próxima etapa é ferramentas na V6.6, depois captura/interface na
V6.7; fala física, fala simultânea, latência acústica completa, hotplug e aceitação longa continuam
em aberto. A verificação isolada comprova rotas nativas sintéticas, sem aceitação física.
