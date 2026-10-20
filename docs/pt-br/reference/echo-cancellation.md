# Cancelamento de eco
<!-- source: 9a23d56913f7 -->

A V6.5.1 qualifica um caminho nativo no Linux com inventário somente de leitura e um experimento
sintético reproduzível. Ela não ativa interrupção por voz. A V6.5.2 deve vincular os dois streams
reais ao par escolhido antes de considerar interrupção com alto-falantes; a aceitação física com
fala continua pendente.

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

O worker CPAL da V6.4 ainda usa a saída padrão; por isso a entrada do app mantém a interrupção
desligada. A próxima task deve fixar captura e reprodução, confirmar os destinos dos streams,
impedir movimentos silenciosos e revogar a geração da rota em mudanças de dispositivo/módulo/servidor.
Drenagem e contabilidade do texto ouvido devem continuar ligadas à saída consumida. Sem AEC
qualificado, a interrupção com alto-falantes fica desligada; com fones, exige escolha explícita do
usuário, sem inferência pelo nome do dispositivo. A configuração e a escolha da Q6 sobre carregar
módulos continuam pendentes; esta task não acrescenta carregador de produção nem nova entrada de microfone.

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
A próxima etapa é propriedade duplex roteada na V6.5.2, depois ferramentas na V6.6 e captura/interface
na V6.7.
