# 0029. Os agentes suportados, e como aposentar um
<!-- source: 45e0b97d6db2 -->

**Status:** Aceita, 2026-10-12.

<a id="context"></a>

## Contexto

O Vults instala um hook em cada agente que suporta e edita a config desse agente
([0005](0005-agent-configs-backup-diff-click.md)). Agentes vêm e vão: o Antigravity tomou o lugar
do Gemini CLI, aposentado no #310. Quem instalou nossos hooks num agente que deixamos de suportar
ainda precisa poder removê-los, e um hook antigo não pode se passar por outra coisa.

<a id="decision"></a>

## Decisão

- Os agentes suportados são Claude Code, Codex, Antigravity, OpenCode, Qwen Code, Copilot CLI,
  Cursor, Factory Droid e Kimi Code, cada um um driver em `crates/agents`.
- Aposentar um agente sobe a versão do protocolo e mantém um instalador que só remove: o Settings
  ainda encontra os hooks que um Vults anterior escreveu e oferece removê-los (diff, backup datado,
  clique), e nunca mais os instala.
- O nome dele continua reservado (`valid_agent_name` em `crates/protocol`), então os hooks antigos
  não enviam nada e nenhuma outra ferramenta pode tomar o nome.
- Os turnos antigos dele ficam fora do histórico, e nada que ele deixou amplia acesso: uma política
  que o cita é ignorada por inteiro.

<a id="consequences"></a>

## Consequências

- O Gemini CLI foi aposentado assim (protocolo versão 7): o Antigravity o substitui, e o
  `crates/agents/src/gemini.rs` só remove os hooks dele.
- Acrescentar um agente é um driver, uma entrada no instalador e a documentação; aposentar um é a
  lista deste registro, não uma remoção.
