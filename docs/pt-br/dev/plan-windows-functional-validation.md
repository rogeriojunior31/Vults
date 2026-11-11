# Validação funcional do Windows
<!-- source: f5bec764368c -->

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
