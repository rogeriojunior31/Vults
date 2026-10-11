import { expect, test } from "@playwright/test";

// The Nest's Chat tab (ui/lab/nest.html): full Markdown, and a card only the island answers.
test("nest: a reply in full Markdown, and a card sent to the island", async ({ page }) => {
  // The real window's size (app/src/lib.rs, open_nest).
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1");
  const reply = page.locator(".msg.zeca").first();
  // A reply's `###` is never larger than the chat's own titles.
  await expect(reply.locator("h5")).toHaveText("What changed");
  await expect(reply.locator("ol li")).toHaveCount(2);
  await expect(reply.locator("table td").first()).toHaveText("Fold after");
  await expect(reply.locator("blockquote")).toContainText("until you answer it");
  // A link shows its address and is never followed.
  await expect(reply.locator("a")).toHaveCount(0);
  await expect(reply.locator(".link-url")).toHaveText(" (https://example.com/guide)");
  // No Allow or Deny here (ADR 0008): the card brings the island up.
  await expect(page.getByRole("button", { name: "Allow" })).toHaveCount(0);
  await page.getByRole("button", { name: "Answer on the island" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-to-island", "1");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-chat.png");
});

test("nest: opened mid-turn, it shows the reply so far and the card waiting", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1&busy=1");
  await expect(page.locator(".msg.zeca").last()).toHaveText("Running the tests first");
  await expect(page.locator(".ask .code")).toHaveText("Bash · cargo test");
  await expect(page.getByRole("button", { name: "Answer on the island" })).toBeVisible();
  // Busy: the send button is Stop, so nothing new is sent behind the turn.
  await expect(page.locator(".send.stop")).toBeVisible();
});

// The Roosts tab (M3): one card per repository, with its trees, sessions, pull requests and memory.
test("nest: the roosts, grouped by repository", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=roosts");
  await expect(page.locator(".nest-chat")).toBeHidden();
  await expect(page.locator(".roost")).toHaveCount(2);
  const vults = page.locator(".roost").first();
  await expect(vults.locator(".roost-name")).toHaveText("rogeriojunior31/Vults");
  await expect(vults.locator(".roost-tree")).toHaveCount(2);
  await expect(vults.locator(".roost-memory li")).toHaveCount(2);
  // A tree on a detached HEAD says so; a roost with no memory says nothing yet.
  await expect(page.locator(".roost").last().locator(".roost-branch")).toHaveText("detached");
  await expect(page.locator(".roost").last()).toContainText("Nothing yet.");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-roosts.png");
  // Back to the chat, as it was.
  await page.locator('.nest-tab[data-tab="chat"]').click();
  await expect(page.locator(".nest-chat")).toBeVisible();
  await expect(page.locator(".nest-roosts")).toBeHidden();
});

test("nest: no roosts yet says what a roost is", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1&tab=roosts&empty=1");
  await expect(page.locator(".roosts")).toContainText("a roost is a repository");
});

// The Memory tab (M6): proposals with their diff and place, the files, the history.
test("nest: memory proposals wait for a yes", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=memory");
  await expect(page.locator(".proposal")).toHaveCount(2);
  // A roost's proposal names its project: the place is shown before any yes.
  await expect(page.locator(".proposal").first().locator(".proposal-place")).toHaveText("Project rogeriojunior31/Vults · ~/Projects/Vults");
  await expect(page.locator(".proposal").nth(1).locator(".diff-old")).toHaveText("- Uses zsh.");
  await expect(page.locator(".memory-rate")).toHaveText("3 of 4 accepted in the last 90 days.");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-memory.png");
  await page.locator(".proposal").first().getByRole("button", { name: "Remember" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-decided", "1");
});

test("nest: an empty memory says how to fill it", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1&tab=memory&empty=1");
  await expect(page.locator(".memory")).toContainText("Nothing waits.");
  await expect(page.locator(".memory")).toContainText("Type /remember in the chat");
});

test("nest: the last sleep's debrief is shown as Zeca's reading", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1&tab=memory");
  await expect(page.locator(".memory-night .memory-debrief")).toContainText("Windows CI fix is still open");
  await expect(page.locator(".memory-night .memory-night-line")).toHaveText("Zeca's reading of the last days' summaries:");
});

// Sharing AGENTS.md with Claude Code (M9): the diff first, then a click.
test("nest: a roost offers to share AGENTS.md with Claude, diff first", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=roosts");
  const vults = page.locator(".roost").first();
  await expect(vults.locator(".roost-share")).toContainText("Claude Code does not");
  await vults.getByRole("button", { name: "Share it with Claude…" }).click();
  await expect(vults.locator(".roost-diff")).toContainText("+@AGENTS.md");
  await expect(page.locator("body")).not.toHaveAttribute("data-shared", "1");
  await vults.getByRole("button", { name: "Apply" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-shared", "1");
});

// The Agents tab (W1): what each agent can do with Vults, and what this computer has.
test("nest: the agents' doctor", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=agents");
  await expect(page.locator(".agent")).toHaveCount(3);
  const codex = page.locator(".agent").nth(1);
  await expect(codex).toContainText("Codex won't run the hooks until you trust them");
  await expect(codex).toContainText("At once, when Vults started it");
  await expect(page.locator(".agent.absent")).toContainText("Not installed");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-agents.png");
});

// The Tasks tab (W3): each task with its criteria, roost, owner, status, cost and issue.
test("nest: the tasks", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=tasks");
  await expect(page.locator(".task")).toHaveCount(3);
  const first = page.locator(".task").first();
  await expect(first).toContainText("1 of 3 met");
  await expect(first).toContainText("Cost $0.42");
  await expect(page.locator(".task").nth(1)).toContainText("Open an issue");
  await first.locator(".task-check").nth(1).click();
  await expect(page.locator("body")).toHaveAttribute("data-edited", "3:criterion");
  // W4: only a task Vults can start offers Start; a running one offers Stop.
  await expect(first).toContainText("Stop");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-tasks.png");
  await page.locator(".task").nth(1).getByText("Go on").click();
  await expect(page.locator("body")).toHaveAttribute("data-started", "2:claude");
  await page.locator(".task").nth(1).getByText("Hand off to Codex").click();
  await expect(page.locator("body")).toHaveAttribute("data-handoff", "2:codex");
  // W6: the other agent reviews it; the last review shows under the task.
  await expect(page.locator(".task").nth(1)).toContainText("Review asked for changes (round 1)");
  // W8: the card says why the router chose its agent.
  await expect(page.locator(".task").nth(1)).toContainText("Delegated to Codex: Claude Code is at 91% of its plan.");
  await page.locator(".task").nth(1).getByText("Review with Codex").click();
  await expect(page.locator("body")).toHaveAttribute("data-review", "2");
});

test("nest: no tasks says how to make one", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1&tab=tasks&empty=1");
  await expect(page.locator(".nest-tasks")).toContainText("Type /task in the chat");
});
