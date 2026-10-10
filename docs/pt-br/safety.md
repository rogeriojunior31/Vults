# Segurança
<!-- source: 049d6cc375cf -->

O que o Vults promete:

- **Ele nunca bloqueia seu agente.** Se o app estiver fechado, lento ou quebrado, o hook sai em
  milissegundos sem imprimir nada, e seu agente pergunta no terminal como sempre.
- **Só um clique aprova uma chamada de ferramenta.** Nenhum timer ou padrão responde a um pedido de permissão; a
  única exceção é uma regra que você mesmo criou com **Always**, para aquele comando ou arquivo exato.
  Uma pergunta do Claude Code só é respondida com a escolha ou as palavras que você deu no card dela.
- **Só a sua conta de usuário fala com ele.** O socket fica na sua pasta de runtime privada, com modo
  `0600`, e os dois lados verificam se o outro processo roda como você (no Windows, pelo SID).
- **Ele nunca edita a configuração de um agente pelas suas costas.** Toda mudança ganha um backup datado e um diff que você
  aprova; os hooks de outras ferramentas são mantidos, e uma status line do Claude Code que seja sua é guardada ao lado do hook, continua rodando e volta como estava quando você remove a nossa; o `trusted_hash` do Codex nunca é escrito (você mesmo confia nos nossos hooks, no `/hooks` do Codex).
- **O uso do plano vem só das CLIs.** A entrada da status line do Claude Code é reduzida no hook
  aos `rate_limits` dela antes de qualquer coisa sair dele; o Codex é consultado com uma chamada só de leitura.
- **O chat usa as CLIs em que você fez login** (`claude`, `codex`); o Vults nunca lê as
  credenciais delas. No chat, todo comando e toda edição esperam o seu Allow; um card que ninguém
  responde é um não. Arquivos que você solta nele são copiados para uma caixa de entrada e apagados depois de uma semana.
- **As chaves de API ficam no chaveiro do sistema.** Se você der ao chat a chave de API de um provedor, ela é gravada
  só no chaveiro (Secret Service no Linux), nunca em um arquivo ou log, nunca devolvida a uma
  janela, e só é enviada para aquele provedor. Modelos locais (Ollama, LM Studio) são acessados só em
  `127.0.0.1`. O chat pela API não tem ferramentas: ele não executa comandos nem edita arquivos.
- **O microfone só escuta enquanto você segura uma gravação.** A voz fica desligada até você baixar um
  modelo; depois o mic grava só na memória enquanto o botão dele está vermelho (no máximo um minuto), e
  o whisper.cpp transcreve neste computador. O áudio nunca é salvo nem enviado para lugar nenhum.
- **O Now playing fica desligado até você ligar.** Aí o app lê seus players de mídia pelo
  session bus (MPRIS no Linux) para mostrar a música; isso nunca é guardado nem enviado para lugar nenhum.
- **As edições que ele mostra ficam na memória.** Para o diff da ilha, o hook repassa as linhas
  alteradas de uma edição concluída (no máximo 400); o app guarda só as dos últimos passos da sessão, e nunca
  as grava em um arquivo ou log.
- **Os conectores guardam só a última resposta.** A última verificação do GitHub (títulos, links, estados de checks e
  reviews dos seus pull requests abertos e repositórios recentes) é salva na pasta de dados do app,
  `connectors/github.json`, para que um reinício não repita notícias antigas. Nada mais é guardado, e isso
  nunca é enviado para lugar nenhum.
- **O histórico de atividade é só contagem, neste computador.** Para Settings → Activity, cada turno
  terminado de um agente fica na pasta de dados (`history.jsonl`, 12 semanas; `days.json`, os totais
  de cada dia por um ano): quando terminou, quanto durou, o agente, o nome da pasta do projeto e
  contagens (passos, comandos, arquivos e linhas mudados, as suas respostas). Nunca um prompt, uma
  resposta, um comando, um nome de arquivo ou um caminho. Nunca é enviado para lugar nenhum; desligue
  ou limpe em Settings → Activity.
- **Os segredos ficam no chaveiro do sistema**, e não há telemetria.
