# Adicionando um conector
<!-- source: 900a4f3cc590 -->

Um conector observa um serviço externo e põe as novidades dele na ilha: uma verificação que falhou, um pull
request aprovado, uma review esperando por você. O GitHub é o primeiro; adicionar outro leva um arquivo Rust,
uma linha no registro e uma entrada na UI de configurações.

<a id="what-you-write"></a>

## O que você escreve

```rust
pub trait Connector: Send + Sync {
    fn id(&self) -> &'static str;                 // "github"
    fn interval(&self, last: &Snapshot) -> Duration; // espera após um poll bom, pelo que ele viu
    fn poll(&self) -> Poll<'_>;                   // o estado atual do serviço, como um Snapshot
    fn diff(&self, before: &Snapshot, after: &Snapshot) -> Vec<Event>; // novidades entre dois estados
    fn board(&self, snapshot: &Snapshot) -> Option<Vec<Row>> { None } // opcional: o card dele
}
```

Um `Snapshot` é um mapa de uma chave estável (`pr:owner/repo#12`) para qualquer JSON de que você precise para saber o que
mudou. Você nunca decide *quando* algo é novidade pelo relógio: você compara dois snapshots.
É isso que torna o resto automático.

<a id="what-the-runtime-does-for-you"></a>

## O que o runtime faz por você

- Faz poll só enquanto o usuário está com o conector ligado, e para na hora quando ele é desligado.
- Salva o último snapshot no disco, para que um reinício não repita novidades antigas.
- No primeiríssimo poll, só registra uma linha de base: nada de uma enxurrada de alertas sobre coisas que já
  eram verdade.
- Recua em caso de erro (a partir de 4 minutos, dobrando, até 15, seja qual for o seu `interval`), espera o fim de
  `Error::RateLimited { retry_after }`, e verifica só a cada 10 minutos quando o usuário precisa agir
  (`Error::Unavailable`, `Error::Auth`).
- Espera `interval(&last_snapshot)` depois de um poll bom, para que um conector possa fazer poll mais cedo enquanto algo
  está rodando (GitHub: 60 s enquanto as verificações rodam, senão 300 s).
- Faz poll antes quando a ilha abre e o último poll, bom ou com falha, tem mais de um minuto
  (`Runtime::refresh_if_stale`): novidades frescas, ou uma nova tentativa depois de um erro que o usuário pode ter corrigido. Nunca
  enquanto desligado, já fazendo poll, ou com limite de taxa.
- Não envia eventos de um poll que termina depois que o usuário desligou o conector; o snapshot dele
  ainda é salvo como linha de base.
- Envia o card (`board`, quando o conector tem um) depois de cada poll bom, depois dos eventos dele, e
  o retira quando o conector é desligado. O `item` de uma linha é a chave dela no snapshot: quando um item sai do card,
  o core descarta os alertas cujas chaves começam com `<item>:`.
- Informa o status (ligado, último poll bem-sucedido, erro, itens observados) para a janela de configurações.

<a id="steps"></a>

## Passos

1. **`crates/connectors/src/<id>.rs`**: implemente `Connector`. Mantenha o `poll` em uma requisição quando o
   serviço permitir, e transforme a resposta dele num `Snapshot` numa função pura que você consiga testar.
2. **`crates/connectors/src/lib.rs`**: adicione `pub mod <id>;` e o conector em `all()`.
3. **`ui/src/connectors.ts`**: adicione `{ id, name, about }`. `about` diz, em uma frase, o que ele observa
   e como ele faz login.
4. **Links**: a `url` de um evento só é mostrada se `core::SafeUrl` aceitá-la (https, host permitido). Adicione
   o host do serviço a `HOSTS` em `crates/core/src/safe_url.rs`.
5. **Testes**: alimente a sua função de snapshot com respostas gravadas e verifique o `diff` para cada tipo de novidade,
   e para "nada mudou significa nenhuma novidade".

<a id="credentials"></a>

## Credenciais

Prefira a CLI do próprio serviço em que o usuário já está logado, como o GitHub faz com o `gh`: o app
nunca lida com um token. Quando não há uma CLI assim, o token vai para o chaveiro do sistema (Secret
Service, Credential Manager), nunca para um arquivo, e nunca para um log.

<a id="event-keys"></a>

## Chaves de evento

`Event::key` identifica a *novidade*, não só a coisa: `pr:owner/repo#12:ci-failed`. A ilha
substitui um alerta com a mesma chave em vez de empilhar duplicatas, e uma nova falha em outro
commit (`branch:owner/repo:ci-failed:<sha>`) é um alerta novo.

`Event::topic` nomeia a história a que uma novidade pertence (`pr:owner/repo#12:ci`): um evento mais novo do
mesmo tópico aposenta os alertas mais antigos, para que *checks passed* não fique ao lado da falha que ele corrigiu.
Deixe `None` para novidades que se sustentam sozinhas. Todo evento é novidade para a ilha, mesmo com uma chave que ela
já mostrou: uma review pedida de novo toca de novo.
