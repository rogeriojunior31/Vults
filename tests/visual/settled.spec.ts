// The settled card: once a card leaves the line, core's `ended` says how, and the island says it
// for a moment. Each outcome is driven from a view, as the app sends it.
import { expect, test, type Page } from "@playwright/test";

/** Each outcome core gives, the lab state whose card it ends, the label and the baseline. A rule's
 *  answer reads as an Allow, so it shares that baseline. */
const OUTCOMES = [
  { outcome: "allowed", state: "approval", label: "Allowed", shot: "allowed" },
  { outcome: "denied", state: "approval", label: "Denied", shot: "denied" },
  { outcome: "answered", state: "question-card", label: "Answered", shot: "answered" },
  { outcome: "released", state: "approval", label: "Over to the terminal", shot: "released" },
  { outcome: "terminal", state: "approval", label: "Answered in the terminal", shot: "terminal" },
  { outcome: "expired", state: "approval", label: "Nobody answered: the terminal asks now", shot: "expired" },
  { outcome: "rule", state: "approval", label: "Allowed", shot: "allowed" },
] as const;

/** The card leaves the line the way core sends it: no approval, its session works again, and
 *  `ended` says how. */
async function endCard(page: Page, outcome: string): Promise<void> {
  await page.evaluate((outcome) => {
    const island = (window as any).island;
    const v = island.last();
    const { request, agent, session } = v.approval;
    island.render({
      ...v,
      approval: null,
      sessions: v.sessions.map((s: any) => (s.card ? { ...s, status: "working", attention: "quiet", card: false } : s)),
      ended: [{ request, agent, session, outcome }],
    });
  }, outcome);
}

for (const { outcome, state, label, shot } of OUTCOMES) {
  test(`settled card: ${outcome}`, async ({ page }) => {
    await page.goto(`/lab/?still=1&t=1500&state=${state}&open=1`);
    // The card shows once its session's state has held (a real-time wait, as in leftovers.spec.ts).
    await expect(page.locator("#island .card.focus:not(.settled)")).toBeVisible({ timeout: 4000 });
    await endCard(page, outcome);
    await expect(page.locator("#island .settled-label")).toHaveText(label);
    await expect(page.locator("#island")).toHaveScreenshot(`island-settled-${shot}.png`);
  });
}

test("settled card: a view without the card's ending says nothing", async ({ page }) => {
  await page.goto("/lab/?still=1&t=1500&state=approval&open=1");
  await expect(page.locator("#island .card.focus:not(.settled)")).toBeVisible({ timeout: 4000 });
  await page.evaluate(() => {
    const island = (window as any).island;
    const v = island.last();
    island.render({ ...v, approval: null, ended: [{ request: "other", agent: "claude", session: "lab", outcome: "allowed" }] });
  });
  await expect(page.locator("#island .settled")).toHaveCount(0);
});
