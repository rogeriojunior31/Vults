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
  // The reply so far, with no Copy yet: it is still being written.
  await expect(page.locator(".msg.zeca").last()).toContainText("Running the tests first");
  await expect(page.locator(".msg.zeca").last()).not.toHaveClass(/settled/);
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
  await page.locator('.suite-tab[data-tab="chat"]').click();
  await expect(page.locator(".nest-chat")).toBeVisible();
  await expect(page.locator(".nest-projects-part")).toBeHidden();
});

// U11: a roost's pull request and its default branch's checks open through the card's row (D1 kept:
// opening a link answers nothing).
test("nest: a roost's pull request and checks open on GitHub", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1&tab=projects");
  const vults = page.locator(".roost").first();
  await vults.locator(".board-row.link").first().click();
  await expect(page.locator("body")).toHaveAttribute("data-opened", "pr:rogeriojunior31/Vults#243");
  await vults.getByRole("button", { name: "Default branch: Passing" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-opened", "branch:rogeriojunior31/Vults");
  // A roost with no GitHub has nothing to open.
  await expect(page.locator(".roost").last().locator("button.row-tag, .board-row.link")).toHaveCount(0);
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
  await expect(page.locator(".suite-tab.on .nav-label")).toHaveText("Flock");
  // L2: the stage, a roost card under each bird.
  await expect(page.locator(".roost-card")).toHaveCount(4);
  await expect(page.locator("#nest")).toHaveScreenshot("nest-flock.png");
  // The waiting one shows what it asks, and only the island answers it (D1).
  await page.locator(".roost-card").first().click();
  await expect(page.locator("body")).toHaveAttribute("data-focus", "a");
  const card = page.locator(".focus-block");
  await expect(card.locator(".card-words")).toHaveText("Run the test suite, then the linter");
  await expect(card.locator(".command code")).toHaveText("Bash · cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings");
  await expect(card.locator(".q-of")).toHaveText("1 of 2");
  await expect(page.getByRole("button", { name: "Allow" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Deny" })).toHaveCount(0);
  await card.getByRole("button", { name: "Review on the island →" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-to-island", "a");
  await card.getByRole("button", { name: "Open terminal" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-jump", "a");
  // The Settings tab opens Settings; the footer goes back to the island.
  await page.getByRole("tab", { name: "Settings" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-settings", "general");
  await page.getByRole("button", { name: "Back to the island" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-island", "1");
});

// L4: the prototype's Week: four figures, a bar a day with a day to inspect, the recap beside it.
test("nest: Activity's Week, as the prototype draws it", async ({ page }) => {
  await page.setViewportSize({ width: 1040, height: 900 });
  await page.goto("/lab/nest.html?still=1&tab=activity");
  await expect(page.locator(".nest-sub.on")).toHaveText("Week");
  await expect(page.locator(".nest-week")).toContainText("41 turns, 6 h 20 min with your agents, most on Vults.");
  const week = page.locator(".nest-week");
  await expect(week.locator(".metric")).toHaveCount(4);
  // The busiest day is shown first; a click on another shows that one.
  await expect(week.locator(".chart-detail strong")).toHaveText("Wed · 2 h 10 min");
  await week.getByRole("button", { name: /^Tue:/ }).click();
  await expect(week.locator(".chart-detail strong")).toHaveText(/^Tue · /);
  await expect(week.getByRole("button", { name: /^Tue:/ })).toHaveAttribute("aria-pressed", "true");
  // Picked with the keyboard, the day keeps the focus through the redraw.
  await week.getByRole("button", { name: /^Thu:/ }).focus();
  await page.keyboard.press("Enter");
  await expect(week.getByRole("button", { name: /^Thu:/ })).toBeFocused();
  await expect(week.getByRole("checkbox", { name: /Hide project names/ })).toBeVisible();
  await expect(page.locator("#nest")).toHaveScreenshot("nest-week.png");
});

test("nest: connections show the GitHub card", async ({ page }) => {
  await page.setViewportSize({ width: 1040, height: 760 });
  await page.goto("/lab/nest.html?still=1&tab=connections");
  // L5: the connector's chip, its head (when it synced, Refresh), its rows by group, one in detail.
  await expect(page.locator(".conn-chip.on")).toContainText("GitHub");
  await expect(page.locator(".conn-head h2")).toHaveText("GitHub");
  await expect(page.locator(".conn-status")).toHaveText("Synced 2 min ago");
  // U11: All, Waiting for your review and Checks failed, each with its count, filter the rows.
  await expect(page.locator(".nest-filter")).toHaveText(["All 3", "Waiting for your review 1", "Checks failed 1"]);
  await page.locator(".nest-filter").nth(1).click();
  await expect(page.locator(".conn-row")).toHaveCount(1);
  await expect(page.locator(".conn-row")).toContainText("lib#7");
  await expect(page.locator(".conn-row .conn-tag")).toHaveText("Review requested");
  await page.locator(".nest-filter").first().click();
  // A row picked shows in detail; only its button opens it.
  await page.locator(".conn-row").nth(2).click();
  await expect(page.locator(".conn-detail h3")).toHaveText("main · Merge pull request #54");
  await page.locator(".conn-row").first().click();
  await expect(page.locator(".conn-detail")).toContainText("Checks failing");
  await expect(page.locator(".conn-detail")).toContainText("Code review");
  await expect(page.locator(".conn-row").first().locator(".conn-tag")).toHaveText("Changes requested");
  await page.getByRole("button", { name: "Open in GitHub ↗" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-opened", "pr:me/vults#56");
  await page.getByRole("button", { name: "↻ Refresh" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-refreshed", "1");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-connections.png");
  // GitHub is on: nothing left to add.
  await expect(page.locator(".nest-add")).toHaveCount(0);
});

// A failed poll: the head says when it last worked and why, never "Synced"; the rows stay.
test("nest: a connector whose poll failed says so in its head", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1&tab=connections&stale=1");
  await expect(page.locator(".conn-status.stale")).toHaveText("Last updated 10 min ago · gh: not logged in");
  await expect(page.locator(".conn-row")).toHaveCount(3);
  // A filter with nothing in it says so.
  await page.locator(".nest-filter").nth(1).click();
  await page.locator(".nest-filter").nth(2).click();
  await expect(page.locator(".conn-row")).toHaveCount(1);
});

// U11: Add a connection lists only connectors Vults has, and turning one on stays in Settings.
test("nest: add a connection lists the real connectors that are off", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=connections&empty=1");
  await expect(page.locator(".nest-add-row")).toHaveCount(1);
  await expect(page.locator(".nest-add-row strong")).toHaveText("GitHub");
  await page.getByRole("button", { name: "Turn on in Settings…" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-settings", "connectors");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-connections-add.png");
});

// Turned on but not polled yet: never offered to add, and the tab says what it waits for.
test("nest: a connection that is on waits for its first check", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1&tab=connections&empty=1&gh=on");
  await expect(page.locator(".nest-connections")).toContainText("GitHub is on; its card shows after its first check.");
  await expect(page.locator(".nest-add")).toHaveCount(0);
});

test("nest: a card means on, whatever the status says", async ({ page }) => {
  // Turned on in Settings after the tab read an older status: the card that came wins.
  await page.goto("/lab/nest.html?still=1&tab=connections&gh=off");
  await expect(page.locator(".conn-head h2")).toHaveText("GitHub");
  await expect(page.locator(".nest-add")).toHaveCount(0);
});

// ADR 0021: the Nest opens with Zeca off, without its Chat.
test("nest: without Zeca there is no Chat tab", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1&zeca=0&tab=chat");
  await expect(page.locator('.suite-tab[data-tab="chat"]')).toBeHidden();
  await expect(page.locator(".suite-tab.on .nav-label")).toHaveText("Flock");
  await expect(page.locator(".nest-zeca")).toHaveText("Zeca off");
  // Turned on from the footer, the Chat comes back.
  await page.locator(".nest-zeca").click();
  await expect(page.locator('.suite-tab[data-tab="chat"]')).toBeVisible();
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
  await page.setViewportSize({ width: 1040, height: 760 });
  await page.goto("/lab/nest.html?still=1");
  await expect(page.locator(".habitat-sky .habitat-bird")).toHaveCount(4);
  // The one in front, in detail: its changes, its steps, where it works.
  await expect(page.locator(".focus-block .card-name")).toHaveText("Vults");
  await expect(page.locator(".detail-count")).toHaveText("+78 −14");
  await expect(page.locator(".diff-block")).toContainText("record_result(&result).await?;");
  await expect(page.locator(".diff-caption")).toContainText("ui/src/surfaces/nest/render.ts");
  await page.getByRole("button", { name: "Focus diff ↗" }).click();
  await expect(page.locator(".focus-block")).toBeHidden();
  await page.getByRole("button", { name: "Back to the card" }).click();
  await expect(page.locator(".focus-block")).toBeVisible();
  // Left and Right walk the detail tabs.
  await page.locator(".detail-tabs").getByRole("tab", { name: "Changes" }).focus();
  await page.keyboard.press("ArrowRight");
  await expect(page.locator(".detail-tabs").getByRole("tab", { name: "Activity" })).toHaveAttribute("aria-selected", "true");
  await expect(page.locator(".detail-tabs").getByRole("tab", { name: "Activity" })).toBeFocused();
  await page.keyboard.press("ArrowLeft");
  await page.locator(".detail-tabs").getByRole("tab", { name: "Activity" }).click();
  await expect(page.locator(".activity-row").first()).toContainText("Editing render.ts");
  await page.locator(".detail-tabs").getByRole("tab", { name: "Context" }).click();
  await expect(page.locator(".details-pane .context-dl")).toContainText("main");
  await expect(page.locator(".details-pane .context-dl")).toContainText("≈$1.37");
  // Needs you keeps the waiting one; the search reads branch and agent too.
  await page.locator(".attention-filter").click();
  await expect(page.locator(".roost-card")).toHaveCount(1);
  // Filtered, a card stays under its own bird: the waiting one is the first.
  await expect(page.locator(".roost-card")).toHaveCSS("grid-column-start", "1");
  await page.locator(".attention-filter").click();
  await page.getByPlaceholder("Find a session…").fill("qwen");
  await expect(page.locator(".roost-card")).toHaveCount(1);
  await expect(page.locator(".roost-card")).toContainText("site");
  await page.getByPlaceholder("Find a session…").fill("nothing like this");
  await page.getByRole("button", { name: "Clear filters" }).click();
  await expect(page.locator(".roost-card")).toHaveCount(4);
  // A bird in the habitat puts its session in front.
  await page.locator(".habitat-sky .habitat-bird").first().click();
  await expect(page.locator("body")).toHaveAttribute("data-focus", "a");
  await expect(page.locator(".focus-block .card-name")).toHaveText("Vults-quick-wins");
  await expect(page.locator(".rule-scope")).toContainText("Needs you");
  // Focus diff ends when another session comes in front: its card is never left hidden.
  await page.locator(".roost-card").nth(1).click();
  await page.locator(".detail-tabs").getByRole("tab", { name: "Changes" }).click();
  await page.getByRole("button", { name: "Focus diff ↗" }).click();
  await expect(page.locator(".focus-block")).toBeHidden();
  await page.locator(".roost-card").nth(2).click();
  await expect(page.locator(".focus-block")).toBeVisible();
  await page.locator(".roost-card").first().click();
  await expect(page.locator("#nest")).toHaveScreenshot("nest-flock-details.png");
});

test("nest: Ask Zeca about a session drafts the question in the Chat", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1");
  await page.locator(".focus-block").getByRole("button", { name: "Ask Zeca", exact: true }).click();
  await expect(page.locator(".suite-tab.on .nav-label")).toHaveText("Chat");
  await expect(page.locator(".nest-chat textarea").first()).toHaveValue("What is Vults doing now?");
});

test("nest: Full adds the plans' usage under the details", async ({ page }) => {
  await page.setViewportSize({ width: 1200, height: 860 });
  await page.goto("/lab/nest.html?still=1&full=1");
  await expect(page.locator(".usage-card")).toContainText("7d 74%");
  await expect(page.locator(".usage-card .meter.amber")).toHaveText("7d 74%");
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

// L1: the prototype's frame. The header counts the flock and keeps sound, search and Full; the tabs
// carry their counts; the footer has Zeca, his look, the terminal in front and the way back.
test("nest: the frame counts the flock and keeps its tools at hand", async ({ page }) => {
  await page.setViewportSize({ width: 1040, height: 760 });
  await page.goto("/lab/nest.html?still=1");
  await expect(page.locator(".session-count")).toHaveText("4 sessions · 1 needs you");
  await expect(page.getByRole("tab", { name: "Flock 1" })).toHaveAttribute("aria-selected", "true");
  await expect(page.locator('.suite-tab[data-tab="connections"] .nav-count')).toHaveText("3");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-frame.png");
  await page.getByRole("button", { name: "Mute" }).click();
  await expect(page.getByRole("button", { name: "Mute" })).toHaveAttribute("aria-pressed", "true");
  await page.getByRole("button", { name: "Command palette (Ctrl+K)" }).click();
  await expect(page.locator(".palette-host")).toBeVisible();
  await page.keyboard.press("Escape");
  await page.locator(".app-footer").getByRole("button", { name: "Zeca's look" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-settings", "flock");
  await page.locator(".app-footer").getByRole("button", { name: "Open terminal" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-jump", /./);
});

test("nest: a narrow frame keeps the tabs' icons and counts", async ({ page }) => {
  await page.setViewportSize({ width: 480, height: 360 });
  await page.goto("/lab/nest.html?still=1");
  await expect(page.locator(".suite-tab .nav-label").first()).toBeHidden();
  await expect(page.getByRole("tab", { name: "Connections 3" })).toBeVisible();
  await expect(page.locator(".session-count")).toBeHidden();
});

// L3: the Chat's title bar (the conversations' column, the title, the chat's pickers, the Memory
// panel) and, before anything is said, the composer in the middle.
test("nest: the chat's title bar and its panels", async ({ page }) => {
  await page.setViewportSize({ width: 1040, height: 760 });
  await page.goto("/lab/nest.html?still=1&tab=chat");
  await expect(page.locator(".chat-title")).toHaveText("A quieter island");
  await expect(page.locator(".chat-titlebar .chat-head")).toBeVisible();
  // Wide, Recent lists the conversations: the chat's own list button steps aside.
  await expect(page.locator(".chat-titlebar .history-box")).toBeHidden();
  await page.getByRole("button", { name: "Recent", exact: true }).click();
  await expect(page.locator(".nest-recent")).toBeHidden();
  await expect(page.locator(".chat-titlebar .history-box")).toBeVisible();
  await page.getByRole("button", { name: "Recent", exact: true }).click();
  await expect(page.locator(".nest-recent")).toBeVisible();
  await page.locator(".nest-memory-toggle").click();
  await expect(page.locator(".nest-memory-toggle")).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator(".nest-memory")).toBeVisible();
});

test("nest: a new conversation waits with the composer in the middle", async ({ page }) => {
  await page.setViewportSize({ width: 1040, height: 760 });
  await page.goto("/lab/nest.html?still=1&tab=chat&fresh=1");
  const composer = (await page.locator(".nest-chat .composer").boundingBox())!;
  expect(composer.y).toBeLessThan(500);
  await expect(page.locator(".nest-chat .log .hint")).toBeVisible();
  await expect(page.locator(".chat-title")).toHaveText("New conversation");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-chat-fresh.png");
});

// The Nest's window has no system frame (ADR 0024): its page carries grips on every edge and corner
// that ask the compositor to resize it, and none in Full.
test("nest: the window's edges and corners resize it", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1");
  await expect(page.locator(".resize-edge")).toHaveCount(8);
  await expect(page.locator(".resize-edge.southeast")).toHaveCSS("cursor", "nwse-resize");
  const box = await page.locator(".resize-edge.east").boundingBox();
  expect(box!.x + box!.width).toBe(page.viewportSize()!.width);
  // Pressed in the lab, with no window behind it, nothing breaks.
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.locator(".resize-edge.east").dispatchEvent("mousedown", { button: 0 });
  expect(errors).toEqual([]);
});
