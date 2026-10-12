# 0019. Rede, segredos e telemetria
<!-- source: bcdc3fdfa170 -->

**Status:** Aceita, 2026-10-10. Consolidada em 2026-10-12 a partir da 0006 e da 0016.

<a id="context"></a>

## Contexto

O app vê comandos, nomes de arquivo, diffs e chaves de API, ao lado de trabalho privado. Alguns
recursos ainda precisam da rede enquanto o usuário não está pedindo naquele momento: conectores,
busca e fetch na web durante um turno do chat, o sono do Zeca, rotinas, o relay e o push do
celular, respostas faladas, a checagem de atualização. Cada um manda para fora da máquina algo que
importa ao usuário, custa dinheiro ou cota, e poderia virar telemetria por acidente.

<a id="decision"></a>

## Decisão

- **Segredos só no chaveiro do sistema**, nunca num arquivo ou num log.
- **Nunca telemetria.** O app não manda nada sobre o próprio uso; nenhuma requisição leva uso, um
  id ou qualquer coisa sobre o usuário além do que o recurso precisa para fazer o seu trabalho.
- **Uma requisição que o usuário inicia** com um clique ou uma mensagem (um turno do chat, um chat
  por API) é escolha explícita dele, dita onde ela é ligada.
- **Uma requisição em nome do usuário** (uma que sai da máquina sem um clique naquele momento,
  inclusive uma busca ou fetch na web que o modelo faz durante um turno que o usuário começou) é
  um recurso com:
  - um **interruptor** próprio no Settings, desligado por padrão; ligá-lo diz o que manda, para
    onde, e quanto custa;
  - um **orçamento** (requisições, tokens ou dinheiro por dia), e ele para quando acaba;
  - um **registro local** que o usuário pode ler (o quê, para onde, quando, custo), mandado para
    nenhum outro lugar;
  - **um único portão de rede, do `app`** (`app/src/gate.rs`), que confere o interruptor e o
    orçamento. Nenhum crate abre uma conexão para um recurso desses por fora dele.
- Os conectores (o GitHub pelo `gh`, hoje) são recursos assim: cada um fica desligado até o usuário
  conectá-lo, e o polling dele passa pelo portão.
- **As atualizações são pedidas e assinadas**:
  - A checagem fica desligada até o usuário ligá-la (no Settings, e oferecida uma vez na primeira
    execução). Ligada, ela busca um manifesto estático do GitHub Releases no máximo uma vez por dia
    e não manda nada além da requisição: nenhum id, nenhum uso, nenhuma versão numa query string.
  - Uma atualização só é instalada depois de um clique que mostra a versão e as notas dela. Um
    agente rodando nunca é interrompido: o reinício espera até o usuário escolher.
  - Três canais, cada um com a própria URL de manifesto: **stable** (o padrão), **beta** (release
    candidates) e **nightly** (commits selecionados). Um build sabe o seu canal em tempo de
    compilação, então um build stable nunca lê o manifesto nightly; trocar de canal é reinstalar.
  - Toda atualização é assinada. A chave privada vive só nos segredos do job de release; a chave
    pública vem embutida no app. Um manifesto ou pacote que falha na verificação é ignorado e
    registrado no log.
  - Pacotes que uma distribuição atualiza (AUR, um futuro Flatpak) saem com o atualizador fora da
    compilação.

<a id="consequences"></a>

## Consequências

- Um teste roda um dia com todos os recursos desligados e conta zero requisições.
- Nenhum número de uso guia o roadmap: aprendemos com as issues e usando o app nós mesmos. Tudo o
  que aprenderia com os dados dos usuários tem de aprender na máquina do usuário, ou não aprender.
- O atualizador chega depois da 0.2.0 ([0020](0020-0.2.0-at-wave-4-then-0.2.x.md)). O
  `release.yml` vai assinar os pacotes e publicar um manifesto por canal; a chave é criada uma vez e
  guardada offline (perdê-la significa que os usuários reinstalam à mão). Os builds nightly e beta
  ganham um sufixo próprio no bundle id, para instalar ao lado do stable sem dividir configurações
  nem o chaveiro.

<a id="history"></a>

## Histórico

- [0006](0006-keyring-no-telemetry.md), 2026-10-04: segredos só no chaveiro; sem telemetria; sair
  da máquina é escolha do usuário.
- [0016](0016-updates-asked-for-and-signed.md), 2026-10-08: a checagem de atualização, pedida,
  assinada, um canal por vez.
