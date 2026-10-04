// The settled card: once a card leaves the line, core's `ended` says how, and the island says it
// for a moment. Each outcome is driven from a view, as the app sends it.
import { expect, test, type Page } from "@playwright/test";

/** Each outcome a card on screen can end with, the lab state whose card it ends, and the label.
 *  (A rule never ends the card on screen: see the Always test below.) */
const OUTCOMES = [
  { outcome: "allowed", state: "approval", label: "Allowed" },
  { outcome: "denied", state: "approval", label: "Denied" },
  { outcome: "answered", state: "question-card", label: "Answered" },
  { outcome: "released", state: "approval", label: "Over to the terminal" },
  { outcome: "terminal", state: "approval", label: "Answered in the terminal" },
  { outcome: "expired", state: "approval", label: "Nobody answered: the terminal asks now" },
] as const;

/** The card leaves the line the way core sends it: no approval, its session works again, and
 *  `ended` says how, newest first. `after` are cards that ended right after it (newer). */
async function endCard(page: Page, outcome: string, after: { request: string; outcome: string }[] = []): Promise<void> {
  await page.evaluate(
    ([outcome, after]) => {
      const island = (window as any).island;
      const v = island.last();
      const { request, agent, session } = v.approval;
      island.render({
        ...v,
        approval: null,
        sessions: v.sessions.map((s: any) => (s.card ? { ...s, status: "working", attention: "quiet", card: false } : s)),
        ended: [...after.map((e) => ({ ...e, agent, session })), { request, agent, session, outcome }, ...(v.ended ?? [])],
      });
    },
    [outcome, after] as const,
  );
}

/** Opens a lab state and waits for its card (it shows once the session's state has held). */
async function cardWaiting(page: Page, state: string): Promise<void> {
  await page.goto(`/lab/?still=1&t=1500&state=${state}&open=1`);
  await expect(page.locator("#island .card.focus:not(.settled)")).toBeVisible({ timeout: 4000 });
}

for (const { outcome, state, label } of OUTCOMES) {
  test(`settled card: ${outcome}`, async ({ page }) => {
    await cardWaiting(page, state);
    await endCard(page, outcome);
    await expect(page.locator("#island .settled-label")).toHaveText(label);
    await expect(page.locator("#island")).toHaveScreenshot(`island-settled-${outcome}.png`);
  });
}

test("settled card: an Always on a queue ends the shown card as allowed, not as the rule's", async ({ page }) => {
  await cardWaiting(page, "approval-queue");
  // Core records the shown card, then the identical queued one the new rule answered (push_front),
  // so the shown card is not first in `ended`.
  await endCard(page, "allowed", [{ request: "q2", outcome: "rule" }]);
  await expect(page.locator("#island .settled-label")).toHaveText("Allowed");
});

test("settled card: a view without the card's ending says nothing", async ({ page }) => {
  await cardWaiting(page, "approval");
  await page.evaluate(() => {
    const island = (window as any).island;
    const v = island.last();
    island.render({ ...v, approval: null, ended: [{ request: "other", agent: "claude", session: "lab", outcome: "allowed" }] });
  });
  await expect(page.getByRole("button", { name: /^Allow/ })).toHaveCount(0);
  await expect(page.locator("#island .settled")).toHaveCount(0);
});
