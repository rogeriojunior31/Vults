# O relay do telefone
<!-- source: aa825313141e -->

Um telefone pareado recebe os seus cards e envia as suas respostas por um relay: um servidor
pequeno que guarda mensagens seladas até o outro lado lê-las. Parear um telefone pelas
Configurações ainda não está no app; esta página serve para rodar um relay antes disso, ou para
conferir o que ele consegue ver.

<a id="what-it-sees"></a>

## O que ele vê

Cada direção entre o desktop e um telefone tem a sua própria caixa, endereçada por 32 bytes de
aparência aleatória que só os dois conseguem derivar das suas chaves. Uma mensagem é selada
(XChaCha20-Poly1305, com uma chave vinda do X25519) antes de sair de qualquer uma das pontas, então
o relay guarda só endereços e texto cifrado: nenhum comando, nenhum projeto, nenhum nome. Uma
mensagem é entregue uma vez e descartada depois de 10 minutos, mesmo que ninguém a tenha lido. Não
há contas e nada é gravado em disco.

Uma resposta do telefone só vale se estiver assinada pela chave daquele telefone, feita para
aquele card exato, no prazo, e nunca vista antes. Um relay que perde, atrasa ou repete mensagens
pode fazer um card esperar ou ir para o terminal; nunca consegue respondê-lo.

<a id="running-it"></a>

## Como rodar

A partir da raiz do repositório:

```sh
docker build -f packaging/relay/Dockerfile -t vults-relay .
docker run -p 8787:8787 vults-relay
```

ou sem contêiner:

```sh
cargo run --release -p vults-relay --bin vults-relay
```

`VULTS_RELAY_ADDR` diz onde ele escuta (`0.0.0.0:8787` por padrão). Ele fala HTTP puro: coloque um
proxy reverso com TLS na frente, ou acesse-o só por uma tailnet ou pela sua rede local.

<a id="limits"></a>

## Limites

| O quê | Limite |
|---|---|
| Uma mensagem | 64 KiB |
| Mensagens esperando numa caixa | 64 |
| Caixas ao mesmo tempo | 10.000 |
| Uma mensagem não lida | 10 minutos |
| Uma leitura esperando uma mensagem | 25 segundos |

## HTTP

| Pedido | Faz |
|---|---|
| `POST /v1/boxes/{box}` | Guarda o corpo (uma mensagem selada). `202`; `404` para um endereço que não tem 64 caracteres hexadecimais minúsculos; `413` grande demais; `429` a caixa está cheia; `503` caixas demais |
| `GET /v1/boxes/{box}?wait=s` | Leva tudo o que espera, como um array JSON de strings; sem nada lá, espera até `s` segundos (25 no máximo) por algo chegar |
| `GET /health` | `ok` |

<a id="trying-it"></a>

## Como testar

Dois substitutos de linha de comando fazem o papel do desktop e do telefone:

```sh
cargo run -p vults-relay --bin vults-relay
cargo run -p vults-relay --example desk -- http://127.0.0.1:8787
cargo run -p vults-relay --example phone -- '<o texto do QR que o desk mostrou>'
```

O `desk` mostra o texto do QR, espera o telefone e manda um card para ele; o `phone` pareia, mostra
o card e envia a sua resposta, assinada; o `desk` mostra o que o Vults faria com ela.
