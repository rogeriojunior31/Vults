# 0023. As Configurações, um webview da janela do Nest
<!-- source: 93fddc2639ab -->

**Status:** Aceita, 2026-10-12. Substitui a [0022](0022-settings-stays-a-window.md) e emenda a
[0021](0021-four-sizes-one-nest.md).

<a id="context"></a>
## Contexto

A [0022](0022-settings-stays-a-window.md) manteve as Configurações numa janela própria. Só as
Configurações podem gravar a config de um agente, uma chave, uma política, um celular ou um pedido
em nome do usuário, e essa confiança se sustenta porque elas não mostram texto de agente. O usuário
pediu uma janela só. A confiança nunca precisou de uma janela separada: precisa de um webview
separado. Um webview é uma página própria, com o seu label, os seus scripts e a sua memória. Uma
página num webview não roda script nem clica em outro; os dois dividem o armazenamento da origem,
que as Configurações não leem.

<a id="decision"></a>
## Decisão

- A janela do Nest (`nest-window`) tem dois webviews: o do Nest (`nest`) e o das Configurações
  (`settings`). Um aparece por vez, os dois sob o mesmo cabeçalho (abas, seletor de tamanho,
  engrenagem). A engrenagem mostra as Configurações; uma aba mostra o Nest nela.
  `open_settings(section)` abre a janela nas Configurações, numa página quando pedida.
- Os comandos conferem o label do webview que chamou, nunca o da janela. As chamadas exclusivas das
  Configurações da 0022 (`settings_page`, `policies::from_settings`) continuam recusadas vindas
  do webview do Nest, que divide a janela e mostra texto de agente.
- O webview das Configurações continua sem texto de agente. A moldura que ele desenha tem nomes
  de abas e contagens, o seletor de tamanho, o som e a tela cheia, e um rodapé que liga o Zeca e
  traz o terminal da sessão da frente: nada escrito por um agente. Ele guarda só quem é essa sessão.
- Os títulos de página da 0022 ficam.

<a id="consequences"></a>
## Consequências

- Uma janela para o Nest e as Configurações, como o usuário pediu; a ilha e o widget continuam
  superfícies próprias.
- Webviews filhos precisam da feature `unstable` do Tauri. No Linux, os webviews de uma janela
  dividem a caixa dela: o escondido não ocupa espaço, então eles são trocados, nunca sobrepostos.
- Abrir a janela nas Configurações carrega também a página do Nest, escondida; o habitat dela
  descansa enquanto está fora de vista.
- Um evento que o app envia chega aos listeners de todos os webviews, como já chegava entre
  janelas; uma página continua desenhando só o que pediu.
- Uma página futura nessa janela escolhe o webview pelo que mostra: texto de agente vai no do Nest,
  poderes vão no das Configurações.
