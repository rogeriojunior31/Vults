# 0005. As configurações dos agentes só mudam depois de um backup, um diff e um clique
<!-- source: 1a5b66acf747 -->

**Status:** Aceita, 2026-10-04 (a regra existe desde o início do projeto).

<a id="context"></a>

## Contexto

Instalar nosso hook significa editar arquivos que pertencem ao usuário e a outras ferramentas: a
config de cada agente suportado ([0029](0029-the-supported-agents.md)).

<a id="decision"></a>

## Decisão

Antes de qualquer escrita: uma leitura estrita, um diff que o usuário vê, um clique, e então um
backup datado e uma escrita atômica. Hooks de outras ferramentas ficam. O `trusted_hash` do Codex
nunca é escrito: é o usuário quem confia nos nossos hooks no Codex.

<a id="consequences"></a>

## Consequências

- Instalar leva uma tela a mais do que uma configuração silenciosa.
- Ideias que injetam nossa própria config nos agentes (skills, servidores MCP) passam pela mesma
  tela, ou não acontecem.
