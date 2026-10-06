import { expect, test } from "@playwright/test";

/** Each open-island screenshot and the lab state it shows (`?state=`: a label of STATES in ui/lab/lab.ts). */
const SHOTS: [string, string][] = [
  ["working", "editing"], ["searching", "searching"], ["web", "on-the-web"], ["approval", "approval"],
  ["done-with-alerts", "done"], ["chat", "chat"], ["busy-flock", "busy-flock"], ["approval-queue", "approval-queue"],
  ["idle-flock", "idle-flock"], ["chat-permission", "chat-permission"], ["question", "question"], ["failed", "failed"],
];
const CLIPS = ["idle", "think", "read", "search", "edit", "run", "approval", "question", "done", "fail", "listen", "dance", "sleep", "swallow", "preen", "startle", "hello", "gape", "fly", "signature"];

const lab = (params: string) => `/lab/?still=1&t=1500&${params}`;
/** States the island announces only once they hold (render.ts SETTLE_MS). */
const SETTLES: Record<string, number> = { approval: 1700, "done-with-alerts": 3200, question: 1700, failed: 1700, "approval-queue": 1700 };

test("island compact", async ({ page }) => {
  await page.goto(lab("state=editing"));
  await expect(page.locator("#island")).toHaveScreenshot("island-compact.png");
});

test("island compact: finished, with news", async ({ page }) => {
  await page.goto(lab("state=done"));
  await page.waitForTimeout(SETTLES["done-with-alerts"]);
  await expect(page.locator("#island")).toHaveScreenshot("island-compact-news.png");
});

test("island compact: a busy flock, with badges", async ({ page }) => {
  await page.goto(lab("state=busy-flock"));
  await expect(page.locator("#island")).toHaveScreenshot("island-compact-flock.png");
});

test("island compact: nobody on the wire", async ({ page }) => {
  await page.goto(lab("empty=1"));
  await expect(page.locator("#island")).toHaveScreenshot("island-compact-empty.png");
});

test("island compact: paused, empty and saying so", async ({ page }) => {
  await page.goto(lab("state=busy-flock&presence=paused"));
  await expect(page.locator("#island .pill-text")).toContainText("Paused");
  await expect(page.locator("#island")).toHaveScreenshot("island-compact-paused.png");
});

test("quiet: nothing at rest, and a card still opens the island", async ({ page }) => {
  await page.goto(lab("state=busy-flock&presence=quiet"));
  await expect(page.locator("#island")).toHaveClass(/is-hidden/);
  await page.goto(lab("state=approval&presence=quiet"));
  await page.waitForTimeout(SETTLES.approval);
  await expect(page.locator("#island")).toHaveClass(/is-open/);
  await expect(page.locator("#island .card")).toContainText("cargo test");
});

test("without Zeca: an empty wire, and the front session on his perch", async ({ page }) => {
  await page.goto(lab("empty=1&nozeca=1"));
  await expect(page.locator("#island")).toHaveScreenshot("island-compact-no-zeca-empty.png");
  await page.goto(lab("state=busy-flock&nozeca=1"));
  await expect(page.locator("#island")).toHaveScreenshot("island-compact-no-zeca-flock.png");
  await page.goto(lab("empty=1&nozeca=1&open=1"));
  await expect(page.locator("#island .tab")).toHaveCount(1);
  await expect(page.locator("#island")).not.toContainText("Ask Zeca");
  await expect(page.locator("#island")).toHaveScreenshot("island-open-no-zeca-empty.png");
});

for (const [name, state] of SHOTS) {
  test(`island open: ${name}`, async ({ page }) => {
    await page.goto(lab(`state=${state}&open=1`));
    if (SETTLES[name]) await page.waitForTimeout(SETTLES[name]);
    await expect(page.locator("#island")).toHaveScreenshot(`island-${name}.png`);
  });
}

test("island open: a question card", async ({ page }) => {
  await page.goto(lab("state=question-card&open=1"));
  await page.waitForTimeout(SETTLES.question);
  await expect(page.locator("#island")).toHaveScreenshot("island-question-card.png");
  // A choice answers the first question; the second takes several and a Next.
  await page.locator(".choice", { hasText: "Noite" }).click();
  await page.locator(".choice", { hasText: "Chat" }).click();
  await expect(page.locator("#island")).toHaveScreenshot("island-question-card-multi.png");
  await page.getByRole("button", { name: "Other…" }).click();
  await expect(page.locator(".other-input")).toBeFocused();
});

test("island open: a finished edit's diff", async ({ page }) => {
  await page.goto(lab("state=live-diff&open=1"));
  await expect(page.locator("#island")).toHaveScreenshot("island-live-diff.png");
  await page.locator(".tick-diff").click();
  await expect(page.locator(".diff-view .diff-line")).toHaveCount(11);
  await expect(page.locator("#island")).toHaveScreenshot("island-live-diff-open.png");
  await page.keyboard.press("Escape");
  await expect(page.locator(".diff-view")).toHaveCount(0);
  await expect(page.locator(".tick-diff")).toBeVisible();
});

test("island open: the GitHub card", async ({ page }) => {
  await page.goto(lab("state=github-card&open=1"));
  await expect(page.locator("#island")).toHaveScreenshot("island-github-tab.png");
  await page.getByRole("button", { name: "GitHub" }).click();
  await expect(page.locator(".board-row")).toHaveCount(6);
  await expect(page.locator("#island")).toHaveScreenshot("island-github-card.png");
  // A row asks the core to open it: the URL never comes from the page.
  await page.locator(".board-row", { hasText: "lib#7" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-opened", "github review:team/lib#7");
  // The flock tab brings the overview back.
  await page.getByRole("button", { name: "Flock" }).click();
  await expect(page.locator(".board")).toHaveCount(0);
});

test("island open: chat with an API key", async ({ page }) => {
  await page.goto(lab("state=chat&open=1&api=1"));
  await expect(page.locator("#island")).toHaveScreenshot("island-chat-api.png");
});

test("island compact: a song playing, nothing running", async ({ page }) => {
  await page.goto(lab("empty=1&music=1"));
  await expect(page.locator("#island")).toHaveScreenshot("island-compact-music.png");
});

test("island open: a song playing, idle birds dance", async ({ page }) => {
  await page.goto(lab("state=idle-flock&open=1&music=1"));
  await expect(page.locator("#island")).toHaveScreenshot("island-music.png");
});

for (const state of ["listening", "transcribing"]) {
  test(`island open: the chat ${state}`, async ({ page }) => {
    await page.goto(lab(`state=chat&open=1&voice=${state}`));
    await expect(page.locator("#island")).toHaveScreenshot(`island-chat-${state}.png`);
  });
}

test("island open: subscription usage in the header", async ({ page }) => {
  await page.goto(lab("state=editing&open=1&usage=1"));
  await expect(page.locator("#island")).toHaveScreenshot("island-usage.png");
});

test("island open: usage and a song share the header", async ({ page }) => {
  await page.goto(lab("state=idle-flock&open=1&usage=1&music=1"));
  await expect(page.locator("#island")).toHaveScreenshot("island-usage-music.png");
});

test("island open: dragging a file over it", async ({ page }) => {
  await page.goto(lab("state=chat&open=1&drag=1"));
  await expect(page.locator("#island")).toHaveScreenshot("island-drop-zone.png");
});

// Not a screenshot: the talk shortcut, held then let go, leaves the words in the input.
test("holding the talk shortcut records, letting go transcribes", async ({ page }) => {
  await page.goto(lab("state=editing"));
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
  await page.goto(lab("state=editing&open=1"));
  await page.locator(".tab[title=\"Chat\"]").click();
  const input = page.locator(".chat textarea");
  await input.click();
  await page.keyboard.type("hello zeca", { delay: 20 });
  await expect(input).toHaveValue("hello zeca");
  await expect(input).toBeFocused();
});

test("every clip at one instant", async ({ page }) => {
  await page.goto(lab("state=editing"));
  const cards = page.locator("#clips .clip-card canvas");
  await expect(cards).toHaveCount(CLIPS.length);
  for (const [i, name] of CLIPS.entries()) {
    await expect(cards.nth(i)).toHaveScreenshot(`clip-${name}.png`);
  }
});
