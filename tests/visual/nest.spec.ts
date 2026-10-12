import { expect, test } from "@playwright/test";

// The Nest's Chat tab (ui/lab/nest.html): full Markdown, and a card only the island answers.
test("nest: a reply in full Markdown, and a card sent to the island", async ({ page }) => {
  // The real window's size (app/src/lib.rs, open_nest).
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=chat");
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
  await page.goto("/lab/nest.html?still=1&tab=chat&busy=1");
  await expect(page.locator(".msg.zeca").last()).toHaveText("Running the tests first");
  await expect(page.locator(".ask .code")).toHaveText("Bash · cargo test");
  await expect(page.getByRole("button", { name: "Answer on the island" })).toBeVisible();
  // Busy: the send button is Stop, so nothing new is sent behind the turn.
  await expect(page.locator(".send.stop")).toBeVisible();
});

// Activity › By project (M3, ADR 0021): one card per repository, with its trees, sessions, pull requests and memory.
test("nest: the roosts, grouped by repository", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=projects");
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
  await expect(page.locator(".nest-projects-part")).toBeHidden();
});

test("nest: no roosts yet says what a roost is", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1&tab=projects&empty=1");
  await expect(page.locator(".roosts")).toContainText("a roost is a repository");
});

// The Chat's Memory (M6), beside the conversation: proposals with their diff and place, the files, the history.
test("nest: memory proposals wait for a yes", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=chat&memory=1");
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
  await page.goto("/lab/nest.html?still=1&tab=chat&memory=1&empty=1");
  await expect(page.locator(".memory")).toContainText("Nothing waits.");
  await expect(page.locator(".memory")).toContainText("Type /remember in the chat");
});

test("nest: the last sleep's debrief is shown as Zeca's reading", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1&tab=chat&memory=1");
  await expect(page.locator(".memory-night .memory-debrief")).toContainText("Windows CI fix is still open");
  await expect(page.locator(".memory-night .memory-night-line")).toHaveText("Zeca's reading of the last days' summaries:");
});

// Sharing AGENTS.md with Claude Code (M9): the diff first, then a click.
test("nest: a roost offers to share AGENTS.md with Claude, diff first", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=projects");
  const vults = page.locator(".roost").first();
  await expect(vults.locator(".roost-share")).toContainText("Claude Code does not");
  await vults.getByRole("button", { name: "Share it with Claude…" }).click();
  await expect(vults.locator(".roost-diff")).toContainText("+@AGENTS.md");
  await expect(page.locator("body")).not.toHaveAttribute("data-shared", "1");
  await vults.getByRole("button", { name: "Apply" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-shared", "1");
});

// The Tasks tab (W3): each task with its criteria, roost, owner, status, cost and issue.
test("nest: the tasks", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=tasks");
  await expect(page.locator(".task")).toHaveCount(3);
  // U8: each roost's tasks under its name, the ones with none last.
  await expect(page.locator(".tasks-roost")).toHaveText(["site", "Vults", "No project"]);
  // The review loop's rounds as marks: round 1 of 3 asked for changes.
  await expect(page.locator(".task").nth(1).locator(".task-round.changes")).toHaveCount(1);
  await expect(page.locator(".task").nth(1).locator(".task-round")).toHaveCount(3);
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

// The records (W11): History, Usage, Traces and Audit, each read when its tab opens.
test("nest: the history of finished turns", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=history");
  await expect(page.locator(".history .record")).toHaveCount(3);
  await expect(page.locator(".history .record").nth(1)).toContainText("Failed");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-history.png");
});

test("nest: usage, the plans and what the agents cost", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=cost");
  await expect(page.locator(".usage")).toContainText("against your $5.00 a day");
  // A day over the budget shows so.
  await expect(page.locator(".usage-fill.over")).toHaveCount(1);
  // U10: the daily budget is set here, over the records.
  await expect(page.locator(".nest-cost .task-spend-line").first()).toContainText("Today the agents cost $4.20 of your $5.00.");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-usage.png");
  await page.locator(".nest-cost .task-budget-input").fill("7.5");
  await page.locator(".nest-cost").getByRole("button", { name: "Save" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-budget", "750");
});

test("nest: Zeca's traces", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=traces");
  await expect(page.locator(".traces .record.child")).toHaveCount(1);
  await expect(page.locator("#nest")).toHaveScreenshot("nest-traces.png");
});

test("nest: the audit log says who answered", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=audit");
  await expect(page.locator(".audit .record")).toHaveCount(4);
  await expect(page.locator(".audit")).toContainText("Policy accepted as #3");
  await expect(page.locator(".audit")).toContainText("Your phone");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-audit.png");
});

// The Flock tab (ADR 0021): each session with its roost and branch; a card only leads to the island.
test("nest: the flock, and a card only the island answers", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1");
  await expect(page.locator(".nest-tab.on")).toHaveText("Flock");
  await expect(page.locator(".nest-session")).toHaveCount(4);
  const waiting = page.locator(".nest-session").first();
  await expect(waiting.locator(".nest-session-place")).toHaveText("rogeriojunior31/Vults · fix/quick-wins-2");
  await expect(page.getByRole("button", { name: "Allow" })).toHaveCount(0);
  await expect(page.locator("#nest")).toHaveScreenshot("nest-flock.png");
  await waiting.getByRole("button", { name: "Review on the island" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-to-island", "a");
  await waiting.getByRole("button", { name: "Open terminal" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-jump", "a");
  // The gear opens Settings; the footer goes back to the island.
  await page.getByRole("button", { name: "Settings" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-settings", "general");
  await page.getByRole("button", { name: "Back to the island" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-island", "1");
});

test("nest: Activity's Week, drawn with the settings' look", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=activity");
  await expect(page.locator(".nest-sub.on")).toHaveText("Week");
  await expect(page.locator(".nest-week")).toContainText("41 turns, 6 h 20 min with your agents, most on Vults.");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-week.png");
});

test("nest: connections show the GitHub card", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=connections");
  await expect(page.locator(".board-head")).toContainText("GitHub");
  await page.locator(".board-row").first().click();
  await expect(page.locator("body")).toHaveAttribute("data-opened", "pr:me/vults#56");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-connections.png");
});

// ADR 0021: the Nest opens with Zeca off, without its Chat.
test("nest: without Zeca there is no Chat tab", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1&zeca=0&tab=chat");
  await expect(page.locator('.nest-tab[data-tab="chat"]')).toBeHidden();
  await expect(page.locator(".nest-tab.on")).toHaveText("Flock");
  await expect(page.locator(".nest-zeca")).toHaveText("Zeca off");
  // Turned on from the footer, the Chat comes back.
  await page.locator(".nest-zeca").click();
  await expect(page.locator('.nest-tab[data-tab="chat"]')).toBeVisible();
});

// Settings → Agents (W1, moved by ADR 0021): what each agent can do with Vults.
test("settings: the agents' doctor", async ({ page }) => {
  await page.setViewportSize({ width: 720, height: 900 });
  await page.goto("/lab/doctor.html?still=1");
  await expect(page.locator(".doctor .card")).toHaveCount(3);
  const codex = page.locator(".doctor .card").nth(1);
  await expect(codex).toContainText("Codex won't run the hooks until you trust them");
  await expect(codex).toContainText("At once, when Vults started it");
  await expect(page.locator(".doctor .card").nth(2)).toContainText("Not installed");
  await expect(page.locator("#settings")).toHaveScreenshot("settings-agents-doctor.png");
});

// The Flock (U7): the habitat, a search and Needs you, the one in front in detail.
test("nest: the flock's habitat, filters and details", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1");
  await expect(page.locator(".nest-habitat .habitat-bird")).toHaveCount(4);
  // D1: the Flock never answers a card.
  await expect(page.getByRole("button", { name: "Allow" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Deny" })).toHaveCount(0);
  // The one in front, in detail: its changes, its steps, where it works.
  const details = page.locator(".nest-details");
  await expect(details.locator(".nest-session-name")).toHaveText("Vults");
  await expect(details).toContainText("Changes +78 −14");
  await expect(details).toContainText("Editing render.ts");
  await expect(details).toContainText("main");
  // Needs you keeps the waiting one; the search reads branch and agent too.
  await page.locator(".nest-needs-filter").click();
  await expect(page.locator(".nest-session")).toHaveCount(1);
  await page.locator(".nest-needs-filter").click();
  await page.getByPlaceholder("Find a session…").fill("gemini");
  await expect(page.locator(".nest-session")).toHaveCount(1);
  await expect(page.locator(".nest-session")).toContainText("site");
  await page.getByPlaceholder("Find a session…").fill("");
  // A bird in the habitat puts its session in front.
  await page.locator(".nest-habitat .habitat-bird").first().click();
  await expect(page.locator("body")).toHaveAttribute("data-focus", "a");
  await expect(details.locator(".nest-session-name")).toHaveText("Vults-quick-wins");
  await expect(details.locator(".nest-needs")).toContainText("Needs you");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-flock-details.png");
});

test("nest: Ask Zeca about a session drafts the question in the Chat", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1");
  await page.getByRole("button", { name: "Ask Zeca about this session" }).click();
  await expect(page.locator(".nest-tab.on")).toHaveText("Chat");
  await expect(page.locator(".nest-chat textarea").first()).toHaveValue("What is Vults doing now?");
});

test("nest: wide, the details sit beside the rows", async ({ page }) => {
  await page.setViewportSize({ width: 1200, height: 760 });
  await page.goto("/lab/nest.html?still=1&full=1");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-flock-full.png");
});

// The Chat's Recent (U9): in a wide window, the saved conversations beside it, and a new one.
test("nest: the chat's recent conversations, in a wide window", async ({ page }) => {
  await page.setViewportSize({ width: 1100, height: 700 });
  await page.goto("/lab/nest.html?still=1&tab=chat");
  await expect(page.locator(".nest-recent-row")).toHaveCount(2);
  await expect(page.locator(".nest-recent-row.current")).toContainText("A quieter island");
  // The dates read against today's: hidden from the picture, so it holds on any day.
  await expect(page.locator("#nest")).toHaveScreenshot("nest-chat-recent.png", { mask: [page.locator(".nest-recent-meta")] });
});

test("nest: narrow, the chat has no Recent column", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=chat");
  await expect(page.locator(".nest-recent")).toBeHidden();
});
