# Configurações e arquivos
<!-- source: 8b1bbd34e6ef -->

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
| `$XDG_RUNTIME_DIR/vults.sock` | O socket com que o hook conversa (modo `0600`) |
| `~/.config/autostart/` | A entrada que **Start with the desktop** adiciona |
| Chaveiro do sistema, serviço `io.github.rogeriojunior31.vults`, conta `<provider>-api-key` (`anthropic-api-key`, `openai-api-key`, …) | As chaves de API do chat, uma por provedor para o qual você deu uma (nunca em arquivo) |
| `~/.local/state/vults/logs/` | O log: um arquivo por dia, os últimos cinco são mantidos. Ele registra o que aconteceu (nomes de eventos, decisões, erros), nunca comandos, caminhos ou texto do chat |

`$XDG_CONFIG_HOME` e `$XDG_DATA_HOME` substituem `~/.config` e `~/.local/share` quando estão definidas.
**Settings → About** mostra os caminhos reais.

## settings.json

```json
{
  "version": 11,
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
| `version` | `11` | Versão do esquema, para que releases futuras possam migrar o arquivo. Se for mais nova que a do app (você voltou para uma release mais antiga), o app (a partir da 0.1.1) usa as chaves que conhece, guarda o arquivo como estava em `settings.json.v<version>-<time>` e avisa no log; a próxima mudança que você fizer salva só as chaves que ele conhece, na versão dele. Até lá, cada início guarda mais uma cópia. Se nenhuma cópia puder ser guardada, as mudanças não são salvas. Para voltar às configurações da release mais nova, restaure essa cópia. Um arquivo mais antigo é lido com os padrões para as chaves que faltam nele, e é salvo na versão do app |
| `connectors` | `{}` | Id do conector → ligado |
| `sounds` | `true` | Sons 8-bit |
| `volume` | `50` | O volume dos sons, em porcentagem (0 a 100); `50` é o volume em que a 0.1.0 os tocava |
| `voice_model` | ausente | O modelo de voz do chat (`base`, `small`, `turbo`), baixado em `~/.local/share/vults/voice/` (junto com o detector de fala, `ggml-silero-v6.2.0.bin`); ausente deixa a voz desligada |
| `voice_language` | ausente | O idioma que o usuário fala para a voz: um código (`pt`, `en`), `auto` para detectar a cada vez, ausente para seguir o idioma do sistema |
| `now_playing` | `false` | Mostra na ilha a música que seus players estão tocando (MPRIS no Linux), com tocar, pausar e pular |
| `fold_after` | `15` | Segundos que a ilha aberta espera, depois que o ponteiro sai, antes de se recolher (5 a 120) |
| `open_on_hover` | `false` | Deixar o ponteiro parado sobre a pílula abre a ilha, sem clique; aberta assim, ela se recolhe assim que o ponteiro sai, a não ser que você tenha clicado nela. Não vale no painel. Definido em **Settings → General** (a partir da versão 11) |
| `monitor` | ausente | A tela em que a ilha fica, como fabricante e modelo (`"Samsung Electric Company LS27AG32x"`); ausente deixa o desktop escolher. Duas telas idênticas têm o mesmo nome, e a primeira vence |
| `rules` | `[]` | Regras de permitir sempre: `{ "agent", "cwd", "tool", "target" }`, cada uma comparada exatamente |
| `zeca_species` | `"atratus"` | A espécie do Zeca, pelo id (`atratus` é o urubu-de-cabeça-preta; os ids estão em `ui/src/character/flock/species.ts`); um desconhecido desenha o urubu-de-cabeça-preta |
| `zeca_look` | `"auto"` | O que o Zeca usa: `auto` (o visual do calendário: `witch-hat` de 1º de outubro a 1º de novembro, `santa-hat` de 1º a 26 de dezembro, `party-hat` de 31 de dezembro a 2 de janeiro, `bunny-ears` da Sexta-feira Santa à segunda-feira depois da Páscoa), `none`, ou um desses ou um visual do ano todo (`sunglasses`, `west-coast`, `fitted-cap`, `mountain-hat`, `headband`, `dreads`, `front-knot`, `durag`, `crown`, `bucket-hat`, `clock-chain`, `headphones`, `shutter-shades`, `chrome-chain`, `eye-patch`). Um desconhecido vira `auto` |
| `flock` | `"brazil"` | De onde vêm as aves das outras sessões: `brazil` (os urubus do Brasil), `americas` (com os dois condores) ou `world` (todos os urubus). O urubu-rei vem pelo papel de qualquer jeito |
| `presence` | `"island"` | A predefinição de presença: `island` (o bando no topo da tela), `panel` (o Zeca na bandeja do painel; a ilha abre perto do painel quando você clica nele ou quando um card precisa de você), `quiet` (nada em repouso; um card ainda abre a ilha com o seu som) ou `paused` (os cards vão na hora para os terminais dos agentes, os conectores param, sem notificações). Arquivos da versão 4 guardam `island` ou `panel` e são lidos como estão. Um valor desconhecido vira `island` |
| `zeca` | `true` | O Zeca, o companheiro: desligado, sem chat, microfone ou atalho de fala, e a bandeja fica sem *Chat…*; o bando, os cards, as notificações e os conectores funcionam como sempre |
| `widget` | ausente | O canto do widget de canto: `top-left`, `top-right`, `bottom-left` ou `bottom-right`; ausente (o padrão) para nenhum widget. Um valor desconhecido é nenhum widget |
| `projects` | ausente | Escolhas por pasta de projeto, definidas pelas ações rápidas de uma sessão ou em **Settings → Projects**: `{ "/home/me/site": { "pin": true }, "/home/me/x": { "mute": true, "hide": true } }`. `mute`: sem sons nem notificações das suas sessões em repouso (um card mantém os dois); `pin`: as suas sessões primeiro; `hide`: as suas sessões fora da ilha, da bandeja e do widget, exceto enquanto uma delas tem um card esperando. Só as escolhas ligadas são escritas, e um projeto sem nenhuma é removido (a partir da versão 8) |
| `dnd_until` | ausente | Não perturbe até esse momento, em segundos desde a época Unix: sem sons e sem notificações em repouso, sem lembretes; um card ainda abre a ilha com o seu som (e, no *Panel*, a sua notificação). Definido em **Settings → General**; um horário que já passou significa desligado (a partir da versão 9) |
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
