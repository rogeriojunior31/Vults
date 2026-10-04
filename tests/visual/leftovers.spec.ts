// What was left on the island after 0.1.0: the ticker across a session switch, and the GitHub card
// after a failed poll or behind a permission.
import { expect, test } from "@playwright/test";

/** The lab's GitHub card state (see island.spec.ts). */
const GITHUB_CARD = 15;
/** The lab's first permission state. */
const APPROVAL = 3;

test("ticker: a slide under way does not land on the next session", async ({ page }) => {
  await page.goto("/lab/?still=1");
  const texts = await page.evaluate(async () => {
    const path = "/src/island/ticker.ts";
    const { Ticker } = await import(path);
    const ticker = new Ticker();
    document.body.append(ticker.element);
    const step = (n: number, text: string) => ({ n, text, diff: null });
    ticker.sync("a", [step(1, "a one"), step(2, "a two")]);
    // A new step starts a slide; the focus moves to another session before it ends.
    ticker.sync("a", [step(1, "a one"), step(2, "a two"), step(3, "a three")]);
    ticker.sync("b", [step(7, "b seven"), step(8, "b eight")]);
    await new Promise((r) => setTimeout(r, 700));
    return [...ticker.element.querySelectorAll(".tick-text")].map((t) => t.textContent);
  });
  expect(texts).toEqual(["b seven", "b eight"]);
});

test("GitHub card: a failed poll keeps the rows and says they are old", async ({ page }) => {
  await page.goto(`/lab/?still=1&t=1500&island=${GITHUB_CARD}&open=1&stale=1`);
  await page.getByRole("button", { name: "GitHub" }).click();
  await expect(page.locator(".board-row")).toHaveCount(6);
  await expect(page.locator(".board-stale")).toHaveText("Last updated 12 min ago · GitHub did not answer (HTTP 502)");
  await expect(page.locator("#island")).toHaveScreenshot("island-github-stale.png");
});

test("GitHub card: no note while the polls go well", async ({ page }) => {
  await page.goto(`/lab/?still=1&t=1500&island=${GITHUB_CARD}&open=1`);
  await page.getByRole("button", { name: "GitHub" }).click();
  await expect(page.locator(".board-row")).toHaveCount(6);
  await expect(page.locator(".board-stale")).toHaveCount(0);
});

test("GitHub tab: rests while a permission waits, and the card stays", async ({ page }) => {
  await page.goto(`/lab/?still=1&t=1500&island=${APPROVAL}&open=1`);
  await page.waitForTimeout(1700);
  await expect(page.getByRole("button", { name: /^Allow/ })).toBeVisible();
  await page.evaluate(() => {
    const island = (window as any).island;
    island.render({ ...island.last(), boards: [{ connector: "github", rows: [] }] });
  });
  const tab = page.getByRole("button", { name: "GitHub opens once the permission is answered" });
  await expect(tab).toBeDisabled();
  await expect(page.locator(".board")).toHaveCount(0);
});
