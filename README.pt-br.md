<!-- source: eac4fb0674d4 -->
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
  <img src="docs/assets/island-flock.png" width="500" alt="A ilha aberta em repouso: vults de espécies diferentes num fio, cada um com o nome do projeto, e um chip para cada sessão que pede um olhar">
</p>

O Vults é para quem roda vários agentes de código ao mesmo tempo. Cada sessão do **Claude Code,
Codex, Antigravity, OpenCode, Qwen Code, Copilot CLI, Cursor, Factory Droid e Kimi Code** vira um urubu 8-bit no topo da tela, fazendo
o que a sessão faz. Você vê quem está trabalhando, quem terminou e quem precisa de você, e responde
de um lugar só. O **Zeca**, um urubu-de-cabeça-preta, fica por cima como um companheiro com quem
você conversa; ele é opcional.

> **0.1.7, no começo.** Linux primeiro (KDE Plasma e outros compositores com layer-shell); Windows
> e macOS depois. Instale o `.deb` ou o `.rpm` da
> [última versão](https://github.com/rogeriojunior31/Vults/releases/latest), o pacote do Arch com
> `makepkg`, ou compile do código: [Primeiros passos](docs/pt-br/getting-started.md).

## Uma superfície, de uma pílula à tela inteira

O seletor de tamanho no topo muda quanto você vê, no mesmo lugar:

- **Mini**: uma pílula fina com o bando no fio. Um selo colorido marca quem terminou, falhou ou
  espera por você, e a borda brilha em âmbar enquanto alguém espera.
- **Quick**: a ilha aberta. Em repouso, cada sessão fica no fio com o nome dela, e um chip marca
  cada uma que pede um olhar. Clique num urubu para abrir a sessão; **‹ Flock** volta.
- **Room** e **tela cheia**: o Nest, a janela do próprio Vults, no lugar da ilha.

<p align="center">
  <img src="docs/assets/island-compact-flock.png" width="440" alt="Mini: uma pílula fina com cinco vults no fio e os selos deles">
</p>
<p align="center">
  <img src="docs/assets/island-quick.png" width="880" alt="Quick: o bando em repouso com o nome de cada sessão e os chips, e uma sessão aberta a partir dele com Flock para voltar">
</p>

O Quick tem cinco abas: **Flock**, **Chat** (o Zeca, com um clipe para arquivos e o microfone),
**Activity** (a sua semana), **Connections** (os pull requests e checks do GitHub) e a engrenagem
(as configurações da ilha, e o resto das Settings no Nest).

<p align="center">
  <img src="docs/assets/island-tabs.png" width="880" alt="As outras abas do Quick: o chat, a atividade da semana, as linhas do GitHub em Connections e as configurações da engrenagem">
</p>

O **Room** abre o Nest no lugar da ilha: o bando pela janela sobre um card por sessão, e a escolhida
embaixo com as mudanças, a atividade e o contexto dela. Puxe as bordas para o tamanho que quiser.

<p align="center">
  <img src="docs/assets/nest-room.png" width="880" alt="Room: o Flock do Nest, quatro vults sobre os cards das sessões, e o diff da sessão escolhida embaixo">
</p>

## O bando

- **Um urubu por sessão.** Ele lê, bica o fio enquanto edita, puxa o fio enquanto um comando roda e
  abre as asas quando algo espera por você. Cada projeto mantém a sua espécie.
- **Aprovações num lugar só.** O card mostra o comando, o arquivo ou o diff inteiro: **Allow**,
  **Deny**, ou o teclado de qualquer janela. Nada é aprovado sem você: além do seu clique, só
  respondem um comando exato que você liberou com **Always**, uma política que você escreveu e
  aceitou, ou o seu telefone pareado (uma ação destrutiva sempre pergunta).
- **Ir para o terminal.** Um clique foca o painel do tmux, kitty, wezterm ou herdr da sessão, ou
  traz a janela dela para a frente.
- **Em volta:** diffs ao vivo, checks e revisões do GitHub pelo seu `gh`, o uso dos planos do Claude
  e do Codex, um resumo semanal do seu trabalho com os agentes e, em prévia, responder cards de um
  telefone pareado.

<p align="center">
  <img src="docs/assets/island-approval.png" width="500" alt="Um card de permissão: o comando inteiro, Deny, Allow e Always allow">
</p>

## Zeca

- **Chat** pelo `claude` ou `codex` em que você já entrou, com a sua própria configuração (todo
  comando e edição ainda pergunta antes), ou com uma chave de API, ou um modelo local. Solte
  arquivos nele, ou segure **Ctrl+Alt+V** e fale: o áudio é transcrito no seu computador.
- **O Nest**, a janela do próprio Vults (abre também com o Zeca desligado): a conversa em Markdown completo, as suas conversas salvas e (na próxima
  versão) a memória dele e os seus projetos com o branch, os pull requests e os checks.
- **Memória** (na próxima versão), desligada até você ligar: um diário local dos seus dias, a lembrança de dias
  anteriores antes de cada mensagem, e fatos que ele propõe e só guarda com o seu sim. Pelo Claude
  Code ele ganha as mesmas ferramentas do próprio servidor MCP dele.

<p align="center">
  <img src="docs/assets/island-chat-permission.png" width="500" alt="O chat: o Zeca pergunta antes de rodar um comando">
</p>

## Seguro por design

- **Seus agentes nunca esperam pelo Vults.** Se o app estiver fechado ou lento, o hook sai na hora e
  o agente pergunta no terminal dele.
- **Só você responde.** Um card é respondido pelo seu clique, por uma regra **Always** exata que
  você criou, por uma política que você viu como diff e aceitou, ou pelo seu telefone pareado com a
  biometria dele; no chat do Zeca, também pelo acesso que você escolheu para os cards dele. Uma ação
  destrutiva sempre pergunta, e o Zeca nunca responde um card.
- **Seus arquivos, com backup e diff.** A configuração de um agente, ou o `CLAUDE.md` de um
  projeto, só muda depois de um backup datado e de um diff que você aprova; os hooks de outras
  ferramentas são mantidos.
- **Nada sai sem o seu interruptor.** Sem telemetria. Toda requisição que o app faz por conta
  própria (os checks do GitHub, a web, os resumos diários, o relay do telefone) fica desligada até
  você ligar (fora o download único do detector de fala), tem um limite diário e um rastro local. Segredos ficam só no chaveiro do sistema. Veja
  [Segurança](docs/pt-br/safety.md).

## Documentação

Online em [rogeriojunior31.github.io/docs/vults](https://rogeriojunior31.github.io/docs/vults/),
atualizada a cada versão, ou aqui: [Primeiros passos](docs/pt-br/getting-started.md) ·
[A ilha](docs/pt-br/guide/island.md) · [O Nest](docs/pt-br/guide/nest.md) · [Aprovações](docs/pt-br/guide/approvals.md) ·
[Chat](docs/pt-br/guide/chat.md) · [A memória do Zeca](docs/pt-br/guide/memory.md) ·
[Conectores](docs/pt-br/guide/connectors.md) · [Atividade](docs/pt-br/guide/activity.md) ·
[Outros agentes](docs/pt-br/guide/other-agents.md) · [O telefone](docs/pt-br/guide/phone.md) ·
[Configurações e arquivos](docs/pt-br/reference/settings.md) ·
[Arquitetura](docs/pt-br/architecture.md) · [Decisões](docs/pt-br/adr/README.md).

**O que vem a seguir:** a 0.1.9 refina as páginas e o tema; a 0.2.0 traz os novos fluxos do Flock e do Chat.
Veja as etapas no [Plano Zeca](docs/dev/plan-zeca.md) e as mudanças publicadas no [changelog](CHANGELOG.md).

## Contribuindo

Relatos de bug, agentes, conectores e correções na documentação são bem-vindos: veja o
[CONTRIBUTING.md](CONTRIBUTING.md) e o [Código de Conduta](CODE_OF_CONDUCT.md). Problemas de
segurança vão em particular, como no [SECURITY.md](SECURITY.md). Feito com Rust e Tauri; não
confundir com o *Vulture*, o buscador de código morto em Python.

## Licença

MIT, veja a [LICENSE](LICENSE). Atribuições de terceiros no [NOTICE](NOTICE).
