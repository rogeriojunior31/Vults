# 0019. Rede, segredos e telemetria
<!-- source: 65945568df0c -->

**Status:** Aceita, 2026-10-10. Consolidada em 2026-10-12 a partir da 0006 e da 0016.
Alterada em 2026-10-17 para a N3 V0.4: requisições a modelos no ditado, limites de loopback e
gastos de voz em tempo real. São requisitos de implementação para V3, V5 e V6, não requisições
habilitadas ou entregues.

<a id="context"></a>

## Contexto

O app vê comandos, nomes de arquivo, diffs e chaves de API, ao lado de trabalho privado. Alguns
recursos ainda precisam da rede enquanto o usuário não está pedindo naquele momento: conectores,
busca e fetch na web durante um turno do chat, o sono do Zeca, rotinas, o relay e o push do
celular, respostas faladas, a checagem de atualização. Cada um manda para fora da máquina algo que
importa ao usuário, custa dinheiro ou cota, e poderia virar telemetria por acidente.

<a id="decision"></a>

## Decisão

- **Segredos só no chaveiro do sistema**, nunca num arquivo ou num log.
- **Nunca telemetria.** O app não manda nada sobre o próprio uso; nenhuma requisição leva uso, um
  id ou qualquer coisa sobre o usuário além do que o recurso precisa para fazer o seu trabalho.
- **Uma requisição que o usuário inicia** com um clique ou uma mensagem (um turno do chat, um chat
  por API) é escolha explícita dele, dita onde ela é ligada.
- **Uma requisição em nome do usuário** (uma que sai da máquina sem um clique naquele momento,
  inclusive uma busca ou fetch na web que o modelo faz durante um turno que o usuário começou) é
  um recurso com:
  - um **interruptor** próprio no Settings, desligado por padrão; ligá-lo diz o que manda, para
    onde, e quanto custa;
  - um **orçamento** (requisições, tokens ou dinheiro por dia), e ele para quando acaba;
  - um **registro local** que o usuário pode ler (o quê, para onde, quando, custo), mandado para
    nenhum outro lugar;
  - **um único portão de rede, do `app`** (`app/src/gate.rs`), que confere o interruptor e o
    orçamento. Nenhum crate abre uma conexão para um recurso desses por fora dele.
- Os conectores (o GitHub pelo `gh`, hoje) são recursos assim: cada um fica desligado até o usuário
  conectá-lo, e o polling dele passa pelo portão.
- **As atualizações são pedidas e assinadas**:
  - A checagem fica desligada até o usuário ligá-la (no Settings, e oferecida uma vez na primeira
    execução). Ligada, ela busca um manifesto estático do GitHub Releases no máximo uma vez por dia
    e não manda nada além da requisição: nenhum id, nenhum uso, nenhuma versão numa query string.
  - Uma atualização só é instalada depois de um clique que mostra a versão e as notas dela. Um
    agente rodando nunca é interrompido: o reinício espera até o usuário escolher.
  - Três canais, cada um com a própria URL de manifesto: **stable** (o padrão), **beta** (release
    candidates) e **nightly** (commits selecionados). Um build sabe o seu canal em tempo de
    compilação, então um build stable nunca lê o manifesto nightly; trocar de canal é reinstalar.
  - Toda atualização é assinada. A chave privada vive só nos segredos do job de release; a chave
    pública vem embutida no app. Um manifesto ou pacote que falha na verificação é ignorado e
    registrado no log.
  - Pacotes que uma distribuição atualiza (AUR, um futuro Flatpak) saem com o atualizador fora da
    compilação.

<a id="dictation-model-requests"></a>

## Requisições a modelos no ditado

Transcrição local, processamento pelo dicionário e comandos de voz determinísticos não chamam um modelo remoto.
Os recursos opcionais de modelos da N3 abaixo são requisições separadas em nome do usuário, não
permissão para enviar uma transcrição só porque a gravação ou o Chat está ligado. Cada interruptor
fica desligado por padrão e é alterado na webview das Configurações, com provedor, dados e limite
explicados antes da ativação. Reutilizar o provedor do Chat não reutiliza o consentimento para
outro recurso.

| Recurso do portão | Fase | Dados enviados ao modelo escolhido | Limite em qualquer janela móvel de 24 horas | Quando desligado, esgotado ou indisponível |
|---|---|---|---|---|
| `dictation-cleanup` | V3 | O texto ditado e o contexto de vocabulário/estilo necessário para filtrar ou traduzir; sem áudio | 2.000 tentativas de requisição ao modelo, incluindo novas tentativas | Manter a transcrição local bruta; nunca respondê-la nem executá-la |
| `meeting-summary` | V5 | A transcrição da reunião necessária ao resumo e às tarefas; sem áudio gravado | 20 tentativas de requisição ao modelo, incluindo novas tentativas | Manter a transcrição local; deixar o resumo por modelo indisponível |
| `voice-realtime` | V6 / K8 | Áudio da conversa ao vivo e o contexto limitado da conversa informado nas Configurações | Um limite monetário positivo, com padrão de US$ 1; zero desabilita voz remota | Parar o áudio remoto e continuar em texto com uma explicação |

- **Um portão, orçamentos separados.** Esses identificadores passam pelo portão do `app` antes
  de iniciar uma requisição ou conexão em tempo real. Tentativas, inclusive chamadas que falham
  e novas tentativas, contam na cota correspondente; nenhuma troca de modelo ganha uma
  requisição sem contagem. Os orçamentos sobrevivem ao reinício e não são zerados à meia-noite.
  Trabalhos concorrentes compartilham a mesma cota. Orçamento ausente ou ilegível não pode
  conceder uma cota nova. Desligar um interruptor cancela o trabalho de modelo daquele recurso
  em andamento; a transcrição local continua utilizável.
- **Tempo real reserva o gasto antes de enviar.** A V6 deve estender o portão atual, que conta
  requisições, com contabilidade monetária e uma cota limitada de transmissão antes de executar
  `voice-realtime`. Incluir reservas em andamento, reconciliar o uso informado e parar a
  transmissão antes de esgotar a cota. Tarifas ausentes ou uso desconhecido não podem significar
  custo zero ou áudio ilimitado; recusar mais áudio remoto quando não for possível estabelecer
  uma cota restante segura. O valor no protótipo é um limite escolhido, não preço do provedor
  nem comprovação de que a contabilidade já existe.
- **Loopback não ignora consentimento nem cota (Q3).** Chamadas locais da N3 ao Ollama/LM Studio
  continuam usando o interruptor do recurso, a cota de requisições e o registro local pelo
  portão. Não consomem orçamento monetário remoto. Isso substitui a proposta inicial de ignorar
  todo o orçamento: trabalho de modelo local ainda precisa de um limite finito de requisições
  sob este registro. Para ser considerado loopback, usar um endpoint com IP de loopback
  explicitamente validado, sem proxy nem redirecionamentos; o nome do provedor ou um hostname
  como `localhost` é insuficiente. Não tentar silenciosamente um provedor remoto. Esse contrato
  rege a conexão do Vults, não garante que um serviço local configurado independentemente não
  encaminhe dados para fora. O caminho existente de embeddings para lembranças não muda; ele
  não autoriza uma nova requisição da N3 por fora do portão.
- **Registrar operações, não o conteúdo do ditado.** Manter localmente recurso, provedor/modelo,
  classificação local/remota, tempos, resultado e uso/custo disponível. Nunca incluir áudio
  bruto, transcrição, conteúdo da área de transferência, chaves ou tokens de restauração num
  registro de requisição. Custo desconhecido continua desconhecido. O histórico de transcrições
  segue sua escolha separada de retenção, incluindo *Não guardar*. Nenhum registro é enviado a
  um provedor ou usado como telemetria.
- **Falhas preservam o trabalho local.** A limpeza é um filtro de texto sem ferramentas:
  recusa, tempo esgotado ou saída inválida usa a transcrição bruta e não altera a proteção de
  entrega da [0032](0032-dictation-into-other-apps.md). A transcrição de reuniões não depende do
  consentimento para resumo. Consentir com voz remota não habilita escuta de resposta nem
  limpeza por modelo; a [0028](0028-spoken-replies-through-a-realtime-model.md) continua regendo
  a escuta e o áudio.

<a id="semantic-command-requests-vc1"></a>
## Requisições de comandos semânticos (VC1)

`voice-command-intent` é um recurso separado desligado por padrão, independente do Chat, voz em
tempo real e envio de fala desconhecida ao Chat. Antes de ativar, as configurações mostram o
provedor/modelo, o envio de transcrição e rótulos mínimos/IDs opacos dos destinos e o limite móvel
de **200 tentativas por 24 horas**, incluindo falhas. Registrar a admissão de forma persistente
antes de cada tentativa, inclusive para endpoints locais. Cada fala permite uma requisição,
sem repetição automática, com prazo de dez segundos. Transcrição bruta, rótulos dos destinos,
credenciais e resposta do modelo não entram no registro de rede.

O transporte inicial suportado é o cliente qualificado só de texto do Claude Code, usando a escolha
explícita do login da CLI e os limites da assinatura. Ter um provedor configurado no Chat não é
consentimento para este recurso. API/modelos locais exigem o mesmo contrato de isolamento/saída
antes de serem oferecidos; preços de API devem ser explicados ao selecionar. A consulta só de texto
pela assinatura Codex continua indisponível enquanto a qualificação de autenticação/isolamento
for falsa. Nunca recorrer silenciosamente a chave de API, outro provedor/conta ou turno comum do
Chat em caso de falha.

Com a opção desligada, cota esgotada, provedor ausente/não suportado ou estado do orçamento
indisponível, não enviar nada e explicar o motivo localmente. Desativação, cancelamento, bloqueio
ou troca de provedor/conta invalida a saída pendente antes de qualquer ação. Opções locais próprias,
desligadas por padrão, autorizam apenas as ações tipadas da
[0032](0032-dictation-into-other-apps.md#natural-voice-commands-vc1); consentimento de rede não
concede execução no desktop. Planejar este contrato não ativa requisições; a implementação deve
adicionar o ponto de chamada ao inventário de rede e guia de segurança e testar cota, reinício e
cancelamento antes da entrega.

<a id="consequences"></a>

## Consequências

- Um teste roda um dia com todos os recursos desligados e conta zero requisições.
- Antes de entregar os recursos de modelos da N3, estender essa verificação a cada recurso,
  incluindo endpoints locais. Cobrir limites exatos de cota, novas tentativas, reservas
  concorrentes, reinício, estado de orçamento ilegível, desligamento durante a transmissão e
  custo desconhecido em tempo real, além do retorno à transcrição bruta e registros sem seu
  conteúdo. Registrar os pontos de chamada reais no inventário de rede e no guia de segurança
  quando implementados; declarar identificadores aqui não adiciona acesso à rede.
- Nenhum número de uso guia o roadmap: aprendemos com as issues e usando o app nós mesmos. Tudo o
  que aprenderia com os dados dos usuários tem de aprender na máquina do usuário, ou não aprender.
- O atualizador chega depois da 0.2.0 ([0020](0020-0.2.0-at-wave-4-then-0.2.x.md)). O
  `release.yml` vai assinar os pacotes e publicar um manifesto por canal; a chave é criada uma vez e
  guardada offline (perdê-la significa que os usuários reinstalam à mão). Os builds nightly e beta
  ganham um sufixo próprio no bundle id, para instalar ao lado do stable sem dividir configurações
  nem o chaveiro.

<a id="history"></a>

## Histórico

- 2026-10-26: a VC1 define ações limitadas de comandos naturais e consentimento independente de modelo antes da 0.1.9.

- 2026-10-17: a N3 V0.4 nomeia os três recursos de modelos com adesões independentes e seus
  limites móveis. A Q3 mantém chamadas de loopback sob interruptor, cota e registro; só a
  contabilidade monetária remota fica isenta. Tempo real exige contabilidade de gastos antes
  que a implementação possa ser entregue.
- 0006, 2026-10-04: segredos só no chaveiro; sem telemetria; sair
  da máquina é escolha do usuário.
- 0016, 2026-10-08: a checagem de atualização, pedida,
  assinada, um canal por vez.
