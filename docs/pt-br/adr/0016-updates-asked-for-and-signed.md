# 0016. Atualizações: pedidas, assinadas, um canal por vez
<!-- source: a93941bf2cb2 -->

**Status:** Aceita, 2026-10-08. Emenda a [0006](0006-keyring-no-telemetry.md). O atualizador em si
chega depois da 0.2.0 ([0015](0015-everything-by-0.2-in-small-releases.md)).

<a id="context"></a>

## Contexto

Hoje o usuário fica sabendo de uma release pelo GitHub e a instala à mão. O atualizador do Tauri
baixaria e instalaria as novas versões sozinho, a partir de um manifesto assinado. Ele também
seria o primeiro pedido que o app faz por conta própria: a [0006](0006-keyring-no-telemetry.md)
promete que nada sai da máquina a menos que o usuário tenha escolhido.

<a id="decision"></a>

## Decisão

- A verificação fica desligada até o usuário ligá-la (no Settings, e oferecida uma vez na primeira
  execução). Quando está ligada, o app busca um único manifesto estático no GitHub Releases no
  máximo uma vez por dia e não envia nada além do próprio pedido: nenhum id, nenhum dado de uso,
  nenhuma versão na query string.
- Uma atualização só é instalada depois de um clique que mostra a versão e suas notas. Nada se
  instala sozinho, e um agente em execução nunca é interrompido: a reinicialização espera até o
  usuário escolhê-la.
- Três canais, cada um com sua própria URL de manifesto: **stable** (o padrão), **beta** (release
  candidates) e **nightly** (commits selecionados). Um build conhece seu canal em tempo de
  compilação, então um build stable nunca consegue ler o manifesto nightly; trocar de canal é uma
  reinstalação.
- Toda atualização é assinada. A chave privada vive só nos segredos do job de release; a chave
  pública vem embutida no app. Um manifesto ou pacote que falha na verificação é ignorado e
  registrado no log.
- Pacotes que uma distribuição atualiza (AUR, um futuro Flatpak) saem com o atualizador fora da
  compilação.

<a id="consequences"></a>

## Consequências

- A 0006 ganha uma frase, no pull request que aceita este registro: a verificação de atualizações é
  o único pedido que o app faz por conta própria, e só quando o usuário a ativou.
- O `release.yml` assina os pacotes e publica um manifesto por canal; a chave é criada uma vez e
  guardada em backup offline (perdê-la significa que os usuários reinstalam à mão).
- Os builds nightly e beta precisam de um sufixo próprio no bundle id, para que se instalem ao lado
  do stable sem compartilhar configurações nem o chaveiro do sistema.
