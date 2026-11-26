# Primeiros passos
<!-- source: 522e3c8c4d69 -->

<a id="install"></a>

## Instalação

As releases trazem um `.deb` e um `.rpm` (Ubuntu 22.04, Debian 12, Fedora e mais novos) e um
instalador de prévia para Windows (reuniões, a conversa com o Zeca, digitar em terminais e alguns
outros recursos ainda são só do Linux). Por enquanto não há AppImage. Baixe-os da
[release mais recente](https://github.com/rogeriojunior31/Vults/releases/latest) e confira um download
com `sha256sum -c SHA256SUMS --ignore-missing`. Para conferir que ele foi gerado pelo workflow de release
deste repositório: `gh attestation verify <arquivo> --repo rogeriojunior31/Vults`.

No Arch Linux, gere o pacote com `makepkg`: `packaging/aur/vults` compila a release mais recente,
`packaging/aur/vults-git` acompanha a `main`. Os dois compilam do código-fonte, então a primeira
instalação demora um pouco. Eles ainda não estão no AUR.

```sh
git clone https://github.com/rogeriojunior31/Vults
cd Vults/packaging/aur/vults && makepkg -si
```

Ou compile do código-fonte:

```sh
git clone https://github.com/rogeriojunior31/Vults
cd Vults
npm install
npm run tauri dev        # executa
npm run bundle:linux     # ou gera o .deb e o .rpm em target/release/bundle/
```

O Linux precisa de `webkit2gtk-4.1`, `gtk3`, `gtk-layer-shell`, `libayatana-appindicator`, `openssl`,
`alsa-lib`, `libpulse`, `gst-plugins-good` (o WebKitGTK toca os sons da ilha pelo GStreamer) e do loader do
Vulkan (para compilar, também `cmake`, os headers do Vulkan e `glslc`). No KDE Plasma, Hyprland, Sway
e outros compositores com layer-shell, a ilha fica na borda de cima como um painel; no GNOME ela é
uma janela comum, sempre por cima. **Settings → General → Start with the desktop** abre o Vults
quando você entra na sessão.

<a id="on-windows"></a>

### No Windows

O instalador do Windows é uma prévia. Para compilar o Vults no Windows, veja
[Desenvolvimento no Windows](contributing/windows-development.md).

O instalador pergunta se o Vults é só para você ou para todos os usuários do computador.
**Instalar apenas pra mim** instala em `%LOCALAPPDATA%\Programs\Vults` e nunca precisa de
administrador; uma conta padrão sempre instala assim. **Instalar pra qualquer um usando este
computador** instala em `%ProgramFiles%\Vults` e precisa de administrador. Numa conta de
administrador, o Windows pede aprovação (UAC) quando o instalador abre, seja qual for a escolha. Nos
dois casos, cada usuário guarda suas configurações, hooks e histórico em `%LOCALAPPDATA%\Vults`, e o
Vults instala os hooks desse usuário na primeira vez que abre. Uma atualização mantém a escolha e a
pasta anteriores. Para uma instalação silenciosa, rode o instalador com `/S` e `/CURRENTUSER` ou
`/ALLUSERS` (este num prompt elevado).

Os atalhos do Windows usam teclas que não escrevem caracteres para preservar o AltGr:
**Ctrl+Alt+Space** abre a ilha; **Ctrl+Alt+Left/Right** muda a sessão; **Ctrl+Alt+F1/F2/F3** seleciona
Mini/Rápido/Sala; **Ctrl+Alt+F4** grava ditado; **Ctrl+Alt+F5** grava um comando de voz;
**Ctrl+Alt+F6** copia novamente o último ditado; **Ctrl+Alt+F8/F9** permite/nega o card visível;
**Ctrl+Alt+F10** cancela o ditado e a voz do Chat. Uma tecla já usada por outro aplicativo fica sem vínculo. As Configurações mostram apenas
os registros bem-sucedidos.

Altere as teclas do Windows em **Configurações → Geral → Atalhos de teclado** (também disponível na
seção de teclas do Ditado). Escolha os modificadores e a tecla de cada ação, ou **Desativado**, e depois
**Aplicar atalhos**. As alterações valem imediatamente e persistem entre reinícios. **Restaurar padrões**
prepara as combinações originais; aplique-as explicitamente. Um conflito ou falha ao salvar mantém os
vínculos ativos anteriores. Teclas Windows/de digitação, Escape, Delete e F12 ficam de fora. O sistema
detecta outros atalhos globais registrados; atalhos internos/locais de um aplicativo não podem ser
detectados. Se uma escolha interferir em um deles, desative-a ou escolha outra combinação aqui.
O Linux mantém seu editor de atalhos do desktop.

Reuniões, a conversa com o Zeca, perguntas de agentes por MCP, as ferramentas de GitHub dos
especialistas, digitar em terminais e na maioria dos apps, e abas de terminal no *Abrir terminal*
ainda são só do Linux.

<a id="connecting-your-agents"></a>

## Conectando seus agentes

Abra **Set up agents…** pelo ícone da bandeja e clique em **Install hooks…** ao lado de um agente.
Você vê o arquivo que vai mudar, o diff exato e um botão **Write the file**; antes é feito um backup
datado, e os hooks de outras ferramentas são mantidos. **Remove hooks…** tira só o que o Vults
adicionou.

| Agente | Arquivo | Depois de instalar |
|---|---|---|
| Claude Code | `~/.claude/settings.json` | Nada: as novas sessões aparecem na ilha |
| Codex | `~/.codex/hooks.json` | Abra o Codex, digite `/hooks` e confie nos hooks do Vults (o Codex só roda um hook quando ele é confiável e está ligado); as configurações mostram quantos ainda esperam. Uma reinstalação pede de novo |
| Antigravity | `~/.gemini/config/hooks.json` | Nada ([ele pergunta no próprio terminal](guide/other-agents.md#antigravity)) |
| OpenCode | `~/.config/opencode/plugins/vults.js`, um plugin nosso | Reinicie o OpenCode ([os cards dele](guide/approvals.md#opencode)) |
| Hermes Agent | `~/.hermes/plugins/vults/` no Linux; `%LOCALAPPDATA%\hermes\plugins\vults\` no Windows (`HERMES_HOME` substitui ambos) | Ative com `hermes plugins enable vults` e abra uma nova sessão ([detalhes](guide/other-agents.md#hermes-agent)) |
| Qwen Code | `~/.qwen/settings.json` | Reinicie o Qwen Code ([os cards dele](guide/approvals.md#qwen-code)) |
| Copilot CLI | `~/.copilot/hooks/vults.json`, um arquivo nosso | Reinicie o Copilot CLI ([os cards dele](guide/approvals.md#copilot-cli)) |
| Cursor | `~/.cursor/hooks.json` | Reinicie o Cursor ([ele pergunta no próprio prompt](guide/other-agents.md#cursor)) |
| Factory Droid | `~/.factory/settings.json` | Reinicie o Droid ([ele pergunta no próprio prompt](guide/other-agents.md#factory-droid)) |
| Kimi Code | `~/.kimi-code/config.toml` | Reinicie o Kimi ([ele pergunta no próprio prompt](guide/other-agents.md#kimi-code)) |

Com `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `QWEN_HOME`, `COPILOT_HOME`, `FACTORY_HOME_OVERRIDE`,
`KIMI_CODE_HOME` ou `XDG_CONFIG_HOME` definida, o arquivo nessa pasta é o usado
([detalhes](reference/settings.md#agent-configs-vults-edits)). O do Antigravity é sempre
`~/.gemini/config/hooks.json`: o `GEMINI_CLI_HOME` não o muda de lugar.

Outras ferramentas (Pi…) também podem reportar, com algumas linhas na própria configuração
delas: veja [Outros agentes](guide/other-agents.md).

<a id="coming-from-vultures-ai"></a>

### Vindo do Vultures AI

O Vults se chamava Vultures AI até a 0.1.5; na primeira vez que abre, ele move as pastas antigas, a
entrada do chaveiro e a de login para os nomes novos, e o `.deb` e o `.rpm` substituem o pacote
antigo. Depois clique em **Update hooks…** ao lado de cada agente em **Set up agents…** (o Codex pede
a confiança dele de novo).

<a id="keys"></a>

## Teclas

No Linux, de qualquer lugar: **Ctrl+Alt+Space** abre a ilha e entrega o teclado a ela,
**Ctrl+Alt+Y** / **Ctrl+Alt+N** permitem ou negam o card da frente, **Ctrl+Alt+J** / **Ctrl+Alt+K**
põem a próxima ou a sessão anterior na frente, e segurar **Ctrl+Alt+V** fala com o Zeca
([todas elas](guide/island.md#getting-to-a-session)).

<a id="language"></a>

## Idioma

O Vults fala inglês, português do Brasil, espanhol e chinês simplificado. Ele segue o idioma do seu
desktop; **Settings → General → Language** escolhe outro para a ilha, as configurações, a bandeja e
as notificações. O que os seus agentes dizem fica nas palavras deles.

<a id="next"></a>

## Próximos passos

- [A ilha](guide/island.md): o bando, recolhido e aberto, chegando a uma sessão
- [Aprovando pela ilha](guide/approvals.md)
- [Atividade](guide/activity.md)
- [Outros agentes](guide/other-agents.md)
- [O Nest](guide/nest.md): o bando inteiro, chat, registros e conexões
- [Conversando com o Zeca](guide/chat.md)
- [A memória do Zeca](guide/memory.md)
- [Conectores](guide/connectors.md)
- [O telefone](guide/phone.md)
