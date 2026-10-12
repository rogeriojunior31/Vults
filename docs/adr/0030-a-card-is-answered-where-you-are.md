# 0030. A card is answered where you are

**Status:** Accepted, 2026-10-12. Amends [0021](0021-four-sizes-one-nest.md): the island is no
longer the only card host (a rule it took from 0008 and 0024); [0004](0004-a-human-answers-permissions.md)
is unchanged.

## Context

0008 made the island the only window that answers a card, to keep [0004](0004-a-human-answers-permissions.md)
simple to check. The Nest (0021) came later as a window of its own, and 0024 put the island aside
while it is open. Both kept the rule: the Nest showed a card and offered **Answer on the island** or
**Review on the island →**, and a card brought the island up over the Nest. In use that is two
surfaces for one click: the user is in the Nest's Chat, Zeca asks to run a command, and the answer
has to be given somewhere else, on top of the window they are typing in. The same holds for an
agent's card while they follow the flock in the Nest.

## Decision

- **A card is answered where the user is.** The Nest answers the flock's cards (Allow, Deny,
  Always allow, a question's choices, Reply in the terminal) on the Flock's card of the session in
  front, and its Chat's cards in the Chat, with the same buttons and rules as the island: destructive
  keeps Deny the bright one and offers no Always (ADR 0025).
- **The user is in the Nest** when its window has the focus and shows the Nest, not Settings. Then
  a card does not open the island; the Nest's need strip and sound say it waits, and the global
  Allow / Deny keys answer the card on screen in the Nest (the Chat's in its tab, the one in front
  on the Flock). Anywhere else, a card opens the island as before, so an agent never waits unseen
  (rule 1). Going into the Nest folds an island that holds a card; leaving it brings the island back.
- **Only the island and the Nest answer.** The app checks the webview behind each answer: Settings,
  the widget and the palette never do. Every answer is still a click (or its key) by the user, and
  is audited as before.

## Consequences

- One click where the user already is. **Answer on the island** and **Review on the island →** are
  gone; the palette's *Review its card* opens the card on the Flock in the Nest, or the island.
- The agent's terminal, while a card waits, says *Waiting for your answer in Vults*: no longer
  only the island.
- 0004 is as simple to check as before: two webview labels instead of one, both the app's own pages.
- The Nest's Flock grows the island's permission and question bodies; visual tests cover both.
