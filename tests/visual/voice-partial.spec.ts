import { expect, test } from "@playwright/test";

const lab = (params: string) => `/lab/?still=1&t=1500&${params}`;

// The words heard so far, dimmed beside the waveform: its end in view while listening, and kept
// while the final transcription runs.
for (const state of ["listening", "transcribing"]) {
  test(`island open: the chat ${state}, with the words so far`, async ({ page }) => {
    await page.goto(lab(`state=chat&open=1&voice=${state}&partial=1`));
    const partial = page.locator("#island .wave em.partial");
    await expect(partial).toContainText("voice partials");
    await expect(page.locator("#island .composer textarea")).toBeHidden();
    await expect(page.locator("#island")).toHaveScreenshot(`island-chat-${state}-partial.png`);
  });
}

test("the final text replaces the partial and goes to the input", async ({ page }) => {
  await page.goto(lab("state=chat&open=1"));
  const mic = page.locator("#island .composer .mic");
  await mic.click();
  // The lab's fake mic sends the words so far every 800 ms.
  await expect(page.locator("#island .wave em.partial")).toContainText("Why is", { timeout: 3000 });
  await mic.click();
  const input = page.locator("#island .composer textarea");
  await expect(input).toHaveValue("Why is the build failing on the release branch?");
  await expect(page.locator("#island .wave em.partial")).toHaveCount(0);
});
