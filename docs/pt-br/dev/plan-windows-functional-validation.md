# Validação funcional do Windows
<!-- source: 6727022a97fa -->

Prioridade alterada pelo usuário em 2026-11-11: estabilizar o comportamento existente no Windows
antes de adicionar conectores ou outros recursos. Notion e a expansão dos conectores ficam adiados.
O trabalho de leituras ao vivo do GitLab está preservado em sua branch/stash e não faz parte destas
correções. Manter comportamento e CI do Linux. Entregar PRs isoladas, testadas e limpar seus caches de build.

<a id="acceptance-matrix"></a>
## Matriz de aceitação

| Área | Evidência necessária | Estado atual |
| --- | --- | --- |
| Inicialização e ciclo de vida | Build atual; instância única; processos próprios encerrados após timeout/saída; sessões de agentes fechadas removidas | Limpeza do diagnóstico corrigida com testes nativos; aceitação ampla de app/sessões pendente |
| Room e Settings | Engrenagem/navegação, voltar/fechar/reabrir, rascunhos preservados, controles acessíveis e sem interceptação indevida de cliques | Validação ampla no WebView nativo pendente |
| Escalas e posicionamento | 100%, 125%, 150%, 175%, 200%; janelas baixas e mudança de monitor; entrada física distinguida da emulação | Conectores têm verificações emuladas; aceitação ampla/DPI nativo pendente |
| Agentes e hooks | Descoberta de CLI, launchers com espaços, instalação/status, eventos de pipe do mesmo usuário, início/fim e sessões antigas | Simulações/CI existentes mantidos; rodada Windows atual pendente |
| Atalhos | Teclas existentes do Windows respeitadas, conflitos visíveis, personalizações/desativações persistentes, pressionar/soltar e cancelar | Verificações existentes mantidas; rodada Windows atual pendente |
| Ditado e comandos | Dispositivo/modelo prontos, latência inicial, transcrição/entrega/cancelamento, perda de foco e restauração do clipboard | Primeiro inventário e testes sintéticos; fala real validada separadamente |
| Áudio e capacidades da plataforma | Reprodução/sessões, abrir apps, arquivos/URLs, notificações; caminhos sem suporte identificados | Auditoria em andamento |
| Distribuição e persistência | Instalador recente com identificação do código/build, atualização/reinício, configurações/modelos preservados, caminhos com espaços/Unicode | Executável instalado com data de 2026-11-08; candidato identificado pelo código pendente |

<a id="evidence-rules"></a>
## Regras de evidência

Usar perfis isolados e processos próprios de teste. Não sobrescrever configurações dos agentes,
credenciais, preferências de atalhos nem modelos de voz do usuário em uma validação. Simulações de
API/CLI/áudio e escalas emuladas não comprovam contas reais, microfone, DPI/entrada físicos nem
aceitação no desktop. Relatar falhas concretas e lacunas; não declarar Windows 100% funcional apenas
porque compilação ou testes sintéticos passaram.

<a id="first-investigation"></a>
## Primeira investigação

O diagnóstico dos agentes limita consultas de versão a três segundos. No Unix, encerra o grupo de
processos após timeout; no Windows, antes encerrava apenas o processo inicial. Uma simulação privada
com `.cmd` reproduziu um worker continuando após o timeout: o contador avançou de 146 para 156 após
o retorno. Agora o diagnóstico começa suspenso, entra em um Job anônimo não herdável que encerra
os processos ao fechar e só então retoma a thread de proprietário verificado. Fechar o Job encerra
os descendentes próprios após sucesso, erro e timeout; falhas na configuração encerram o processo
ainda suspenso. O comportamento do Unix não mudou.

Os testes nativos do Windows cobrem stdout herdado, workers sem stdout, consultas com erro/sucesso,
launchers npm normais e caminhos com espaços. Passaram localmente 1.537 testes de workspace/evals,
24 verificações da interface, fmt, clippy de workspace/todos os alvos sem avisos, checks estáticos e
cargo-deny. Esses testes de processos não comprovam desktop físico, fala real nem aceitação completa do Windows.

<a id="hook-commands-per-agent-shell-2026-11-13"></a>
## Comandos dos hooks por shell do agente (2026-11-13)

Antigravity, Droid, Kimi, Qwen, Cursor e Copilot recebiam o comando do Git Bash
(`'C:/…/vults-hook.exe' --agent …`), e os shells deles no Windows leem essas aspas simples
literalmente. Um workspace de teste (`.agents/hooks.json`) mostrou o agy 1.5.8 executando
`cmd /s /c "<comando>"`, então as sessões dele nunca chegavam; o `|| exit 0` escondia a falha. Os
outros shells vêm do código ou da documentação deles: Droid `%ComSpec% /d /s /c`, Kimi
`%ComSpec% /c`, Qwen cmd (Git Bash sob MSYS), Cursor e Copilot PowerShell (a chave `powershell` do
Copilot). Um caminho simples agora é escrito sem aspas, com barras normais, igual para cmd,
PowerShell e Git Bash; os outros caminhos levam aspas conforme o shell. O teste nativo ignorado
`windows_hooks_run_in_each_agents_shell` passa em cmd, Git Bash, PowerShell 5.1 e pwsh. Só o
Antigravity rodou ao vivo; Cursor, Copilot, Droid, Kimi e Qwen não estavam instalados. O Qwen em
modo cmd ainda não executa um hook cujo caminho tem espaço.

<a id="console-windows-2026-11-13"></a>
## Janelas de console (2026-11-13)

O app de release é um processo GUI, então cada filho de console iniciado sem `CREATE_NO_WINDOW`
ganhava uma janela: com o Windows Terminal como terminal padrão, uma janela do Terminal que ficava
aberta enquanto o filho rodava. Abrir o 0.1.8 instalado mostrou uma aos 0,58 s, do
`gh api graphql` (a primeira consulta do conector do GitHub), e de novo a cada consulta; os `git`
de cada turno e o relay destacado do plugin do OpenCode faziam o mesmo. Com o `gh`, o git de
`folder.rs` e o `windowsHide` do plugin corrigidos, o build de release da branch iniciou o `gh` aos
0,6 s e não abriu nenhuma janela de console em 20 s.

## `/continue` (2026-11-13)

O `/continue` recusava toda sessão do Windows ("Vults can't tell whether that session is still
open"): a checagem do processo lia `/proc` e era desconhecida nos outros sistemas, enquanto os hooks
do Windows sempre mandam um PID. Agora usa a sonda da detecção de presença (processo do mesmo
usuário, encerrado ou não), no Linux também.

<a id="hook-update-while-it-runs-2026-11-13"></a>
## Atualização do hook enquanto ele roda (2026-11-13)

O Windows nega substituir um executável em execução, e a status line de um agente roda o hook a
cada poucos segundos, então uma atualização do app podia deixar os agentes no hook antigo, só com
uma linha no log. Um hook em execução ainda pode ser renomeado: a atualização o move para o lado
como `vults-hook.old`, coloca o novo no lugar e apaga a cópia antiga assim que nada a executa,
na hora ou na próxima abertura. Um teste nativo mantém uma cópia rodando, mostra o rename simples
falhando e a substituição dando certo.
