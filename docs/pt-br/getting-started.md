# Primeiros passos
<!-- source: bb8a8f0d1507 -->

<a id="install"></a>

## Instalação

As releases trazem um `.deb` e um `.rpm` (Ubuntu 22.04, Debian 12, Fedora e mais novos) e um instalador
para Windows. Por enquanto não há AppImage. Baixe-os da
[release mais recente](https://github.com/rogeriojunior31/Vults/releases/latest) e confira um download
com `sha256sum -c SHA256SUMS --ignore-missing`. Para conferir que ele foi gerado pelo workflow de release
deste repositório, a partir da tag da release: `gh attestation verify <arquivo> --repo rogeriojunior31/Vults`.

No Arch Linux, gere o pacote com `makepkg`: `packaging/aur/vults` compila a release mais
recente, `packaging/aur/vults-git` acompanha a `main`. Os dois compilam do código-fonte, então a primeira instalação
demora um pouco. Eles ainda não estão no AUR.

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

O Linux precisa de `webkit2gtk-4.1`, `gtk3`, `gtk-layer-shell`, `libayatana-appindicator`, `openssl`, `alsa-lib`, `gst-plugins-good` (os sons da ilha:
o WebKitGTK toca eles pelo GStreamer)
e do loader do Vulkan (para compilar, também `cmake`, os headers do Vulkan e `glslc`). No KDE Plasma,
Hyprland, Sway e outros compositores com layer-shell, a ilha fica na borda de cima como um painel;
no GNOME ela é uma janela comum, sempre por cima.

<a id="connecting-your-agents"></a>

## Conectando seus agentes

Abra **Set up agents…** pelo ícone da bandeja e clique em **Install hooks…** ao lado de um agente. Você vê o
arquivo que vai mudar, o diff exato e um botão **Write the file**; antes é feito um backup datado, e os hooks de outras ferramentas são
mantidos. **Remove hooks…** tira só o que o Vults adicionou.

| Agente | Arquivo | Depois de instalar |
|---|---|---|
| Claude Code | `~/.claude/settings.json` | Nada: as novas sessões aparecem na ilha |
| Codex | `~/.codex/hooks.json` | Abra o Codex, digite `/hooks` e confie nos hooks do Vults. O Codex só executa um hook depois que você confia nele, e só o Codex registra essa confiança. A janela de configurações mostra quantos ainda estão esperando. Uma reinstalação que muda um hook pede essa confiança de novo |
| Gemini CLI | `~/.gemini/settings.json` | Nada: as novas sessões aparecem na ilha. O Gemini pede as permissões dele no próprio terminal (veja [Aprovando](guide/approvals.md)) |
| Antigravity | `~/.gemini/config/hooks.json` | Nada: a CLI agy, o app e a IDE aparecem na ilha, com o nome `antigravity`. O Antigravity pede as permissões ele mesmo (veja [Outros agentes](guide/other-agents.md#antigravity)) |
| OpenCode | `~/.config/opencode/plugins/vults.js`, um plugin nosso | Reinicie o OpenCode. As sessões dele aparecem na ilha, e as permissões e as perguntas dele aparecem lá como cards: responda na ilha ou no OpenCode, o que vier primeiro (veja [Aprovando](guide/approvals.md#opencode)) |
| Qwen Code | `~/.qwen/settings.json` | Reinicie o Qwen Code. As sessões dele aparecem na ilha, e as permissões dele aparecem lá como cards (veja [Aprovando](guide/approvals.md#qwen-code)) |

Outras ferramentas (Pi, Cursor…) também podem reportar, com algumas linhas na própria configuração delas: veja
[Outros agentes](guide/other-agents.md).

<a id="coming-from-vultures-ai"></a>

### Vindo do Vultures AI

O Vults se chamava Vultures AI até a 0.1.5. Na primeira vez que abre, ele move as pastas antigas
(`~/.config/vultures-ai`, `~/.local/share/vultures-ai`, …) para os nomes novos e as chaves de API para
a nova entrada do chaveiro do sistema, e deixa um link onde ficava o hook antigo, para que seus agentes continuem reportando.
Se ele abria no login, a entrada antiga de login é trocada pela nova. O `.deb` e o `.rpm`
substituem o pacote antigo `vultures-ai` em vez de instalar ao lado dele.
Abra **Set up agents…** e clique em **Update hooks…** ao lado de cada agente: as entradas antigas são
substituídas, não duplicadas. O Codex pede a confiança dele de novo.

<a id="language"></a>

## Idioma

O Vults fala inglês, português do Brasil, espanhol e chinês simplificado. Ele segue o idioma do seu
desktop; **Settings → General → Language** escolhe outro para a ilha, as configurações, a bandeja e
as notificações. O que os seus agentes dizem fica nas palavras deles.

<a id="next"></a>

## Próximos passos

- [A ilha](guide/island.md): o bando, recolhido e aberto, chegando a uma sessão
- [Aprovando pela ilha](guide/approvals.md)
- [Conversando com o Zeca](guide/chat.md)
- [Conectores](guide/connectors.md)
