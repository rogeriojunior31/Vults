# Conectores
<!-- source: 11e618f4e60f -->

Os conectores trazem novidades de serviços externos para a ilha. Cada um fica desligado até você
ligá-lo em **Settings → Connectors**.

## GitHub

Ele verifica a cada cinco minutos, e a cada minuto enquanto houver checks de CI rodando:

- seus pull requests abertos: checks que falharam ou passaram, aprovado, mudanças solicitadas;
- pull requests em que sua revisão foi solicitada;
- os checks na branch padrão dos dez repositórios seus (de que você é dono) com push mais recente,
  e a release mais recente de cada um.

A mesma consulta única também traz o que a aba **Connections** do Room mostra de cada linha: o branch
de um pull request, o tamanho, os revisores e a última atualização, e cada check pelo nome
([O Nest](nest.md#connections)). As releases só aparecem lá: nunca viram alerta, nunca vão para a ilha.

Ele usa o GitHub CLI em que você já está logado (`gh auth login`), então o Vults nunca vê um token.
A primeira verificação só aprende como as coisas estão; os alertas começam na próxima mudança.
Clique em um alerta para abri-lo no GitHub, ou no × para dispensá-lo. Checks que passam substituem
uma falha, e uma decisão de revisão mais nova substitui a mais antiga.

Depois de um erro ele tenta de novo em 15 minutos se for um limite de requisições do GitHub, em 10
quando o `gh` não está instalado ou está deslogado, e de 4 a 15 (dobrando) nos outros casos. Abrir a
ilha ou o card verifica antes quando a última verificação tem mais de um minuto, exceto sob um
limite de requisições. O GitHub é verificado no máximo 3.000 vezes por dia; passado isso,
**Settings → Connectors** mostra **Budget spent**. Um repositório sobre o qual o GitHub não consegue
responder (por exemplo, um que exige login via SAML) não impede o resto de aparecer.

<a id="the-github-card"></a>

### O card do GitHub

Depois que o GitHub responde, a aba **Connections** da ilha aberta mostra o card dele, com um chip para o GitHub e um card por linha:

![O card do GitHub: seus pull requests, uma revisão esperando, as branches padrão e um check que falhou embaixo](../../assets/island-github-card.png)

- **Your pull requests** (seus pull requests), do atualizado mais recentemente para o mais antigo:
  um ponto para os checks (verde passando, âmbar rodando, vermelho falhando) e a revisão
  (*Approved* ou *Changes requested*);
- **Waiting for your review** (esperando sua revisão);
- **Default branches** (branches padrão): o último commit em cada uma, com o resultado dos checks.
  Repositórios sem checks ficam de fora.

Clique em uma linha para abri-la no GitHub. Quando um pull request é mergeado ou fechado, ou uma
solicitação de revisão é retirada, os alertas dele vão junto.

Com o Zeca ligado, uma linha pode perguntar a ele sobre ela:

- **Review with Zeca** (revisar com o Zeca), numa revisão que espera por você: ele lê o diff e diz
  os riscos antes de você aprovar;
- **Ask Zeca why** (perguntar ao Zeca por quê), em checks falhando: ele lê só o log da etapa que
  falhou e diz o que quebrou e onde.

Cada um abre o chat com a pergunta escrita no campo de texto. Nada é perguntado até você enviar,
então dá para mudá-la antes.

**Squash and merge** aparece no seu próprio pull request quando ele está aprovado e com os checks
passando. O primeiro clique só pergunta, embaixo da linha, o que o segundo vai fazer; **Merge** então
faz o squash na branch base e apaga a branch dele, pelo seu login do `gh`, e o card pergunta ao GitHub de
novo, então ela sai com a próxima resposta. Só o commit que o card mostrou verde é mergeado: se
alguém deu push depois, o GitHub recusa. Se o GitHub recusar (uma regra de branch, um conflito), a linha diz por quê.
Cada merge escreve uma linha no rastro local, e nada é mergeado por conta própria.

**✓** marca uma linha como vista: ela sai da ilha, e da contagem da aba, até mudar (os checks, a revisão
ou o último commit de uma branch), e continua nas Connections do Nest. **Mark all seen** (marcar todos como
vistos) limpa o card de uma vez. Quais linhas você marcou fica guardado só neste computador; o
GitHub nunca fica sabendo.

Quando uma verificação falha, o card mantém o que viu por último e mostra quando foi atualizado pela
última vez, e o erro. Enquanto um card de permissão espera, a aba Connections fica esmaecida até ele ser
respondido.

O que a última verificação viu (os títulos, links e estados de checks que o card mostra, e nada
mais) fica guardado em `connectors/github.json` na pasta de dados do app (`~/.local/share/vults/`
no Linux), para que uma reinicialização não gere alertas de novo sobre novidades antigas.

Quer outro serviço? Veja [Como adicionar um conector](../contributing/connectors.md).
