import { expect, test } from "@playwright/test";

/** The lab's island states, by index (see ui/lab/lab.ts). */
const STATES = ["working", "searching", "web", "approval", "done-with-alerts", "chat", "busy-flock", "approval-queue", "idle-flock", "chat-permission", "question", "failed"];
const CLIPS = ["idle", "think", "read", "search", "edit", "run", "approval", "question", "done", "fail", "sleep", "swallow", "preen", "startle", "hello", "gape", "fly"];

const lab = (params: string) => `/lab/?still=1&t=1500&${params}`;
/** States the island announces only once they hold (render.ts SETTLE_MS). */
const SETTLES: Record<string, number> = { approval: 1700, "done-with-alerts": 3200, question: 1700, failed: 1700, "approval-queue": 1700 };

test("island compact", async ({ page }) => {
  await page.goto(lab("island=0"));
  await expect(page.locator("#island")).toHaveScreenshot("island-compact.png");
});

test("island compact: finished, with news", async ({ page }) => {
  await page.goto(lab(`island=${STATES.indexOf("done-with-alerts")}`));
  await page.waitForTimeout(SETTLES["done-with-alerts"]);
  await expect(page.locator("#island")).toHaveScreenshot("island-compact-news.png");
});

test("island compact: a busy flock, with badges", async ({ page }) => {
  await page.goto(lab(`island=${STATES.indexOf("busy-flock")}`));
  await expect(page.locator("#island")).toHaveScreenshot("island-compact-flock.png");
});

test("island compact: nobody on the wire", async ({ page }) => {
  await page.goto(lab("empty=1"));
  await expect(page.locator("#island")).toHaveScreenshot("island-compact-empty.png");
});

for (const [i, name] of STATES.entries()) {
  test(`island open: ${name}`, async ({ page }) => {
    await page.goto(lab(`island=${i}&open=1`));
    if (SETTLES[name]) await page.waitForTimeout(SETTLES[name]);
    await expect(page.locator("#island")).toHaveScreenshot(`island-${name}.png`);
  });
}

test("island open: chat with an API key", async ({ page }) => {
  await page.goto(lab(`island=${STATES.indexOf("chat")}&open=1&api=1`));
  await expect(page.locator("#island")).toHaveScreenshot("island-chat-api.png");
});

test("island open: dragging a file over it", async ({ page }) => {
  await page.goto(lab(`island=${STATES.indexOf("chat")}&open=1&drag=1`));
  await expect(page.locator("#island")).toHaveScreenshot("island-drop-zone.png");
});

// Not a screenshot: typing re-renders the island, and that must never take the input away.
test("the chat keeps its input while you type", async ({ page }) => {
  await page.goto(lab("island=0&open=1"));
  await page.locator(".tab[title=\"Chat\"]").click();
  const input = page.locator(".chat textarea");
  await input.click();
  await page.keyboard.type("hello zeca", { delay: 20 });
  await expect(input).toHaveValue("hello zeca");
  await expect(input).toBeFocused();
});

test("every clip at one instant", async ({ page }) => {
  await page.goto(lab("island=0"));
  const cards = page.locator("#clips .clip-card canvas");
  await expect(cards).toHaveCount(CLIPS.length);
  for (const [i, name] of CLIPS.entries()) {
    await expect(cards.nth(i)).toHaveScreenshot(`clip-${name}.png`);
  }
});
