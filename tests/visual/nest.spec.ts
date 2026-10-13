import { expect, test } from "@playwright/test";

// The Nest's Chat tab (ui/lab/nest.html): full Markdown, and a card answered right there (ADR 0021).
test("nest: a reply in full Markdown, and a card answered in the Nest", async ({ page }) => {
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
  // The card is answered here, as on the island (ADR 0021).
  await page.locator(".ask").getByRole("button", { name: /^Allow/ }).scrollIntoViewIfNeeded();
  await expect(page.locator("#nest")).toHaveScreenshot("nest-chat.png");
  await page.locator(".ask").getByRole("button", { name: /^Allow/ }).click();
  await expect(page.locator("body")).toHaveAttribute("data-chat-decided", /allow$/);
  await expect(page.locator(".ask .settled-label")).toHaveText("Allowed");
});

test("nest: opened mid-turn, it shows the reply so far and the card waiting", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1&tab=chat&busy=1");
  // The reply so far, with no Copy yet: it is still being written.
  await expect(page.locator(".msg.zeca").last()).toContainText("Running the tests first");
  await expect(page.locator(".msg.zeca").last()).not.toHaveClass(/settled/);
  await expect(page.locator(".ask .code")).toHaveText("Bash · cargo test");
  await expect(page.locator(".ask").getByRole("button", { name: /^Allow/ })).toBeVisible();
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
  await vults.locator(".roost-part", { hasText: "Your pull requests" }).locator(".board-row.link").first().click();
  await expect(page.locator("body")).toHaveAttribute("data-opened", "pr:rogeriojunior31/Vults#243");
  await vults.getByRole("button", { name: "Default branch: Passing" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-opened", "branch:rogeriojunior31/Vults");
  // A roost with no GitHub has nothing to open.
  await expect(page.locator(".roost").last().locator("button.row-tag")).toHaveCount(0);
  await expect(page.locator(".roost").last().locator(".roost-part", { hasText: "Your pull requests" })).toHaveCount(0);
});

// The prototype's By project: the week's time per project, a search, sessions that lead to the Flock.
test("nest: by project has the week's bars, a search, and rows that lead to the session", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=projects");
  await expect(page.locator(".act-projects-head")).toContainText("2 projects · 3 worktrees");
  await expect(page.locator(".act-bar")).toHaveText([/^Vults · 2 wt.*9 h 40/, /^site.*2 h 38/]);
  await page.getByRole("searchbox", { name: "Find a project, branch or agent…" }).fill("m3-roosts");
  await expect(page.locator(".roost")).toHaveCount(1);
  await page.getByRole("searchbox", { name: "Find a project, branch or agent…" }).fill("nothing like it");
  await expect(page.locator(".roosts")).toContainText("No project matches.");
  await page.getByRole("button", { name: "Clear filters" }).click();
  await expect(page.getByRole("searchbox", { name: "Find a project, branch or agent…" })).toHaveValue("");
  await expect(page.locator(".roost")).toHaveCount(2);
  await page.locator(".roost").first().locator(".roost-part", { hasText: "Sessions" }).locator(".board-row").first().click();
  await expect(page.locator("body")).toHaveAttribute("data-focus", "b");
  await page.getByRole("button", { name: "Preferences ↗" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-settings", "projects");
});

// Over the other tabs, who needs you; a tag leads to that session on the Flock.
test("nest: the tabs but the Flock show who needs you", async ({ page }) => {
  await page.setViewportSize({ width: 1040, height: 760 });
  await page.goto("/lab/nest.html?still=1&tab=connections");
  await expect(page.locator(".nest-need .need-tag")).toHaveText(["!Vults-quick-wins"]);
  await expect(page.locator("#nest")).toHaveScreenshot("nest-need-strip.png");
  await page.locator(".nest-need .need-tag").click();
  await expect(page.locator("body")).toHaveAttribute("data-focus", "a");
  await expect(page.locator(".nest-need")).toBeHidden();
});

// A right click on a bird: the island's quick actions that work from the Room.
test("nest: a bird's right click opens its quick actions", async ({ page }) => {
  await page.setViewportSize({ width: 1040, height: 760 });
  await page.goto("/lab/nest.html?still=1");
  await expect(page.locator(".habitat-summary")).toHaveText("0 in flight · 4 on the wire");
  await page.locator(".roost-card", { hasText: "site" }).click({ button: "right" });
  await expect(page.locator(".nest-menu .menu-item")).toHaveText(["Open folder", "Keep in front", "Mute this project", "Pin this project", "Hide this project"]);
  await expect(page.locator("#nest")).toHaveScreenshot("nest-bird-menu.png");
  await page.getByRole("menuitem", { name: "Mute this project" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-pref", "d mute true");
  await expect(page.locator(".nest-menu")).toBeHidden();
  // From the keyboard too; Escape closes it.
  await page.locator(".roost-card", { hasText: "Vults-z6" }).focus();
  await page.keyboard.press("Shift+F10");
  await expect(page.locator(".nest-menu")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.locator(".nest-menu")).toBeHidden();
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
  await expect(first.locator(".task-status")).toHaveText("Doing");
  await expect(first.locator(".task-next")).toHaveText("Next: no sleep added");
  await expect(first.getByRole("combobox", { name: "Status for Fix the flaky login test" })).toBeVisible();
  await expect(first.getByRole("combobox", { name: "Owner for Fix the flaky login test" })).toBeVisible();
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
  await expect(page.locator(".history .records-head")).toHaveText(["Vults", "site"]);
  await expect(page.locator(".history .record").nth(1)).toContainText("Failed");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-history.png");
});

test("nest: usage, the plans and what the agents cost", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=cost");
  await expect(page.locator(".usage")).toContainText("against your $5.00 a day");
  await expect(page.locator(".usage-status.critical")).toHaveText("Near limit");
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
  await expect(page.locator(".audit .records-head")).toHaveText(["Vults", "site"]);
  await expect(page.locator(".audit")).toContainText("Policy accepted as #3");
  await expect(page.locator(".audit")).toContainText("Your phone");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-audit.png");
});

// The Flock tab (ADR 0021): each session with its roost and branch; its card answered there (ADR 0021).
test("nest: the flock, and a card answered in the Nest", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1");
  await expect(page.locator(".suite-tab.on .nav-label")).toHaveText("Flock");
  // L2: the stage, a roost card under each bird.
  await expect(page.locator(".roost-card")).toHaveCount(4);
  await expect(page.locator("#nest")).toHaveScreenshot("nest-flock.png");
  // The waiting one shows what it asks, and is answered here (ADR 0021).
  await page.locator(".roost-card").first().click();
  await expect(page.locator("body")).toHaveAttribute("data-focus", "a");
  const card = page.locator(".focus-block");
  await expect(card.locator(".card-words")).toHaveText("Run the test suite, then the linter");
  await expect(card.locator(".command code")).toHaveText("Bash · cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings");
  await expect(card.locator(".q-of")).toHaveText("1 of 2");
  await expect(card.getByRole("button", { name: "Always allow" })).toBeVisible();
  await card.getByRole("button", { name: /^Allow/ }).click();
  await expect(page.locator("body")).toHaveAttribute("data-decided", /allow$/);
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
  expect(await week.locator(".metric-grid").evaluate((metrics) => metrics.nextElementSibling?.classList.contains("lede"))).toBe(true);
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

test("nest: a connection's detail has the island's actions: merge, ask Zeca, ✓", async ({ page }) => {
  await page.setViewportSize({ width: 1040, height: 760 });
  await page.goto("/lab/nest.html?still=1&tab=connections");
  // Failing checks: ask Zeca why, in the Chat's composer.
  await page.locator(".conn-row", { hasText: "vults#56" }).click();
  await page.getByRole("button", { name: "Ask Zeca why" }).click();
  await expect(page.locator(".nest-chat textarea, .chat textarea").first()).toHaveValue(/^Why are the checks failing on vults#56/);
  await page.locator('.suite-tab[data-tab="connections"]').click();
  // The approved, green pull request: Squash and merge asks first.
  await page.locator(".conn-row", { hasText: "site#9" }).click();
  await page.getByRole("button", { name: "Squash and merge" }).click();
  await expect(page.locator(".conn-confirm")).toContainText("Squash site#9 into its base branch");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-connections-merge.png");
  await page.getByRole("button", { name: "Merge", exact: true }).click();
  await expect(page.locator("body")).toHaveAttribute("data-merged", "github pr:me/site#9");
  await expect(page.getByRole("button", { name: "Merged" })).toBeDisabled();
  // ✓ Seen dims the row here; it leaves Quick.
  await page.getByRole("button", { name: "✓ Seen" }).click();
  await expect(page.locator(".conn-row.seen")).toHaveCount(1);
  await expect(page.getByRole("button", { name: "✓ Seen" })).toHaveCount(0);
});

test("nest: connections show the GitHub card", async ({ page }) => {
  await page.setViewportSize({ width: 1040, height: 760 });
  await page.goto("/lab/nest.html?still=1&tab=connections");
  // L5: the connector's chip, its head (when it synced, Refresh), its rows by group, one in detail.
  await expect(page.locator(".conn-chip.on")).toContainText("GitHub");
  await expect(page.locator(".conn-head h2")).toHaveText("GitHub");
  await expect(page.locator(".conn-status")).toHaveText("Synced 2 min ago");
  // The prototype's tabs, each with its count: what waits on the user (new ones), the pull requests,
  // the default branches' Actions, the releases.
  await expect(page.locator(".nest-filter > span")).toHaveText(["Inbox", "Pull requests", "Actions", "Releases"]);
  await expect(page.locator(".nest-filter b")).toHaveText(["3 new", "3", "1", "1"]);
  await expect(page.locator(".conn-list .records-head")).toHaveText(["Review requests", "Your pull requests"]);
  await expect(page.locator(".conn-row").first()).toContainText("lib#7 · deps/serde");
  await expect(page.locator(".conn-detail")).toContainText("@bob");
  // A pull request in detail: its repository, branch, size, reviewers, age, each check, a command.
  await page.locator(".conn-row", { hasText: "vults#56" }).click();
  const detail = page.locator(".conn-detail");
  await expect(detail.locator(".conn-detail-head")).toContainText("me/vults#56");
  for (const fact of ["feat/github-card", "+120 −40 · 6 files", "@ana asked for changes", "2 h ago"]) await expect(detail).toContainText(fact);
  await expect(detail.locator(".conn-check")).toHaveText(["Typecheckpassed", "Unit testspassed", "Visual checksfailed"]);
  await expect(detail.locator(".conn-cmd code")).toHaveText("gh pr checks 56 --repo me/vults");
  await expect(page.locator(".conn-row", { hasText: "vults#56" }).locator(".conn-tag")).toHaveText("Changes requested");
  await page.getByRole("button", { name: "Open in GitHub ↗" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-opened", "pr:me/vults#56");
  await page.getByRole("button", { name: "↻ Refresh" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-refreshed", "1");
  await page.mouse.move(0, 0);
  await expect(page.locator("#nest")).toHaveScreenshot("nest-connections.png");
  // Copy puts the command on the clipboard; Vults never runs it.
  await page.context().grantPermissions(["clipboard-read", "clipboard-write"]);
  await detail.getByRole("button", { name: "Copy" }).click();
  await expect(detail.getByRole("button", { name: "Copied" })).toBeVisible();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe("gh pr checks 56 --repo me/vults");
  // Actions: the default branch, its checks one by one.
  await page.locator(".nest-filter", { hasText: "Actions" }).click();
  await expect(page.locator(".conn-detail h3")).toHaveText("main · Merge pull request #54");
  await expect(detail).toContainText("2 of 3 passed");
  await expect(detail.locator(".conn-check")).toHaveCount(3);
  await expect(detail.locator(".conn-cmd code")).toHaveText("gh run list --repo me/vults --branch main");
  // Releases: the latest one, when it was published.
  await page.locator(".nest-filter", { hasText: "Releases" }).click();
  await expect(page.locator(".conn-row .conn-tag")).toHaveText("Latest release");
  await expect(detail).toContainText("4 d ago");
  await expect(detail.locator(".conn-cmd code")).toHaveText("gh release view v0.1.7 --repo me/vults");
  await page.mouse.move(0, 0);
  await expect(page.locator("#nest")).toHaveScreenshot("nest-connections-release.png");
  // One repository only: its rows, and the tabs count them.
  await page.getByRole("combobox", { name: "Repository" }).selectOption("me/site");
  await page.locator(".nest-filter", { hasText: "Inbox" }).click();
  await expect(page.locator(".conn-row")).toHaveCount(1);
  await expect(page.locator(".conn-row")).toContainText("site#9");
  await expect(page.locator(".nest-filter b")).toHaveText(["1 new", "1", "0", "0"]);
  // GitHub is on: nothing left to add.
  await expect(page.locator(".nest-add")).toHaveCount(0);
});

test("nest: narrow connections return from a detail without losing its filter", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=connections");
  await page.getByRole("combobox", { name: "Repository" }).selectOption("me/vults");
  const row = page.locator(".conn-row", { hasText: "vults#56" });
  await row.click();
  await expect(page.locator(".conn-body")).toHaveClass(/detail-open/);
  await expect(page.locator(".conn-list")).toHaveCount(0);
  await page.getByRole("button", { name: "← Connections" }).click();
  await expect(page.getByRole("combobox", { name: "Repository" })).toHaveValue("me/vults");
  await expect(row).toBeFocused();
});

// A failed poll: the head says when it last worked and why, never "Synced"; the rows stay.
test("nest: a connector whose poll failed says so in its head", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1&tab=connections&stale=1");
  await expect(page.locator(".conn-status.stale")).toHaveText("Last updated 10 min ago · gh: not logged in");
  await expect(page.locator(".conn-row")).toHaveCount(3);
  // A tab with nothing in it for the repository says so.
  await page.getByRole("combobox", { name: "Repository" }).selectOption("team/lib");
  await page.locator(".nest-filter", { hasText: "Actions" }).click();
  await expect(page.locator(".conn-row")).toHaveCount(0);
  await expect(page.locator(".conn-list")).toHaveText("Nothing here for this repository.");
  await page.getByRole("button", { name: "Clear filters" }).click();
  await expect(page.getByRole("combobox", { name: "Repository" })).toHaveValue("");
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

// The card in front, as the prototype draws it: its steps and cost in the head, the ticker with the
// edit's +N −M (opening it in Changes), what it changed in all, and a quiet bird's three answers.
test("nest: the card in front has its ticker, cost, changes and quiet flag", async ({ page }) => {
  await page.setViewportSize({ width: 1040, height: 760 });
  await page.goto("/lab/nest.html?still=1");
  const card = page.locator(".focus-block .app-card");
  await expect(card.locator(".card-verb")).toHaveText(["12 steps", "≈$1.37"]);
  await expect(card.locator(".card-tick")).toHaveText(["✓ Searching Bird", "› Editing render.ts+78 −14"]);
  await expect(card.locator(".compact-detail")).toHaveText("+78 −141 edit");
  await page.locator(".detail-tabs").getByRole("tab", { name: "Activity" }).click();
  await card.locator(".diffbtn").click();
  await expect(page.locator(".detail-tabs").getByRole("tab", { name: "Changes" })).toHaveAttribute("aria-selected", "true");
  await expect(page.locator("#nest")).toHaveScreenshot("nest-flock-card.png");
  // Quiet for 15 minutes: Snooze, Keep going, Dismiss, none of them reaching the agent.
  await page.evaluate(() => {
    const { nest, view } = (window as any).nestTest;
    nest.view({ ...view, sessions: view.sessions.map((s: any) => (s.id === "b" ? { ...s, silent: "loud" } : s)) });
  });
  await expect(card.locator(".silence")).toContainText("No news for 15 minutes.");
  await card.getByRole("button", { name: "Keep going" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-hush", "b keep-going");
  // Finished: the last reply instead of a ticker.
  await page.evaluate(() => {
    const { nest, view } = (window as any).nestTest;
    nest.view({ ...view, sessions: view.sessions.map((s: any) => (s.id === "b" ? { ...s, status: "finished", note: "All 42 tests pass." } : s)) });
  });
  await expect(card.locator(".card-words")).toHaveText("All 42 tests pass.");
  await expect(card.locator(".card-ticker")).toHaveCount(0);
});

// The Flock (U7): the habitat, a search and Needs you, the one in front in detail.
test("nest: the flock's habitat, filters and details", async ({ page }) => {
  await page.setViewportSize({ width: 1040, height: 760 });
  await page.goto("/lab/nest.html?still=1");
  await expect(page.locator(".habitat-sky .habitat-bird")).toHaveCount(4);
  await expect(page.locator(".flock-toolbar h2")).toHaveText("Vults · Editing render.ts");
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
  // Each step at the time it started, the newest first, as the prototype writes it.
  await expect(page.locator(".activity-row time")).toHaveText(["14:24", "14:22", "14:21"]);
  await expect(page.locator(".activity-row > span")).toHaveText(["+78", "✓", "✓"]);
  await expect(page.locator(".details-pane")).toHaveScreenshot("nest-activity-steps.png");
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
  await expect(page.locator(".focus-block").getByRole("button", { name: /^Deny/ })).toBeVisible();
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

test("nest: the main tabs use roving focus", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1");
  const flock = page.getByRole("tab", { name: "Flock 1" });
  await expect(flock).toHaveAttribute("aria-controls", "nest-pane-flock");
  await expect(page.locator("#nest-pane-flock")).toHaveAttribute("aria-labelledby", "nest-tab-flock");
  await flock.focus();
  await page.keyboard.press("ArrowRight");
  await expect(page.getByRole("tab", { name: "Tasks" })).toBeFocused();
  await expect(page.getByRole("tab", { name: "Tasks" })).toHaveAttribute("aria-selected", "true");
  await page.keyboard.press("End");
  await expect(page.getByRole("tab", { name: "Settings" })).toBeFocused();
  await page.keyboard.press("Home");
  await expect(flock).toBeFocused();
  await expect(flock).toHaveAttribute("aria-selected", "true");
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
  await expect(page.locator(".usage-card .meter.amber")).toHaveText("7d 74% · Warning");
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
  await expect(page.locator('.suite-tab[data-tab="connections"] .nav-count')).toHaveText("4");
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
  await expect(page.getByRole("tab", { name: "Connections 4" })).toBeVisible();
  await expect(page.locator(".session-count")).toBeHidden();
});

test("nest: a short chat keeps its composer above the footer", async ({ page }) => {
  await page.setViewportSize({ width: 480, height: 360 });
  await page.goto("/lab/nest.html?still=1&tab=chat");
  const composer = (await page.locator(".nest-chat .composer").boundingBox())!;
  const footer = (await page.locator(".app-footer").boundingBox())!;
  expect(composer.y + composer.height).toBeLessThanOrEqual(footer.y);
  await expect(page.locator(".nest-chat .composer .send")).toBeInViewport();
});

test("nest: a full-height chat leaves two lines for a draft", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 640 });
  await page.goto("/lab/nest.html?still=1&tab=chat");
  const draft = await page.locator(".nest-chat textarea").boundingBox();
  expect(draft?.height).toBeGreaterThanOrEqual(44);
});

test("nest: a short flock shows the pending decision first", async ({ page }) => {
  await page.setViewportSize({ width: 480, height: 360 });
  await page.goto("/lab/nest.html?still=1");
  await page.evaluate(() => {
    const test = (window as typeof window & { nestTest: { nest: { view: (v: unknown) => void }; view: { focus: { agent: string; id: string } } } }).nestTest;
    test.view.focus = { agent: "claude", id: "a" };
    test.nest.view(test.view);
  });
  await expect(page.locator(".flock-stage")).toBeHidden();
  await expect(page.locator(".focus-block").getByRole("button", { name: /^Allow/ })).toBeInViewport();
});

test("nest: narrow memory returns to the chat and keeps the draft", async ({ page }) => {
  await page.setViewportSize({ width: 480, height: 360 });
  await page.goto("/lab/nest.html?still=1&tab=chat");
  const draft = page.locator(".nest-chat textarea");
  await draft.fill("keep this draft");
  await page.locator(".nest-memory-toggle").click();
  await expect(page.getByRole("button", { name: "Back to Chat" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.locator(".nest-memory-toggle")).toHaveAttribute("aria-pressed", "false");
  await expect(page.locator(".nest-memory-toggle")).toBeFocused();
  await expect(draft).toHaveValue("keep this draft");
});

// L3: the Chat's title bar (the conversations' column, the title, the chat's pickers, the Memory
// panel) and, before anything is said, the composer in the middle.
test("nest: the chat's title bar and its panels", async ({ page }) => {
  await page.setViewportSize({ width: 1040, height: 760 });
  await page.goto("/lab/nest.html?still=1&tab=chat");
  await expect(page.locator(".chat-title")).toHaveText("A quieter island");
  await expect(page.locator(".chat-titlebar .chat-head")).toBeVisible();
  await expect(page.getByRole("button", { name: "Memory", exact: true })).toBeVisible();
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

// The Nest's window has no system frame (ADR 0021): its page carries grips on every edge and corner
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

// The prototype's Room chat: the engine pill, as on the island, and a paperclip; the song and the
// moon by the header's switch.
test("nest: the chat has the engine pill and a paperclip; the header has the song and the moon", async ({ page }) => {
  await page.setViewportSize({ width: 1040, height: 760 });
  await page.goto("/lab/nest.html?still=1&tab=chat&music=1");
  await page.locator(".nest-chat .engine-box .place.model").click();
  // The engines as the prototype groups them, each saying how it stands.
  const menu = page.locator(".nest-chat .engine-box .model-menu");
  await expect(menu.locator(".menu-label").first()).toHaveText("Your setup");
  await expect(menu.locator(".engine-state")).toHaveText(["Signed in", "Not installed"]);
  await expect(menu.locator(".engine-state.off")).toHaveText("Not installed");
  await page.mouse.move(0, 0);
  await expect(menu).toHaveScreenshot("nest-chat-engines.png");
  await menu.getByRole("button", { name: "Engines in Settings ›" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-chat-settings", "1");
  await expect(menu).toBeHidden();
  await page.locator(".nest-chat .composer .attach").click();
  await expect(page.locator("body")).toHaveAttribute("data-picked", "1");
  await expect(page.locator(".hdr-extras .now-playing")).toContainText("Paranoid Android · Radiohead");
  await page.getByRole("button", { name: "Pause" }).click();
  await expect(page.locator("body")).toHaveAttribute("data-media", "playpause");
  await page.evaluate(() => {
    const { nest, view } = (window as any).nestTest;
    nest.view({ ...view, dnd: true });
  });
  await page.getByRole("button", { name: /^Do not disturb is on/ }).click();
  await expect(page.locator("body")).toHaveAttribute("data-dnd", "off");
  await expect(page.locator(".app-header")).toHaveScreenshot("nest-header-extras.png");
});
