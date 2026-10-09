# Conectores
<!-- source: fe518f54f5b0 -->

Os conectores trazem novidades de serviços externos para a ilha. Cada um fica desligado até você
ligá-lo em **Settings → Connectors**.

## GitHub

Ele verifica a cada cinco minutos, e a cada minuto enquanto houver checks de CI rodando:

- seus pull requests abertos: checks que falharam ou passaram, aprovado, mudanças solicitadas. Um
  push cujos checks já terminaram quando a próxima verificação roda ainda recebe seu alerta;
- pull requests em que sua revisão foi solicitada;
- os checks na branch padrão dos seus dez repositórios com push mais recente.

Ele usa o GitHub CLI em que você já está logado (`gh auth login`), então o Vults nunca vê um token.
Abrir a ilha verifica de novo na hora quando a última verificação tem mais de um minuto, então ele
também tenta de novo logo depois de um erro que você corrigiu (por exemplo, depois de um
`gh auth login`); só um limite de requisições do GitHub é sempre respeitado até o fim. A primeira
verificação só aprende como as coisas estão; os alertas começam na próxima mudança. Clique em um
alerta para abri-lo no GitHub, ou no × para dispensá-lo.

Um alerta só fica enquanto for a palavra mais recente sobre aquela história: checks que passam
substituem a falha no mesmo pull request ou branch padrão, e uma decisão de revisão mais nova
substitui a mais antiga. Uma revisão solicitada de novo depois de ter sido retirada gera alerta de
novo. Quando o GitHub não consegue responder sobre uma organização ou repositório (por exemplo, um
que exige login via SAML), o resto continua aparecendo.

<a id="the-github-card"></a>

### O card do GitHub

Depois que o GitHub responde, a ilha aberta ganha uma aba GitHub ao lado de Flock, Chat e Drop. Ela
mostra o que está aberto agora:

![O card do GitHub: seus pull requests, uma revisão esperando, as branches padrão e um check que falhou embaixo](../../assets/island-github-card.png)

- **Your pull requests** (seus pull requests), do atualizado mais recentemente para o mais antigo:
  um ponto para os checks (verde passando, âmbar rodando, vermelho falhando, nenhum se não houver
  checks) e a revisão no final (*Approved* ou *Changes requested*);
- **Waiting for your review** (esperando sua revisão): os pull requests em que sua revisão foi
  solicitada;
- **Default branches** (branches padrão), do push mais recente para o mais antigo: o último commit
  na branch padrão dos seus repositórios recentes, com o resultado dos checks. Repositórios sem
  checks ficam de fora.

Clique em uma linha para abri-la no GitHub. Abrir o card verifica de novo quando a última
verificação tem mais de um minuto, como abrir a ilha faz. Quando um pull request sai do card
(mergeado ou fechado), ou uma solicitação de revisão é retirada, os alertas dele vão junto.
Desligar o GitHub tira o card.

Quando uma verificação falha (GitHub fora do ar, `gh` deslogado), o card mantém o que viu por
último e avisa ao lado do nome: quando foi atualizado pela última vez, e o erro. Enquanto um card
de permissão espera, a aba GitHub fica esmaecida: a permissão vem primeiro, e a aba volta a
funcionar assim que ela for respondida.

O que a última verificação viu fica guardado em disco, em `connectors/github.json` na pasta de
dados do app (`~/.local/share/vults/` no Linux), para que uma reinicialização não gere alertas de
novo sobre novidades antigas. O arquivo guarda os títulos, links e estados de checks que o card
mostra, e nada mais.

Quer outro serviço? Veja [Como adicionar um conector](../contributing/connectors.md).
