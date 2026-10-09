# 0011. Oferecer só as ações que os agentes permitem
<!-- source: 14b556b92952 -->

**Status:** Aceita, 2026-10-04.

<a id="context"></a>

## Contexto

Foram propostas ações rápidas em um pássaro: abrir terminal, ver diff, aprovar, parar, renomear.
Mas o app só alcança um agente enquanto um hook de permissão ou de pergunta espera uma resposta, e
ele não é dono do processo do agente. Pular para um terminal traz a janela para a frente no KDE e
foca painéis do tmux, kitty, wezterm e herdr; em outros lugares, ele não consegue trazer uma janela
para a frente.

<a id="decision"></a>

## Decisão

Um menu mostra só o que funciona hoje, e avisa quando não dá (*Open terminal* em uma área de
trabalho onde não conseguimos trazer janelas para a frente). *Stop* não é oferecido até que um
agente dê uma forma suportada de pará-lo de fora; até lá, é pesquisa. Preferências como silenciar,
fixar e esconder ficam guardadas por projeto, porque as sessões saem depois de 10 a 30 minutos e
não são salvas.

<a id="consequences"></a>

## Consequências

- Menos botões do que nos mock-ups, mas nenhum que mente.
- Sessões que o próprio app inicia (0.2.0) podem ser paradas, porque aí somos donos do processo.
