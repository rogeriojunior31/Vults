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

test("nest: last night's sleep shows its debrief and what it did", async ({ page }) => {
  await page.goto("/lab/nest.html?still=1&tab=memory");
  await expect(page.locator(".memory-night .memory-debrief")).toContainText("Windows CI fix is still open");
  await expect(page.locator(".memory-night .memory-night-line")).toHaveText(["2 repeated facts dropped.", "1 merges wait for you below."]);
});
