# O relay do telefone
<!-- source: db5d89096379 -->

Um telefone pareado recebe os seus cards e envia as suas respostas por um relay: um servidor
pequeno que guarda mensagens seladas até o outro lado lê-las. Parear um telefone pelas
Configurações ainda não está no app; esta página serve para rodar um relay antes disso, ou para
conferir o que ele consegue ver.

<a id="what-it-sees"></a>

## O que ele vê

Cada direção entre o desktop e um telefone tem a sua própria caixa, endereçada por 32 bytes de
aparência aleatória que só os dois conseguem derivar das suas chaves. Uma mensagem é selada
(XChaCha20-Poly1305, com uma chave vinda do X25519) antes de sair de qualquer uma das pontas e não
nomeia aparelho nenhum, então o relay guarda só endereços e texto cifrado: nenhum comando, nenhum
projeto, nenhum nome. Ele ainda vê de onde os pedidos vêm e quando, o que pode lhe dizer que duas
caixas andam juntas. Uma mensagem fica até a próxima leitura de quem a lê dizer que já a tem, e é
descartada depois de 10 minutos mesmo que ninguém a tenha lido. Não há contas e o relay não grava
nada em disco.

O endereço de uma caixa é a sua única chave: quem o descobre pode ler o texto cifrado, ou dizer que
já o leu para o relay descartá-lo. Acesse o relay por HTTPS ou por uma tailnet, e mantenha os
caminhos dos pedidos fora dos logs de um proxy reverso.

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
proxy reverso com TLS na frente, ou acesse-o só por uma tailnet ou pela sua rede local. Um proxy na
frente também deve limitar as conexões por endereço: os limites do próprio relay valem para o
servidor inteiro.

<a id="limits"></a>

## Limites

| O quê | Limite |
|---|---|
| Uma mensagem | 64 KiB |
| Mensagens esperando numa caixa | 64 |
| Caixas ao mesmo tempo | 10.000 |
| Mensagens guardadas no total | 256 MiB |
| Uma mensagem não lida | 10 minutos |
| Uma leitura esperando uma mensagem | 25 segundos |
| Um pedido, com o corpo | 35 segundos |
| Conexões ao mesmo tempo (as outras esperam a vez) | 1.000 |
| Uma conexão que não envia nada | 60 segundos |

## HTTP

| Pedido | Faz |
|---|---|
| `POST /v1/boxes/{box}` | Guarda o corpo (uma mensagem selada). `202`; `400` o corpo não é UTF-8; `404` um endereço que não tem 64 caracteres hexadecimais minúsculos; `413` grande demais; `429` a caixa está cheia; `503` caixas demais, ou bytes demais guardados |
| `GET /v1/boxes/{box}?after=n&wait=s` | Descarta as mensagens até `n` (quem lê já as tem) e entrega o resto como um array JSON de `{"seq", "msg"}`; sem nada lá, espera até `s` segundos (25 no máximo) por algo chegar. `400` uma query que não são dois números; `404` um endereço que não é um |
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
