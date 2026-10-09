# 0012. Regras antes de modelos nas decisões do próprio app
<!-- source: 7d9477d39c7b -->

**Status:** Aceita, 2026-10-04.

<a id="context"></a>

## Contexto

Foi proposto um pequeno modelo de decisão local (um classificador "Sistema 1") para escolher quando
notificar, qual visual o Zeca deve ter, para qual agente mandar uma tarefa e quando chamar um
modelo grande. Os eventos que o app recebe já são tipados (um status, uma ferramenta, uma
permissão), e a precisão publicada desses modelos sem fine-tuning fica abaixo de um chute pela
classe majoritária, com uma confiança que precisa de uma calibração que não conseguimos fazer sem
coletar dados ([0006](0006-keyring-no-telemetry.md)).

<a id="decision"></a>

## Decisão

- O que pode ser decidido a partir de estado tipado é decidido por regras simples no `core`, com
  testes: notificações, atenção, o visual do Zeca, qual agente está instalado e tem cota.
- Texto livre do usuário (o que ele pede ao Zeca) vai para o modelo do chat, que já está lá.
- Um classificador aprendido pode vir depois, só atrás da mesma interface, só para texto livre, só
  local, e só se um conjunto de avaliação neste repositório mostrar que as regras não dão conta.
- Nenhum modelo jamais responde a uma permissão ([0004](0004-a-human-answers-permissions.md)).

<a id="consequences"></a>

## Consequências

- As decisões são exatas, testáveis e explicáveis.
- Encaminhar uma tarefa entre agentes é uma preferência do usuário mais disponibilidade, não um
  palpite.
