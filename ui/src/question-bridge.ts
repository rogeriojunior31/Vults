import { Bridge, type TextQuestionView } from "./bridge";
import { questionCard } from "./questions";

/** The native surface owns the lease. Losing the page clears the local text immediately. */
export function connectQuestions(surface: "island" | "nest", mount: (card: HTMLElement | null) => void, keyboard: (on: boolean) => void) {
  let current: TextQuestionView | null = null;
  let refreshing = false;
  let epoch = 0;
  let departed = false;
  let barrier = -1;
  let leaving = false;
  const card = questionCard({
    act: async (token, action, text) => {
      try { await Bridge.textQuestionAction(token, action, text); }
      finally { void refresh(); }
    }, keyboard,
    changed: () => mount(card.element.hidden ? null : card.element),
  });
  function receive(view: TextQuestionView) {
    if (document.hidden || leaving || (current && view.seq < current.seq)) return;
    if (departed) {
      if (!view.offer || view.seq <= barrier) return;
      departed = false;
    }
    current = view;
    card.update(view.surface === surface ? view : { seq: view.seq, surface: "", phase: "", offer: null });
  }
  async function refresh() {
    if (refreshing || document.hidden) return;
    refreshing = true;
    const asked = epoch;
    try { const view = await Bridge.textQuestionStatus(card.token()); if (asked === epoch) receive(view); }
    catch { if (asked === epoch && current?.offer) { card.clear(); current = null; } }
    finally { refreshing = false; if (asked !== epoch && !leaving && !document.hidden) void refresh(); }
  }
  function depart() {
    const asked = ++epoch;
    departed = true;
    leaving = true;
    card.clear();
    if (current) current = { seq: current.seq, surface: "", phase: "", offer: null };
    void Bridge.textQuestionDepart().then(view => {
      if (asked !== epoch) return;
      barrier = Math.max(barrier, view.seq);
      leaving = false;
      void refresh();
    }, () => { /* No ordered acknowledgement: remain closed rather than restore private text. */ });
  }
  Bridge.onTextQuestion(receive);
  document.addEventListener("visibilitychange", () => { if (document.hidden) depart(); else void refresh(); });
  window.addEventListener("pagehide", depart);
  // A missing/destroyed renderer cannot keep a question alive through a stale event listener.
  window.setInterval(() => { if (current?.offer || current?.phase === "sending") void refresh(); }, 600);
  void refresh();
  return { depart, relabel: () => { if (current) receive(current); }, pending: () => !!card.token() };
}
