import { expect, test, type Page } from "@playwright/test";

// The app draws with WebKitGTK, not Chromium: the screens most likely to differ, in WebKit, at the
// sizes the real windows have. (The rest of the suite runs in Chromium.)
test.use({ browserName: "webkit" });

const lab = (params: string) => `/lab/?still=1&t=1500&${params}`;

/** Every element of the chat's head stays inside it, after the one before it. */
async function headFits(page: Page): Promise<void> {
  const head = page.locator(".chat-head");
  const box = (await head.boundingBox())!;
  let right = box.x;
  for (const child of await head.locator(":scope > *:visible").all()) {
    const b = (await child.boundingBox())!;
    expect(b.x).toBeGreaterThanOrEqual(right - 0.5);
    expect(b.x + b.width).toBeLessThanOrEqual(box.x + box.width + 0.5);
    right = b.x + b.width;
  }
}

test("webkit: the chat's head with a long model and folder never overlaps", async ({ page }) => {
  await page.goto(lab("state=chat&open=1"));
  await page.getByRole("button", { name: "Default model" }).click();
  await page.locator(".model-menu").getByRole("button", { name: "Sonnet" }).click();
  await page.locator(".model-menu").getByRole("button", { name: "Extra high" }).click();
  await page.locator(".place.model").click();
  await page.evaluate(() => {
    const folder = document.querySelector(".place:not(.model) span");
    if (folder) folder.textContent = "in Vults-q2-agent-env-with-a-long-name";
  });
  await headFits(page);
  await expect(page.locator("#island")).toHaveScreenshot("webkit-chat-head-long.png");
});

test("webkit: the chat's menus", async ({ page }) => {
  await page.goto(lab("state=chat&open=1"));
  await page.getByRole("button", { name: "Default model" }).click();
  await expect(page.locator("#island")).toHaveScreenshot("webkit-chat-model-menu.png");
  await page.getByRole("button", { name: "Conversations" }).click();
  await expect(page.locator("#island")).toHaveScreenshot("webkit-chat-conversations.png");
});

test("webkit: the chat's card", async ({ page }) => {
  await page.goto(lab("state=chat-permission&open=1"));
  await expect(page.locator("#island")).toHaveScreenshot("webkit-chat-permission.png");
});

// The Nest opens at 760×640 and may be made as small as 480×360 (app/src/lib.rs, open_nest).
for (const [w, h] of [
  [760, 640],
  [480, 360],
]) {
  test(`webkit: the Nest at ${w}×${h}`, async ({ page }) => {
    await page.setViewportSize({ width: w, height: h });
    await page.goto("/lab/nest.html?still=1");
    // The whole window is used, and nothing spills past it.
    const nest = (await page.locator("#nest").boundingBox())!;
    expect(nest.height).toBeLessThanOrEqual(h);
    await expect(page.locator(".composer")).toBeInViewport();
    await headFits(page);
    await expect(page.locator("#nest")).toHaveScreenshot(`webkit-nest-${w}x${h}.png`);
  });
}

test("webkit: a card after the conversation read from outside warns", async ({ page }) => {
  await page.goto(lab("state=chat-permission&open=1&outside=1"));
  await expect(page.locator(".ask-outside")).toContainText("read content from outside (WebFetch)");
  await expect(page.locator("#island")).toHaveScreenshot("webkit-chat-permission-outside.png");
});
