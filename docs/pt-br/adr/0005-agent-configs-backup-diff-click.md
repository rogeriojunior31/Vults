# 0005. As configurações dos agentes, e os agentes que suportamos
<!-- source: 6a509aa2ef19 -->

**Status:** Aceita, 2026-10-04 (a regra existe desde o início do projeto). Consolidada em
2026-10-12 a partir da 0029.

<a id="context"></a>

## Contexto

Instalar o nosso hook significa editar arquivos que pertencem ao usuário e a outras ferramentas: a
configuração de cada agente suportado. Os agentes também vêm e vão (o Antigravity tomou o lugar do
Gemini CLI), e quem instalou os nossos hooks num agente que deixamos de suportar ainda precisa
conseguir removê-los.

<a id="decision"></a>

## Decisão

- **Antes de qualquer escrita**: uma leitura estrita, um diff que o usuário vê, um clique, depois
  um backup datado e uma escrita atômica. Os hooks de outras ferramentas ficam. O `trusted_hash` do
  Codex nunca é escrito: o próprio usuário confia nos nossos hooks no Codex.
- Tudo o que injeta configuração nossa num agente (skills, servidores MCP) passa pela mesma tela,
  ou não acontece.
- **Os agentes suportados** são Claude Code, Codex, Antigravity, OpenCode, Qwen Code, Copilot CLI,
  Cursor, Factory Droid e Kimi Code, cada um um driver em `crates/agents`. Acrescentar um é um
  driver, uma entrada no instalador e a documentação.
- **Aposentar um agente** é uma lista de passos, não uma exclusão:
  - a versão do protocolo sobe;
  - um instalador que só remove continua: o Settings ainda acha os hooks que um Vults anterior
    escreveu e oferece removê-los (diff, backup datado, clique), e nunca os instala de novo;
  - o nome dele continua reservado (`valid_agent_name` em `crates/protocol`), então os hooks
    antigos não mandam nada e nenhuma outra ferramenta pode tomar o nome;
  - os turnos antigos dele ficam fora do histórico, e uma política que o nomeia é ignorada inteira.

<a id="consequences"></a>

## Consequências

- Instalar leva uma tela a mais que uma configuração silenciosa.
- O Gemini CLI foi aposentado assim (versão 7 do protocolo): `crates/agents/src/gemini.rs` só
  remove os hooks dele.

<a id="history"></a>

## Histórico

- [0029](0029-the-supported-agents.md), 2026-10-12: a lista dos agentes suportados, e como
  aposentar um.
