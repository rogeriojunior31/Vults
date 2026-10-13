import { expect, test } from "@playwright/test";

/** Each open-island screenshot and the lab state it shows (`?state=`: a label of STATES in ui/lab/lab.ts). */
const SHOTS: [string, string][] = [
  ["working", "editing"], ["searching", "searching"], ["web", "on-the-web"], ["approval", "approval"],
  ["done-with-alerts", "done"], ["chat", "chat"], ["busy-flock", "busy-flock"], ["approval-queue", "approval-queue"],
  ["idle-flock", "idle-flock"], ["chat-permission", "chat-permission"], ["question", "question"], ["failed", "failed"],
];
const CLIPS = ["idle", "think", "read", "search", "edit", "run", "approval", "question", "done", "fail", "listen", "dance", "sleep", "swallow", "preen", "startle", "hello", "gape", "fly", "signature", "speak", "doze", "wake", "court", "fed", "sun"];

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

// The prototype's pill: a failed session keeps a red edge, and a connector's news a cyan dot.
test("island compact: a failure's red edge, the news dot", async ({ page }) => {
  await page.goto(lab("state=failed"));
  await expect(page.locator(".compact")).toHaveClass(/\bfailed\b/);
  await expect(page.locator(".compact")).toHaveAttribute("aria-label", /1 failed/);
  await page.goto(lab("state=done"));
  await page.waitForTimeout(SETTLES["done-with-alerts"]);
  await expect(page.locator(".compact .pill-dot")).toBeVisible();
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
  // Mini has no text (ADR 0021): what it would say is the pill's label.
  await expect(page.locator("#island .compact")).toHaveAttribute("aria-label", /^Paused/);
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
  // Flock, Activity and Connections (and the gear); no Chat without him.
  await expect(page.locator("#island .tab:not(.tab-gear)")).toHaveCount(3);
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
  await page.locator("#island .habitat-bird.chosen").click();
  await expect(page.locator("#island")).toHaveScreenshot("island-live-diff.png");
  await page.locator(".tick-diff").click();
  await expect(page.locator(".diff-view .diff-line")).toHaveCount(11);
  await expect(page.locator("#island")).toHaveScreenshot("island-live-diff-open.png");
  await page.keyboard.press("Escape");
  await expect(page.locator(".diff-view")).toHaveCount(0);
  await expect(page.locator(".tick-diff")).toBeVisible();
});

test("island open: tabs announce their selection and alerts name their dismissal", async ({ page }) => {
  await page.goto(lab("state=done&open=1"));
  await page.waitForTimeout(SETTLES["done-with-alerts"]);
  await expect(page.getByRole("button", { name: "Flock" })).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByRole("button", { name: "Activity" })).toHaveAttribute("aria-pressed", "false");
  await expect(page.getByRole("button", { name: "Dismiss Checks failed on main · me/dog_stack" })).toBeVisible();
});

test("island open: the GitHub card", async ({ page }) => {
  await page.goto(lab("state=github-card&open=1"));
  await expect(page.locator("#island")).toHaveScreenshot("island-github-tab.png");
  await page.getByRole("button", { name: /^Connections/ }).click();
  await expect(page.locator(".qconn-row")).toHaveCount(6);
  await expect(page.locator("#island")).toHaveScreenshot("island-github-card.png");
  // A row asks the core to open it: the URL never comes from the page.
  await page.locator(".qconn-open", { hasText: "lib#7" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-opened", "github review:team/lib#7");
  // The flock tab brings the overview back.
  await page.getByRole("button", { name: "Flock" }).click();
  await expect(page.locator(".board")).toHaveCount(0);
});

test("island open: Connections' rows ask Zeca, and ✓ hides them until they change", async ({ page }) => {
  await page.goto(lab("state=github-card&open=1"));
  await page.getByRole("button", { name: /^Connections/ }).click();
  // A review asks for one, failing checks ask why; the rest keep their tag.
  await expect(page.locator(".qconn-act", { hasText: "Review with Zeca" })).toHaveCount(1);
  await expect(page.locator(".qconn-act", { hasText: "Ask Zeca why" })).toHaveCount(2);
  // The question waits in the composer: nothing is sent.
  await page.locator(".qconn-row", { hasText: "lib#7" }).locator(".qconn-act").click();
  await expect(page.locator(".chat textarea")).toHaveValue(/^Review the pull request lib#7, "Bump serde to 1.0.220"/);
  await expect(page.locator(".chat .msg")).toHaveCount(0);
  await page.getByRole("button", { name: /^Connections/ }).click();
  // ✓ hides a row, and the tab counts one fewer.
  await page.getByRole("button", { name: "Mark Seasonal looks for Zeca as seen" }).click();
  await expect(page.locator(".qconn-row")).toHaveCount(5);
  await expect(page.getByRole("button", { name: "Connections 5" })).toBeVisible();
  // It comes back once the row changes.
  await page.evaluate(() => {
    const island = (window as any).island;
    const last = island.last();
    const rows = last.boards[0].rows.map((r: any) => (r.name === "vults#55" ? { ...r, checks: "passing" } : r));
    island.render({ ...last, boards: [{ connector: "github", rows }] });
  });
  await expect(page.locator(".qconn-row")).toHaveCount(6);
  await page.getByRole("button", { name: "Mark all seen" }).click();
  await expect(page.locator(".qconn-row")).toHaveCount(0);
  await expect(page.locator(".qconn-empty")).toContainText("All caught up");
  await expect(page.locator("#island")).toHaveScreenshot("island-github-caught-up.png");
});

test("island open: Squash and merge asks first, then merges only that row", async ({ page }) => {
  await page.goto(lab("state=github-card&open=1"));
  await page.getByRole("button", { name: /^Connections/ }).click();
  // Only the user's approved, green pull request offers it.
  await expect(page.locator(".qconn-act", { hasText: "Squash and merge" })).toHaveCount(1);
  const row = page.locator(".qconn-row", { hasText: "site#9" });
  await row.getByRole("button", { name: "Squash and merge" }).click();
  await expect(row.locator(".qconn-confirm")).toHaveText(/Squash site#9 into its base branch/);
  await expect(page.locator("#island")).toHaveScreenshot("island-github-merge-ask.png");
  // A poll arriving meanwhile keeps the question open.
  await page.evaluate(() => {
    const island = (window as any).island;
    const last = island.last();
    const rows = last.boards[0].rows.map((r: any) => (r.name === "vults#55" ? { ...r, checks: "passing" } : r));
    island.render({ ...last, boards: [{ connector: "github", rows }] });
  });
  await expect(page.locator(".qconn-row", { hasText: "vults#55" }).locator(".qconn-icon.passing")).toHaveCount(1);
  await expect(row.locator(".qconn-confirm")).toHaveCount(1);
  // Cancel asks nothing of GitHub.
  await row.getByRole("button", { name: "Cancel" }).click();
  await expect(row.locator(".qconn-confirm")).toHaveCount(0);
  await expect(page.locator("body")).not.toHaveAttribute("data-merged", /.*/);
  await row.getByRole("button", { name: "Squash and merge" }).click();
  await row.getByRole("button", { name: "Merge", exact: true }).click();
  await expect(page.locator("body")).toHaveAttribute("data-merged", "github pr:me/site#9");
  await expect(row.getByRole("button", { name: "Merged" })).toBeDisabled();
});

test("island open: a refused merge says why on its row", async ({ page }) => {
  await page.goto(lab("state=github-card&open=1&refuse=1"));
  await page.getByRole("button", { name: /^Connections/ }).click();
  const row = page.locator(".qconn-row", { hasText: "site#9" });
  await row.getByRole("button", { name: "Squash and merge" }).click();
  await row.getByRole("button", { name: "Merge", exact: true }).click();
  await expect(row.locator(".qconn-confirm.error")).toHaveText(/^GitHub did not merge it: Pull request is not mergeable/);
  await expect(row.getByRole("button", { name: "Squash and merge" })).toBeEnabled();
});

test("island open: chat with an API key", async ({ page }) => {
  await page.goto(lab("state=chat&open=1&api=1"));
  await expect(page.locator("#island")).toHaveScreenshot("island-chat-api.png");
});

test("island open: the chat's card allowed for the conversation", async ({ page }) => {
  await page.goto(lab("state=chat-permission&open=1"));
  await page.getByRole("button", { name: "Always in this chat" }).click();
  await expect(page.locator(".settled-label")).toHaveText("Allowed for this conversation");
});

test("island open: the chat's model and effort", async ({ page }) => {
  await page.goto(lab("state=chat&open=1"));
  await page.getByRole("button", { name: "Default model" }).click();
  // Each engine says how it stands, read when the menu opens.
  await expect(page.locator(".model-menu .engine-state")).toHaveText(["Signed in", "Installed"]);
  await page.locator(".model-menu").getByRole("button", { name: "Sonnet" }).click();
  await page.locator(".model-menu").getByRole("button", { name: "Extra high" }).click();
  // The island's one pill: the engine, then its model and effort.
  await expect(page.locator(".place.model .engine-name")).toHaveText("Claude Code");
  await expect(page.locator(".place.model .engine-model")).toHaveText("Sonnet · Extra high");
  await expect(page.locator("#island")).toHaveScreenshot("island-chat-model.png");
  // Haiku takes no effort: the effort goes with it.
  await page.locator(".model-menu").getByRole("button", { name: "Haiku" }).click();
  await expect(page.locator(".place.model .engine-model")).toHaveText("Haiku");
  // Another engine from the pill's menu, before the first message: its own list, and only the
  // efforts all its models take.
  await page.locator(".model-menu").getByRole("button", { name: "Codex" }).click();
  await expect(page.locator(".place.model .engine-name")).toHaveText("Codex");
  await page.getByRole("button", { name: "Default model" }).click();
  await expect(page.locator(".model-menu").getByRole("button", { name: "GPT-6-Luna" })).toBeVisible();
  await expect(page.locator(".model-menu").getByRole("button", { name: "Extra high" })).toHaveCount(0);
});

test("island open: the chat's saved conversations", async ({ page }) => {
  await page.goto(lab("state=chat&open=1"));
  await page.getByRole("button", { name: "Conversations" }).click();
  await expect(page.locator(".saved-row")).toHaveCount(2);
  await expect(page.locator("#island")).toHaveScreenshot("island-chat-conversations.png");
  // Opening one goes on with it, as it was.
  await page.locator(".saved-open", { hasText: "Explain the last error" }).click();
  await expect(page.locator(".msg.zeca").last()).toContainText("expected 5 and got 6");
  await expect(page.locator(".place.model .engine-name")).toHaveText("Codex");
});

test("island compact: a song playing, nothing running", async ({ page }) => {
  await page.goto(lab("empty=1&music=1"));
  await expect(page.locator("#island")).toHaveScreenshot("island-compact-music.png");
});

test("island open: at dawn the idle flock suns itself on the wire, with no mark", async ({ page }) => {
  await page.goto(lab("state=idle-flock&open=1&hour=dawn"));
  await expect(page.locator("#island")).toHaveScreenshot("island-dawn.png");
});

test("island open: a song playing, idle birds court on the wire", async ({ page }) => {
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

// Not a screenshot: the whole reply copies, and ↑ in an empty composer brings back what was sent.
test("the chat copies a reply and recalls the last message", async ({ page }) => {
  await page.goto(lab("state=editing&open=1"));
  await page.locator(".tab[title=\"Chat\"]").click();
  const input = page.locator(".chat textarea");
  await input.fill("hello zeca");
  await input.press("Enter");
  await expect(input).toHaveValue("");
  await expect(page.locator(".chat .msg.zeca.settled .msg-tool")).toHaveText("Copy");
  await input.press("ArrowUp");
  await expect(input).toHaveValue("hello zeca");
});

test("every clip at one instant", async ({ page }) => {
  await page.goto(lab("state=editing"));
  const cards = page.locator("#clips .clip-card canvas");
  await expect(cards).toHaveCount(CLIPS.length);
  for (const [i, name] of CLIPS.entries()) {
    await expect(cards.nth(i)).toHaveScreenshot(`clip-${name}.png`);
  }
});

// L7: past the habitat's six places, five birds keep theirs and +N takes the last, opening the Nest's
// Flock; amber when one of them needs you. At rest the front session counts among them.
test("island open: a big flock keeps five on the wire and counts the rest", async ({ page }) => {
  await page.goto(lab("state=big-flock&open=1"));
  await expect(page.locator("#island .habitat-bird")).toHaveCount(5);
  const more = page.locator("#island .habitat-more");
  await expect(more).toHaveText("+11");
  await expect(more).toHaveClass(/needs/);
  await more.click();
  await expect(page.locator("body")).toHaveAttribute("data-size", "room:flock");
});

// The Quick card's one line (ADR 0021): why it asks, under the command; the buttons stay put.
test("island open: a destructive command says no policy allows it", async ({ page }) => {
  await page.goto(lab("state=destructive-approval&open=1"));
  await expect(page.locator("#island .ask-why.destructive")).toHaveText("Destructive: no policy ever allows it");
  await expect(page.locator("#island")).toHaveScreenshot("island-destructive-approval.png");
});

// ADR 0021: the Nest open is the same surface grown larger. The island steps aside, pill and all,
// with nothing to wake it; a card still opens it, unless the user is in the Nest (ADR 0021).
test("island: steps aside while the Nest is open, but a card still comes", async ({ page }) => {
  await page.goto(lab("state=busy-flock&nest=1"));
  await expect(page.locator("#island")).toHaveClass(/is-hidden/);
  // News never wakes it meanwhile: a finished session with alerts stays out of sight.
  await page.goto(lab("state=done-with-alerts&nest=1"));
  await expect(page.locator("#island")).toHaveClass(/is-hidden/);
  await page.goto(lab("state=approval&nest=1"));
  await expect(page.locator("#island")).toHaveClass(/is-open/);
  await expect(page.getByRole("button", { name: /Allow/ })).toBeVisible();
  // In the Nest, the Nest answers it: the island stays aside.
  await page.goto(lab("state=approval&nest=1&inNest=1"));
  await expect(page.locator("#island")).toHaveClass(/is-hidden/);
});

// Quick as the prototype has it: at rest the flock alone, every session named; a bird opens its
// session (back to the flock with ‹ Flock), and the dots step between sessions.
test("island open: a bird opens its session, and Flock goes back", async ({ page }) => {
  await page.goto(lab("state=busy-flock&open=1"));
  await expect(page.locator("#island .overview")).toHaveClass(/at-rest/);
  await expect(page.locator("#island .card.focus")).toBeHidden();
  await page.locator("#island .habitat-bird", { hasText: "site" }).click();
  await expect(page.locator("#island .card.focus .who .name")).toHaveText("site");
  await expect(page.locator("#island")).toHaveScreenshot("island-session-view.png");
  await page.locator("#island .session-dot").first().click();
  await expect(page.locator("#island .card.focus .who .name")).toHaveText("vults");
  await page.locator("#island .session-back").click();
  await expect(page.locator("#island .overview")).toHaveClass(/at-rest/);
});

// A waiting card holds the session view: no way back to the flock until it is answered.
test("island open: a permission keeps its session in front", async ({ page }) => {
  await page.goto(lab("state=approval&open=1"));
  await page.waitForTimeout(SETTLES.approval);
  await expect(page.locator("#island .card.focus")).toContainText("cargo test");
  await expect(page.locator("#island .session-back")).toBeHidden();
});

// Over the other tabs the need chips stay under the tabs, and one opens its session on the flock.
test("island open: a need chip over the chat opens its session", async ({ page }) => {
  await page.goto(lab("state=busy-flock&open=1"));
  await page.getByRole("button", { name: "Chat" }).click();
  await expect(page.locator("#island .need-strip.top .need-tag")).toHaveCount(2);
  await page.locator("#island .need-strip.top .need-tag", { hasText: "docs" }).click();
  await expect(page.locator("#island .card.focus .who .name")).toHaveText("docs");
  await expect(page.locator("#island .need-strip.top")).toBeHidden();
});

// The chat's paperclip opens the file chooser (the lab picks one): Zeca swallows it, a vulture
// carries it across, and it joins the next message, as a file dropped on the island does.
test("island open: the chat's paperclip hands Zeca a file", async ({ page }) => {
  await page.goto(lab("state=chat&open=1"));
  await page.getByRole("button", { name: "Attach a file" }).click();
  await expect(page.locator("#island .drop-zone .drop-title")).toHaveText("Taking notes.pdf");
  await expect(page.locator("#island .drop-zone")).toBeHidden({ timeout: 5000 });
  await expect(page.locator("#island .chips")).toContainText("notes.pdf");
  await expect(page.locator("#island .composer textarea")).toBeFocused();
});
