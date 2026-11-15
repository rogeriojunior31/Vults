# Desenvolvimento no Windows
<!-- source: ff604efcc3c5 -->

Compile e rode o Vults a partir de um checkout no Windows. O Linux tem a configuração própria em
[Primeiros passos](../getting-started.md#install).

Instale estes pré-requisitos juntos: Node.js 22 ou uma LTS compatível mais nova, Rust para
`x86_64-pc-windows-msvc`, Visual Studio com **Desenvolvimento para desktop com C++** (MSVC x64/x86,
SDK do Windows e ferramentas CMake/Ninja), LLVM com `libclang.dll` e Microsoft Edge WebView2 Runtime.
Defina `LIBCLANG_PATH` para a pasta `bin` do LLVM se ele estiver instalado em outro lugar. O
inicializador aceita Visual Studio Insiders; o Ninja evita seus problemas de descoberta de
instância no CMake.

Na raiz do repositório:

```bat
scripts\dev-windows.cmd --check
npm ci
scripts\dev-windows.cmd
```

A checagem informa as ferramentas ausentes antes de compilar. O inicializador muda apenas seu
próprio ambiente. Se uma compilação anterior guardou outro gerador CMake no cache, execute
`cargo clean -p whisper-rs-sys` uma vez.

O inicializador ativa o recurso de compilação `voice-vulkan` quando os cabeçalhos do Vulkan SDK,
`glslc` e `vulkan-1.lib` estão disponíveis por `VULKAN_SDK`, `C:\VulkanSDK` ou
`target/local-tools/vulkan`. Use `scripts\dev-windows.cmd --gpu` para exigir essa compilação,
ou `--cpu` como alternativa pela CPU. O SDK é uma dependência de compilação; a execução usa o
driver Vulkan do fabricante da GPU. O inicializador informa a compilação escolhida antes de abrir.
Compilações com GPU mapeiam temporariamente uma unidade livre de V a Z para seu cache local no
checkout, evitando o limite de caminhos do gerador de shaders do MSVC, e removem esse mapeamento
ao encerrar normalmente o inicializador. Um `CARGO_TARGET_DIR` explícito mantém seu dono e caminho existentes.
A primeira compilação com GPU pode levar vários minutos; isso é separado da latência de transcrição.

Compile o instalador do Windows com `npm run bundle:windows -- --bundles nsis`. Esse comando sempre
ativa Vulkan; a ausência do SDK nunca produz silenciosamente um instalador exclusivo de CPU. Ele
prepara ferramentas do SDK e um loader Vulkan x64 com versões fixas e SHA-256 verificado, sem executar
seus instaladores nem alterar drivers do sistema. O loader e sua licença acompanham o Vults, portanto
os usuários finais não precisam do SDK. O app usa transcrição pela CPU quando não há dispositivo
Vulkan compatível. O empacotamento extrai o conteúdo do NSIS, confere app/hook/loader/licença com a
compilação e verifica o executável empacotado normalmente e com a descoberta de drivers desativada.
A verificação (`vults.exe --voice-backend-check`) termina antes de abrir perfis, janelas ou microfones.
O instalador verificado é copiado para `target/release/bundle/nsis`, inclusive quando o cache está
em outra unidade. O CI de releases usa o mesmo comando e registra o relatório de verificação.
