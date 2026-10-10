<!-- source: 24c094454597 -->
<p align="center">
  <img src="docs/assets/zeca.png" width="128" height="128" alt="Zeca, um urubu-de-cabeça-preta em 8 bits">
</p>

<h1 align="center">Vults</h1>

<p align="center">
  <strong>Seus agentes de código, vivos no desktop. E um urubu que ajuda.</strong>
</p>

<p align="center">
  <a href="https://github.com/rogeriojunior31/Vults/actions/workflows/ci.yml"><img src="https://github.com/rogeriojunior31/Vults/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/rogeriojunior31/Vults/releases/latest"><img src="https://img.shields.io/github/v/release/rogeriojunior31/Vults" alt="Última versão"></a>
  <a href="https://rogeriojunior31.github.io/docs/vults/"><img src="https://img.shields.io/badge/docs-ler%20online-f2984a" alt="Documentação"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="Licença MIT"></a>
  <img src="https://img.shields.io/badge/platform-Linux-informational" alt="Linux">
  <img src="https://img.shields.io/badge/built%20with-Rust%20%2B%20Tauri-orange" alt="Feito com Rust e Tauri">
</p>

<p align="center"><a href="README.md">English</a> · <b>Português</b></p>

<p align="center">
  <img src="docs/assets/island-flock.png" width="640" alt="A ilha aberta: um grifo-do-himalaia com a sessão em destaque, e quatro vults de espécies diferentes na lista do bando">
</p>

O Vults é um app de desktop para quem roda vários agentes de código ao mesmo tempo. Ele tem duas
partes: o bando é a ferramenta, e o Zeca é o companheiro que vive em cima dela.

- **O bando.** Cada sessão do Claude Code, Codex, Gemini CLI, Antigravity, OpenCode ou Qwen Code vira um urubu 8-bit no
  seu desktop, um vult, fazendo o que a sessão está fazendo. Você vê de relance quem está
  trabalhando, quem terminou, quem falhou e quem precisa de você; aprova, responde e pula para o
  terminal certo sem precisar procurar. O bando vive numa ilha no topo da tela, ou junto do
  painel; a bandeja, um widget de canto e as notificações do desktop deixam você escolher o
  quanto ele aparece.
- **Zeca, o companheiro.** Um urubu-de-cabeça-preta que conversa com você pelos CLIs que você já
  usa, escuta quando você segura uma tecla e fala, recebe os arquivos que você solta nele e, com o
  tempo, vira um agente pessoal que age por você, sempre perguntando antes de rodar qualquer
  coisa. Ele é opcional: um botão desliga ele e deixa só o bando.

> **Status: 0.1, no começo.** Linux primeiro (KDE Plasma e outros compositores com layer-shell);
> Windows e macOS depois. Baixe o `.deb` ou o `.rpm` da
> [última release](https://github.com/rogeriojunior31/Vults/releases/latest), gere o pacote do
> Arch com `makepkg` ou compile do código: veja [Primeiros passos](docs/pt-br/getting-started.md).

<p align="center">
  <img src="docs/assets/island-approval.png" width="640" alt="Um card de permissão: o comando inteiro, Deny, Allow e Always allow">
</p>

## O que ele faz

**O bando**

- **Um pássaro por sessão.** Ele lê, bica o fio enquanto edita, puxa o fio enquanto um comando
  roda, sai voando quando o agente vai para a web e abre as asas quando uma permissão espera por
  você.
- **Aprovações num lugar só.** O card mostra o comando, o arquivo ou o diff inteiro; **Allow**,
  **Deny** ou **Ctrl+Alt+Y** / **Ctrl+Alt+N** de qualquer janela. Nada é aprovado sem você, a não
  ser o que você mesmo permitiu com **Always**, para aquele comando exato naquele projeto.
- **Diffs ao vivo.** Cada edição mostra as linhas adicionadas e removidas; um clique mostra o diff.
- **Pule para o terminal.** Um clique foca o painel do tmux, kitty, wezterm ou herdr da sessão e,
  no KDE Plasma ou em qualquer desktop X11, traz a janela para a frente (um terminal, o VS Code ou o Cursor).
- **Novidades do GitHub.** Checks com falha, aprovações e pedidos de review, pelo `gh` que você já
  usa.
- **Consumo do plano de relance.** Quanto dos limites do Claude e do Codex você já gastou, lido dos
  CLIs.
- **A sua semana com os agentes.** Tempo, turnos, linhas e respostas por semana, e o ano numa
  grade, contados no seu computador (nunca um prompt ou um caminho).

**Zeca**

- **Chat.** Pelo CLI `claude` ou `codex` em que você está logado (todo comando e toda edição
  perguntam antes), por uma chave de API ou por um modelo local com Ollama ou LM Studio. Solte
  arquivos nele.
- **Voz.** Segure **Ctrl+Alt+V** e fale: o whisper.cpp transcreve no seu computador, e você lê o
  texto antes de enviar.
- **Tocando agora**, se você quiser: a música na ilha, e o bando dança junto.

<p align="center">
  <img src="docs/assets/island-live-diff.png" width="640" alt="O diff de uma edição concluída no card: +4 −2, com números de linha">
  <img src="docs/assets/island-chat-permission.png" width="640" alt="O chat: o Zeca pergunta antes de rodar um comando">
</p>

## Seguro por padrão

O hook nunca trava o seu agente: se o app estiver fechado ou lento, ele sai na hora e o agente
pergunta no próprio terminal. A configuração dos agentes só muda depois de um backup datado e de um
diff que você aprova, e os hooks de outras ferramentas são mantidos. Segredos ficam só no chaveiro
do sistema, e não há telemetria. Veja [Segurança](docs/pt-br/safety.md).

## Para onde ele vai

A primeira release é a **0.1.0**. Depois dela, releases pequenas (0.1.1, 0.1.2…) saem conforme
cada parte fica pronta, Linux primeiro, até a **0.2.0**:

- **Já entrou**: a bandeja, um widget de canto, notificações do desktop, modos de presença de
  *Island* a *Paused*, ações rápidas em cada pássaro, cards de espera que chamam mais alto com o
  tempo, um resumo do que aconteceu enquanto você estava fora e uma voz que sabe quando você parou
  de falar.
- **Controle**: uma paleta de comandos; um dicionário de voz para as palavras que você usa; o Zeca
  respondendo em voz alta por um modelo de voz em tempo real, se você ligar.
- **Plataforma**: histórico local, seus projetos com branch, pull request e checks, custo por
  sessão.
- **0.2.0, Operações**: o app completo, políticas que você escreve e aprova, iniciar agentes
  daqui.

O Zeca continua opcional o tempo todo: o bando funciona sem ele. Por que escolhemos isso, e o que
não vamos fazer, está nos [registros de decisão](docs/pt-br/adr/README.md).

## Documentação

Leia online em [rogeriojunior31.github.io/docs/vults](https://rogeriojunior31.github.io/docs/vults/),
atualizada a cada release, ou aqui:

- [Primeiros passos](docs/pt-br/getting-started.md)
- [A ilha](docs/pt-br/guide/island.md) · [Aprovações](docs/pt-br/guide/approvals.md) · [Chat](docs/pt-br/guide/chat.md) · [Conectores](docs/pt-br/guide/connectors.md) · [Atividade](docs/pt-br/guide/activity.md) · [Outros agentes](docs/pt-br/guide/other-agents.md)
- [Configurações e arquivos](docs/pt-br/reference/settings.md) · [Protocolo do hook](docs/pt-br/reference/protocol.md)
- [Arquitetura](docs/pt-br/architecture.md) · [Decisões](docs/pt-br/adr/README.md) · [Animações](docs/pt-br/ANIMATIONS.md) · [Como adicionar um conector](docs/pt-br/contributing/connectors.md)

Feito com Rust e Tauri. Não confundir com o *Vulture*, o detector de código morto em Python.

## Como contribuir

Relatos de bugs, agentes, conectores e correções nas docs são bem-vindos: veja o
[CONTRIBUTING.md](CONTRIBUTING.md) e o [Código de Conduta](CODE_OF_CONDUCT.md). Problemas de
segurança vão em particular, como diz o [SECURITY.md](SECURITY.md). O que mudou em cada release
está no [changelog](CHANGELOG.md). O código, os comentários e a interface do app ficam em inglês.

## Licença

MIT, veja [LICENSE](LICENSE). Atribuições de terceiros em [NOTICE](NOTICE).
