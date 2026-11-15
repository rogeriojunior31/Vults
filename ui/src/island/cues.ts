// What a settled state does on the island: its sound and the island coming forward.
import type { Presence, SessionView } from "../bridge";

/** Quiet keeps only a card's sound (ADR 0006); Paused stays silent; a muted project is quiet at
 *  rest. A finish whose terminal was in front (FF3, `watched`) was seen ending: no sound, no
 *  reveal. A card is never `watched`, and never quieted here. */
export function settledCue(s: SessionView | undefined, preset: Presence): { sound: boolean; reveal: boolean } {
  const card = !!s?.card;
  const watched = !!s?.watched && !card;
  return {
    sound: preset !== "paused" && !watched && (card || (!s?.muted && preset !== "quiet")),
    reveal: !watched,
  };
}
