# Configurações e arquivos
<!-- source: 7028987a5896 -->

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
| `~/.local/share/vults/vults.sqlite` | O banco local: o histórico dos turnos dos agentes (só contagens), para Settings → Activity, o log de auditoria das respostas aos cards (90 dias), as políticas que você aceitou (guardadas até você revogá-las, e depois), os telefones que você pareou (as chaves públicas deles e o último contador com que cada um assinou; os revogados ficam, para que um telefone revogado nunca volte), os traces do Zeca (30 dias), as conversas dele (as 200 mais recentes), o diário e os resumos dos dias (90 dias), os vetores do recall, as propostas de memória (as decididas ficam 90 dias), as rodadas de sono e as suas tarefas (veja [Segurança](../safety.md)). O **Clear history** esvazia tudo isso, menos os consentimentos, os telefones e as tarefas. O `history.jsonl` e o `days.json` das versões anteriores entram nele na primeira abertura |
| `~/.local/share/vults/memory/` | A memória do Zeca: um repositório git de pequenos arquivos Markdown, cada mudança um commit (modo da pasta conforme o seu umask). O **Clear history** não mexe nela; apague a pasta para apagar os fatos dele ([A memória do Zeca](../guide/memory.md)) |
| `~/.local/share/vults/voice/` | Os modelos de voz que você baixou e o detector de fala |
| `~/.local/share/vults/backups/` | Backups datados dos arquivos que o Vults edita dentro de um projeto, como um `CLAUDE.md` que ele fez ler o `AGENTS.md` (nunca guardados no repositório) |
| `~/.local/share/vults/zeca-mcp.json` | A configuração de MCP que o Claude Code do chat recebe com `--mcp-config`: este app rodando como o servidor do Zeca (`vults --zeca-mcp`); modo `0600` |
| `~/.local/share/vults/zeca-flock.json` | As sessões da ilha (redigidas) para o `flock_list` desse servidor, reescrito quando mudam e apagado quando o app fecha; modo `0600` |
| `$XDG_RUNTIME_DIR/vults.sock` | O socket com que o hook conversa (modo `0600`) |
| `~/.config/autostart/` | A entrada que **Start with the desktop** adiciona |
| Chaveiro do sistema, serviço `io.github.rogeriojunior31.vults`, conta `<provider>-api-key` (`anthropic-api-key`, `openai-api-key`, …) | As chaves de API do chat, uma por provedor para o qual você deu uma (nunca em arquivo) |
| `~/.local/state/vults/logs/` | O log: um arquivo por dia, os últimos cinco são mantidos. Ele registra o que aconteceu (nomes de eventos, decisões, erros), nunca comandos, caminhos ou texto do chat |

`$XDG_CONFIG_HOME` e `$XDG_DATA_HOME` substituem `~/.config` e `~/.local/share` quando estão definidas.
**Settings → About** mostra os caminhos reais.

## settings.json

```json
{
  "version": 18,
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
| `version` | `20` | Versão do esquema, para que releases futuras possam migrar o arquivo. Se for mais nova que a do app (você voltou para uma release mais antiga), o app (a partir da 0.1.1) usa as chaves que conhece, guarda o arquivo como estava em `settings.json.v<version>-<time>` e avisa no log; a próxima mudança que você fizer salva só as chaves que ele conhece, na versão dele. Até lá, cada início guarda mais uma cópia. Se nenhuma cópia puder ser guardada, as mudanças não são salvas. Para voltar às configurações da release mais nova, restaure essa cópia. Um arquivo mais antigo é lido com os padrões para as chaves que faltam nele, e é salvo na versão do app |
| `connectors` | `{}` | Id do conector → ligado |
| `sounds` | `true` | Sons 8-bit |
| `volume` | `50` | O volume dos sons, em porcentagem (0 a 100); `50` é o volume em que a 0.1.0 os tocava |
| `voice_model` | ausente | O modelo de voz do chat (`base`, `small`, `turbo`), baixado em `~/.local/share/vults/voice/` (junto com o detector de fala, `ggml-silero-v6.2.0.bin`); ausente deixa a voz desligada |
| `voice_language` | ausente | O idioma que o usuário fala para a voz: um código (`pt`, `en`), `auto` para detectar a cada vez, ausente para seguir o idioma do sistema |
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
| `memory` | `false` | A memória e o diário do Zeca; desligada, nada dela chega a um modelo ([A memória do Zeca](../guide/memory.md)) (a partir da versão 18) |
| `journal_summary` | `false` | Com `memory` ligada, **A summary of each day** do diário, pelo menor modelo do Claude Code ([A memória do Zeca](../guide/memory.md)) (a partir da versão 18) |
| `local_embeddings` | `false` | Com `memory` ligada, **Recall by meaning**, por um Ollama em `127.0.0.1` ([A memória do Zeca](../guide/memory.md)) (a partir da versão 18) |
| `memory_suggest` | `false` | Com `memory` ligada, **Suggest what to remember** a partir da conversa que você deixou ([A memória do Zeca](../guide/memory.md)) (a partir da versão 18) |
| `sleep` | `false` | Com `memory` ligada, **Let Zeca sleep on it**: o sono noturno dele também pede a um modelo junções e um resumo da volta ([A memória do Zeca](../guide/memory.md)) (a partir da versão 18) |
| `journal_summary_since` | ausente | O dia em que `journal_summary` foi ligado (`YYYY-MM-DD`): o diário guardado antes nunca vai para o modelo (a partir da versão 18) |
| `fold_after` | `15` | Segundos que a ilha aberta espera, depois que o ponteiro sai, antes de se recolher (5 a 120) |
| `open_on_hover` | `false` | Deixar o ponteiro parado sobre a pílula abre a ilha, sem clique; aberta assim, ela se recolhe assim que o ponteiro sai, a não ser que você tenha clicado nela. Não vale no painel. Definido em **Settings → General** (a partir da versão 11) |
| `monitor` | ausente | A tela em que a ilha fica, como fabricante e modelo (`"Samsung Electric Company LS27AG32x"`); ausente deixa o desktop escolher. Duas telas idênticas têm o mesmo nome, e a primeira vence |
| `rules` | `[]` | Regras de permitir sempre: `{ "agent", "cwd", "tool", "target" }`, cada uma comparada exatamente |
| `zeca_species` | `"atratus"` | A espécie do Zeca, pelo id (`atratus` é o urubu-de-cabeça-preta; os ids estão em `ui/src/character/flock/species.ts`); um desconhecido desenha o urubu-de-cabeça-preta |
| `zeca_look` | `"auto"` | O que o Zeca usa: `auto` (o visual do calendário: `witch-hat` de 1º de outubro a 1º de novembro, `santa-hat` de 1º a 26 de dezembro, `party-hat` de 31 de dezembro a 2 de janeiro, `bunny-ears` da Sexta-feira Santa à segunda-feira depois da Páscoa), `none`, ou um desses ou um visual do ano todo (`sunglasses`, `west-coast`, `fitted-cap`, `mountain-hat`, `headband`, `dreads`, `front-knot`, `durag`, `crown`, `bucket-hat`, `clock-chain`, `headphones`, `shutter-shades`, `chrome-chain`, `eye-patch`). Um desconhecido vira `auto` |
| `flock` | `"brazil"` | De onde vêm as aves das outras sessões: `brazil` (os urubus do Brasil), `americas` (com os dois condores) ou `world` (todos os urubus). Cada projeto sorteia a espécie dele daqui, a mesma a cada início; mudar isto sorteia todos os projetos de novo. O urubu-rei vem pelo papel de qualquer jeito |
| `presence` | `"island"` | A predefinição de presença: `island` (o bando no topo da tela), `panel` (o Zeca na bandeja do painel; a ilha abre perto do painel quando você clica nele ou quando um card precisa de você), `quiet` (nada em repouso; um card ainda abre a ilha com o seu som) ou `paused` (os cards vão na hora para os terminais dos agentes, os conectores param, sem notificações). Arquivos da versão 4 guardam `island` ou `panel` e são lidos como estão. Um valor desconhecido vira `island` |
| `zeca` | `true` | O Zeca, o companheiro: desligado, sem chat, microfone ou atalho de fala, e a bandeja fica sem *Chat…*; o bando, os cards, as notificações e os conectores funcionam como sempre |
| `widget` | ausente | O canto do widget de canto: `top-left`, `top-right`, `bottom-left` ou `bottom-right`; ausente (o padrão) para nenhum widget. Um valor desconhecido é nenhum widget |
| `projects` | ausente | Escolhas por pasta de projeto, definidas pelas ações rápidas de uma sessão ou em **Settings → Projects**: `{ "/home/me/site": { "pin": true }, "/home/me/x": { "mute": true, "hide": true } }`. `mute`: sem sons nem notificações das suas sessões em repouso (um card mantém os dois); `pin`: as suas sessões primeiro; `hide`: as suas sessões fora da ilha, da bandeja e do widget, exceto enquanto uma delas tem um card esperando; `species`: a ave das suas sessões, um id de espécie do renderer (`"vultur"`), qualquer uma menos a do urubu-rei (`"papa"`), ignorada como uma desconhecida (a partir da versão 12). Só as escolhas ligadas são escritas, e um projeto sem nenhuma é removido (a partir da versão 8) |
| `delegate` | ausente | Em qual agente as tarefas delegadas de um projeto começam, pelo roost dele (`{ "github:me/site": "codex" }`, `claude` ou `codex`), definido com `/delegate default` no chat; quando o projeto tem 10 tarefas terminadas de um tipo, as notas delas escolhem (a partir da versão 19) |
| `budget_daily_cents` | ausente | O orçamento diário, em centavos de dólar (`500` é $5,00), definido na aba **Tasks** do Nest: em 80% uma notificação, e quando acaba outra, e o Vults não inicia nenhuma sessão nova naquele dia. Nunca para nem bloqueia um agente. Ausente, sem orçamento (a partir da versão 20) |
| `dnd_until` | ausente | Não perturbe até esse momento, em segundos desde a época Unix: sem sons e sem notificações em repouso, sem lembretes; um card ainda abre a ilha com o seu som (e, no *Panel*, a sua notificação). Definido em **Settings → General**; um horário que já passou significa desligado (a partir da versão 9) |
| `history` | `true` | Guardar um histórico local dos turnos dos agentes para **Settings → Activity**: as contagens de cada turno terminado (12 semanas) e os totais de cada dia (um ano), no `vults.sqlite` da pasta de dados. Desligado, nada novo é guardado e o que existe fica; **Clear history** apaga tudo (a partir da versão 13), e o log de auditoria junto |
| `recap_shown_week` | ausente | A segunda-feira da última semana cujo card de resumo você leu na ilha, para que ele venha uma vez só (a partir da versão 14) |
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
| Gemini CLI | `~/.gemini/settings.json` | Uma entrada de hook por evento, executando `vults-hook --agent gemini` (timeouts em milissegundos) |
| Antigravity | `~/.gemini/config/hooks.json` | Um hook de nível superior chamado `vults`, com um handler por evento executando `vults-hook --agent antigravity <Event> \|\| exit 0` (timeouts em segundos). Os outros hooks nomeados no arquivo nunca são tocados, **Remove hooks…** apaga só a chave `vults`, com tudo o que estiver dentro dela, e desligá-lo com `"enabled": false` no Antigravity é respeitado. Um hook `vults` seu que não executa o nosso nunca é sobrescrito: a instalação é recusada |

A variável de pasta de cada agente move o arquivo dele, lida do jeito que o agente a lê:
`CLAUDE_CONFIG_DIR` (Claude Code, no lugar de `~/.claude`), `CODEX_HOME` (Codex, no lugar de
`~/.codex`, o `config.toml` também), `QWEN_HOME` (Qwen Code, no lugar de `~/.qwen`) e
`GEMINI_CLI_HOME` (Gemini CLI, que guarda a sua `.gemini` dentro dela). Aberto pelo desktop, o app
não vê o que os arquivos de inicialização do seu shell exportam, então ele pergunta ao seu shell de
login uma vez, quando abre, e guarda só essas quatro; um shell que leva mais de dois segundos é
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
