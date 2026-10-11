# O telefone
<!-- source: 775a2b01f3ea -->

Um telefone que você pareia pode responder aos cards que esperam por você enquanto está longe da
mesa. Ele só decide: este computador continua fazendo o que você escolheu, e a ilha também mantém
todos os cards. O app de telefone do Vults ainda não saiu; até lá, o cliente de teste de linha de
comando de [O relay do telefone](../reference/relay.md) pareia e responde como um telefone.

<a id="turning-it-on"></a>

## Como ligar

Em **Settings → Phone**, ligue **Answer cards from a phone**, depois escolha uma opção em **How the
phone reaches this computer** e preencha o campo abaixo dela:

- **Through a relay**: um servidor pequeno que guarda mensagens seladas até o outro lado lê-las.
  Rode o seu ([O relay do telefone](../reference/relay.md)) e informe o endereço dele em **Relay**.
- **Straight (LAN or tailnet)**: este computador roda o próprio relay no endereço que você informa
  em **Listen on**, como o endereço dele na tailnet e uma porta (`100.64.0.2:8787`). O telefone
  precisa conseguir alcançá-lo.

Cada pedido ao relay é um pedido em seu nome: desligado até você ligar, no máximo 10.000 por dia,
cada um guardado no trace local ([Segurança](../safety.md)).

<a id="pairing"></a>

## Pareamento

Aperte **Pair**: um QR aparece por dois minutos. Escaneie com o telefone. O QR leva a chave pública
deste computador, o endereço do relay e um segredo de uso único; só um telefone que o viu consegue
parear, e ele pareia um telefone. Abaixo do QR, **The code's text, for the command-line test
client** dá o mesmo código em texto. O telefone então aparece em **Paired phones**.

<a id="answering"></a>

## Como responder

Cada card esperando na ilha vai para todos os telefones pareados, e sai deles quando o card sai da
fila. No preset *Paused* nenhum card existe, então nada chega ao telefone. No telefone:

- **Deny** e **Dismiss** (mandar para o terminal) não pedem biometria.
- **Allow**, **Always** e responder a uma pergunta pedem a sua digital ou o seu rosto, para aquela
  resposta.

Uma resposta só vale se a chave daquele telefone a assinou para aquele card exato, a tempo, e ela
nunca foi usada antes. Um relay que perde, atrasa ou repete mensagens pode fazer um card esperar,
nunca respondê-lo ([O que ele vê](../reference/relay.md#what-it-sees)). Se este computador não
conseguir guardar que uma resposta foi usada, a resposta é descartada e o card continua esperando.
**Always** pelo telefone cria a mesma regra que na ilha ([Aprovando](approvals.md#answering)).

<a id="revoking"></a>

## Como revogar

**Revoke** ao lado de um telefone e nada do que ele assinar vale a partir dali. Um telefone revogado
fica na lista e não consegue parear de novo com as mesmas chaves.
