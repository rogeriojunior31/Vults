import { expect, test } from "@playwright/test";

test("flight joins preserve position and velocity, including an interrupted climb", async ({ page }) => {
  await page.goto("/lab/flight/");
  const result = await page.evaluate(async () => {
    const director = "/src/character/director.ts", sprites = "/src/character/zeca/index.ts";
    const { Bird } = await import(director), { ZECA } = await import(sprites);
    const perch = { x: 30, wireY: 116, height: 20, skyRight: 230, skyTop: 10 };
    const th = { cx: 128, cy: 46, rx: 70, ry: 20, lapMs: 6200, phase: Math.PI };
    const bird = new Bird(ZECA, perch);
    bird.soar(th, 0);
    const a = bird.shot(1799), b = bird.shot(1800), c = bird.shot(1801);
    const continuity = Math.hypot((b.x - a.x) - (c.x - b.x), (b.y - a.y) - (c.y - b.y));
    const landings = [1000, 8000].map(t => {
      const bird = new Bird(ZECA, { ...perch }); bird.soar(th, 0);
      const before = bird.shot(t);
      bird.want("run", t);
      const after = bird.shot(t);
      const end = bird.shot(t + 5000);
      return { jump: Math.hypot(after.x - before.x, after.y - before.y), flying: bird.isFlying(), x: end.x, y: end.y };
    });
    bird.want("approval", 8000);
    bird.shot(8800);
    return { continuity, landings, urgentLanded: !bird.isFlying() };
  });
  expect(result.continuity).toBeLessThan(0.001);
  for (const landing of result.landings) {
    expect(landing.jump).toBeLessThan(0.001);
    expect(landing.flying).toBe(false);
    expect(landing.x).toBe(30);
    expect(landing.y).toBe(96);
  }
  expect(result.urgentLanded).toBe(true);
});

test("idle flock persists; only working sessions land; reduced motion restores perches", async ({ page }) => {
  await page.clock.install();
  await page.goto("/lab/flight/");
  await page.evaluate(async () => {
    const path = "/src/island/sky.ts";
    const { Sky } = await import(path);
    const sky = new Sky(() => {});
    document.body.append(sky.canvas);
    const sessions = ["a", "b", "c"].map(id => ({ id, agent: "claude", project: id, cwd: null,
      status: "idle", activity: null, step: null, steps: [], step_count: 0, subagents: 0, note: null }));
    sky.place(new Map(sessions.map((s, i) => [`claude:${s.id}`, { x: 200 + i * 40, y: 24, scale: 1 }])), 38);
    sky.update(sessions, true);
    Object.assign(window, { flockTest: { sky, sessions } });
  });
  const owns = () => page.evaluate(() => {
    const { sky } = (window as any).flockTest;
    return ["a", "b", "c"].map(id => sky.owns(`claude:${id}`));
  });
  await page.clock.runFor(5000);
  expect(await owns()).toEqual([true, true, true]);
  // Jump elapsed time without simulating thousands of frames: there is no five-minute roost.
  await page.clock.fastForward(6 * 60_000);
  expect(await owns()).toEqual([true, true, true]);
  await page.evaluate(() => {
    const { sky, sessions } = (window as any).flockTest;
    sessions[1] = { ...sessions[1], status: "working", activity: "edit" };
    sky.update(sessions, true);
  });
  await page.clock.runFor(5000);
  expect(await owns()).toEqual([true, false, true]);
  await page.evaluate(() => {
    const { sky, sessions } = (window as any).flockTest;
    sessions[1] = { ...sessions[1], status: "idle" };
    sky.update(sessions, true);
  });
  await page.clock.runFor(4000);
  expect(await owns()).toEqual([true, true, true]);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect.poll(owns).toEqual([false, false, false]);
});

test("opening the island keeps the flock up", async ({ page }) => {
  await page.clock.install();
  await page.goto("/lab/?island=8");
  await page.clock.runFor(5000);
  const canvas = page.locator(".flock-sky");
  const pixels = () => canvas.evaluate((node: HTMLCanvasElement) => node.toDataURL());
  const before = await pixels();
  await page.locator("#island .compact").click();
  await page.clock.runFor(3000);
  await expect(page.locator("#island")).toHaveClass(/is-open/);
  expect(await pixels()).not.toEqual(before);
  expect(await canvas.evaluate((node: HTMLCanvasElement) => {
    const data = node.getContext("2d")!.getImageData(0, 0, node.width, node.height).data;
    return data.some((n, i) => i % 4 === 3 && n > 0);
  })).toBe(true);
  await page.screenshot({ path: "/tmp/vultures-flock-open.png" });
});

test("work takes focus from idle sessions and preserves manual selection", async ({ page }) => {
  await page.goto("/lab/?still=1&island=8");
  await page.evaluate(async () => {
    const path = "/src/island/render.ts";
    const { createIsland } = await import(path);
    const noop = () => {};
    document.body.replaceChildren();
    const root = document.createElement("main"); root.id = "island"; document.body.append(root);
    const app = createIsland(root, { decide: noop, decideAlways: noop, layout: noop, openAlert: noop,
      jump: noop, openSettings: noop, setSounds: noop, dismissAlert: noop,
      chat: { send: async () => {}, decide: noop, stop: noop, reset: noop, keyboard: noop } });
    const sessions = ["a", "b", "c"].map(id => ({ id, agent: "claude", project: `project-${id}`, cwd: null,
      status: "idle", activity: null, step: null, steps: [], step_count: 0, subagents: 0, note: null }));
    app.render({ sessions, approval: null, alerts: [] });
    Object.assign(window, { focusTest: { app, sessions } });
  });
  await expect(page.locator(".pill-text .name")).toHaveText("project-a");
  await page.evaluate(() => {
    const { app, sessions } = (window as any).focusTest;
    sessions[2] = { ...sessions[2], status: "working", activity: "run" };
    app.render({ sessions, approval: null, alerts: [] });
  });
  await expect(page.locator(".pill-text .name")).toHaveText("project-c");
  await page.locator(".compact").click();
  await page.locator(".flock-row").filter({ hasText: "project-a" }).click();
  await page.evaluate(() => {
    const { app, sessions } = (window as any).focusTest;
    sessions[1] = { ...sessions[1], status: "working", activity: "edit" };
    app.render({ sessions, approval: null, alerts: [] });
  });
  await expect(page.locator(".pill-text .name")).toHaveText("project-a");
});
