# O telefone
<!-- source: 2bb66a010393 -->

Um telefone que você pareia pode responder aos cards que esperam por você enquanto está longe da
mesa. Ele só decide: este computador continua fazendo o que você escolheu, e a ilha também mantém
todos os cards. O app de telefone do Vults ainda não saiu; até lá, o cliente de teste de linha de
comando de [O relay do telefone](../reference/relay.md) pareia e responde como um telefone.

<a id="turning-it-on"></a>

## Como ligar

Em **Settings → Phone**, ligue **Responder aos cards por um telefone** e escolha como o telefone
chega a este computador:

- **Por um relay**: um servidor pequeno que guarda mensagens seladas até o outro lado lê-las. Rode
  o seu ([O relay do telefone](../reference/relay.md)) e informe o endereço dele.
- **Direto (rede local ou tailnet)**: este computador roda o próprio relay num endereço que você
  informa, como o endereço dele na tailnet e uma porta (`100.64.0.2:8787`). O telefone precisa
  conseguir alcançá-lo.

Cada pedido ao relay é um pedido em seu nome: desligado até você ligar, no máximo 10.000 por dia,
cada um guardado no trace local ([Segurança](../safety.md)).

<a id="pairing"></a>

## Pareamento

Aperte **Parear**: um QR aparece por dois minutos. Escaneie com o telefone. O QR leva a chave
pública deste computador, o endereço do relay e um segredo de uso único; só um telefone que o viu
consegue parear, e ele pareia um telefone. O telefone então aparece em **Telefones pareados**.

<a id="answering"></a>

## Como responder

Cada card esperando na ilha vai para todos os telefones pareados, e sai deles quando o card sai da
fila. No telefone:

- **Negar** e **Dispensar** (mandar para o terminal) não pedem biometria.
- **Permitir**, **Always** e responder a uma pergunta pedem a sua digital ou o seu rosto, para
  aquela resposta.

Uma resposta só vale se estiver assinada pela chave daquele telefone, feita para aquele card exato
(o mesmo comando, na mesma pasta), antes de o tempo do card acabar, e nunca vista antes. Um relay
que perde, atrasa ou repete mensagens pode fazer um card esperar ou ir para o terminal; nunca
consegue respondê-lo. **Always** pelo telefone cria a mesma regra que na ilha, para a pasta em que
o card foi oferecido.

Se este computador não conseguir guardar quais respostas um telefone já usou, ele não age sobre a
resposta daquele telefone: o card vai para o terminal.

<a id="revoking"></a>

## Como revogar

**Revogar** ao lado de um telefone e nada do que ele assinar vale a partir dali. Um telefone
revogado fica na lista e não consegue parear de novo com as mesmas chaves.
