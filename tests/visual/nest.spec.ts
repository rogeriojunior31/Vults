import { expect, test } from "@playwright/test";

// The Nest's Chat tab (ui/lab/nest.html): full Markdown, and a card only the island answers.
test("nest: a reply in full Markdown, and a card sent to the island", async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 900 });
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
