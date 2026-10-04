import { expect, test } from "@playwright/test";

/** The lab's island states, by index (see ui/lab/lab.ts). */
const STATES = ["working", "searching", "web", "approval", "done-with-alerts", "chat", "busy-flock", "approval-queue", "idle-flock", "chat-permission", "question", "failed"];
const CLIPS = ["idle", "think", "read", "search", "edit", "run", "approval", "question", "done", "fail", "listen", "dance", "sleep", "swallow", "preen", "startle", "hello", "gape", "fly", "signature"];

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

/** After the states above, in ui/lab/lab.ts: Gemini, then the question card. */
const QUESTION_CARD = STATES.length + 1;

test("island open: a question card", async ({ page }) => {
  await page.goto(lab(`island=${QUESTION_CARD}&open=1`));
  await page.waitForTimeout(SETTLES.question);
  await expect(page.locator("#island")).toHaveScreenshot("island-question-card.png");
  // A choice answers the first question; the second takes several and a Next.
  await page.locator(".choice", { hasText: "Noite" }).click();
  await page.locator(".choice", { hasText: "Chat" }).click();
  await expect(page.locator("#island")).toHaveScreenshot("island-question-card-multi.png");
  await page.getByRole("button", { name: "Other…" }).click();
  await expect(page.locator(".other-input")).toBeFocused();
});

/** The last state in ui/lab/lab.ts. */
const LIVE_DIFF = QUESTION_CARD + 1;

test("island open: a finished edit's diff", async ({ page }) => {
  await page.goto(lab(`island=${LIVE_DIFF}&open=1`));
  await expect(page.locator("#island")).toHaveScreenshot("island-live-diff.png");
  await page.locator(".tick-diff").click();
  await expect(page.locator(".diff-view .diff-line")).toHaveCount(11);
  await expect(page.locator("#island")).toHaveScreenshot("island-live-diff-open.png");
  await page.keyboard.press("Escape");
  await expect(page.locator(".diff-view")).toHaveCount(0);
  await expect(page.locator(".tick-diff")).toBeVisible();
});

test("island open: chat with an API key", async ({ page }) => {
  await page.goto(lab(`island=${STATES.indexOf("chat")}&open=1&api=1`));
  await expect(page.locator("#island")).toHaveScreenshot("island-chat-api.png");
});

test("island compact: a song playing, nothing running", async ({ page }) => {
  await page.goto(lab("empty=1&music=1"));
  await expect(page.locator("#island")).toHaveScreenshot("island-compact-music.png");
});

test("island open: a song playing, idle birds dance", async ({ page }) => {
  await page.goto(lab(`island=${STATES.indexOf("idle-flock")}&open=1&music=1`));
  await expect(page.locator("#island")).toHaveScreenshot("island-music.png");
});

for (const state of ["listening", "transcribing"]) {
  test(`island open: the chat ${state}`, async ({ page }) => {
    await page.goto(lab(`island=${STATES.indexOf("chat")}&open=1&voice=${state}`));
    await expect(page.locator("#island")).toHaveScreenshot(`island-chat-${state}.png`);
  });
}

test("island open: subscription usage in the header", async ({ page }) => {
  await page.goto(lab(`island=${STATES.indexOf("working")}&open=1&usage=1`));
  await expect(page.locator("#island")).toHaveScreenshot("island-usage.png");
});

test("island open: usage and a song share the header", async ({ page }) => {
  await page.goto(lab(`island=${STATES.indexOf("idle-flock")}&open=1&usage=1&music=1`));
  await expect(page.locator("#island")).toHaveScreenshot("island-usage-music.png");
});

test("island open: dragging a file over it", async ({ page }) => {
  await page.goto(lab(`island=${STATES.indexOf("chat")}&open=1&drag=1`));
  await expect(page.locator("#island")).toHaveScreenshot("island-drop-zone.png");
});

// Not a screenshot: the talk shortcut, held then let go, leaves the words in the input.
test("holding the talk shortcut records, letting go transcribes", async ({ page }) => {
  await page.goto(lab("island=0"));
  const shortcut = (id: string) => page.evaluate((s) => (window as unknown as { island: { shortcut(id: string): void } }).island.shortcut(s), id);
  await shortcut("talk");
  await expect(page.locator(".wave")).toBeVisible();
  await page.waitForTimeout(300);
  await shortcut("talk-up");
  await expect(page.locator(".chat textarea")).toHaveValue("Why is the build failing on the release branch?");
  // Let go at once: the stop waits for the mic to open instead of finding nothing.
  await page.locator(".chat textarea").fill("");
  await shortcut("talk");
  await shortcut("talk-up");
  await expect(page.locator(".chat textarea")).toHaveValue("Why is the build failing on the release branch?");
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
