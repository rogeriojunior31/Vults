import { expect, test, type Page } from "@playwright/test";

// The app draws with WebKitGTK, not Chromium: the screens most likely to differ, in WebKit, at the
// sizes the real windows have. (The rest of the suite runs in Chromium.)
test.use({ browserName: "webkit" });

const lab = (params: string) => `/lab/?still=1&t=1500&${params}`;

/** Every element of the chat's head stays inside it, and none overlaps another (it may wrap). */
async function headFits(page: Page): Promise<void> {
  const head = page.locator(".chat-head");
  const box = (await head.boundingBox())!;
  const seen: { x: number; y: number; width: number; height: number }[] = [];
  for (const child of await head.locator(":scope > *:visible").all()) {
    const b = (await child.boundingBox())!;
    expect(b.x).toBeGreaterThanOrEqual(box.x - 0.5);
    expect(b.x + b.width).toBeLessThanOrEqual(box.x + box.width + 0.5);
    for (const o of seen) {
      const apart = b.x >= o.x + o.width - 0.5 || o.x >= b.x + b.width - 0.5 || b.y >= o.y + o.height - 0.5 || o.y >= b.y + b.height - 0.5;
      expect(apart).toBe(true);
    }
    seen.push(b);
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
  /** The tabs and the gear stay inside the header; nothing runs past the window. */
  const fits = async (page: import("@playwright/test").Page, card: string) => {
    const head = (await page.locator(".nest-head").boundingBox())!;
    for (const b of [page.locator('.nest-tab[data-tab="connections"]'), page.locator(".nest-gear")]) {
      const box = (await b.boundingBox())!;
      expect(box.x + box.width).toBeLessThanOrEqual(head.x + head.width);
    }
    const nest = (await page.locator("#nest").boundingBox())!;
    expect(nest.height).toBeLessThanOrEqual(h);
    const c = (await page.locator(card).first().boundingBox())!;
    expect(c.x + c.width).toBeLessThanOrEqual(w);
  };

  test(`webkit: the Nest's chat at ${w}×${h}`, async ({ page }) => {
    await page.setViewportSize({ width: w, height: h });
    await page.goto("/lab/nest.html?still=1&tab=chat");
    await expect(page.locator(".composer")).toBeInViewport();
    await headFits(page);
    await fits(page, ".composer");
    await expect(page.locator("#nest")).toHaveScreenshot(`webkit-nest-${w}x${h}.png`);
  });

  test(`webkit: the Nest's flock at ${w}×${h}`, async ({ page }) => {
    await page.setViewportSize({ width: w, height: h });
    await page.goto("/lab/nest.html?still=1");
    await fits(page, ".nest-session");
    await expect(page.locator(".nest-foot")).toBeInViewport();
    await expect(page.locator("#nest")).toHaveScreenshot(`webkit-nest-flock-${w}x${h}.png`);
  });

  test(`webkit: the Nest's tasks at ${w}×${h}`, async ({ page }) => {
    await page.setViewportSize({ width: w, height: h });
    await page.goto("/lab/nest.html?still=1&tab=tasks");
    await fits(page, ".task");
    await expect(page.locator("#nest")).toHaveScreenshot(`webkit-nest-tasks-${w}x${h}.png`);
  });

  test(`webkit: the Nest's memory at ${w}×${h}`, async ({ page }) => {
    await page.setViewportSize({ width: w, height: h });
    await page.goto("/lab/nest.html?still=1&tab=chat&memory=1");
    await fits(page, ".proposal");
    await expect(page.locator("#nest")).toHaveScreenshot(`webkit-nest-memory-${w}x${h}.png`);
  });

  test(`webkit: the Nest's cost at ${w}×${h}`, async ({ page }) => {
    await page.setViewportSize({ width: w, height: h });
    await page.goto("/lab/nest.html?still=1&tab=cost");
    // Activity's sub-tabs wrap inside it.
    const subs = (await page.locator(".nest-subs").boundingBox())!;
    const audit = (await page.locator('.nest-sub[data-sub="audit"]').boundingBox())!;
    expect(audit.x + audit.width).toBeLessThanOrEqual(subs.x + subs.width);
    await fits(page, ".record");
    await expect(page.locator("#nest")).toHaveScreenshot(`webkit-nest-usage-${w}x${h}.png`);
  });

  test(`webkit: the Nest's week at ${w}×${h}`, async ({ page }) => {
    await page.setViewportSize({ width: w, height: h });
    await page.goto("/lab/nest.html?still=1&tab=week");
    await fits(page, ".nest-week");
    await expect(page.locator("#nest")).toHaveScreenshot(`webkit-nest-week-${w}x${h}.png`);
  });

  test(`webkit: the Nest's roosts at ${w}×${h}`, async ({ page }) => {
    await page.setViewportSize({ width: w, height: h });
    await page.goto("/lab/nest.html?still=1&tab=projects");
    await fits(page, ".roost");
    await expect(page.locator("#nest")).toHaveScreenshot(`webkit-nest-roosts-${w}x${h}.png`);
  });
}

test("webkit: a card after the conversation read from outside warns", async ({ page }) => {
  await page.goto(lab("state=chat-permission&open=1&outside=1"));
  await expect(page.locator(".ask-outside")).toContainText("read content from outside (WebFetch)");
  await expect(page.locator("#island")).toHaveScreenshot("webkit-chat-permission-outside.png");
});
