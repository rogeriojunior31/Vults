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

test("a finished session celebrates on its perch, then joins the flock", async ({ page }) => {
  await page.clock.install();
  await page.goto("/lab/flight/");
  await page.evaluate(async () => {
    const path = "/src/island/sky.ts";
    const { Sky } = await import(path);
    const sky = new Sky(() => {});
    document.body.append(sky.canvas);
    // It was working on its perch, then finished.
    const session = { id: "a", agent: "claude", project: "a", cwd: null, status: "working", activity: null,
      step: null, steps: [], step_count: 0, subagents: 0, note: null, editor: null };
    sky.place(new Map([["claude:a", { x: 200, y: 24, scale: 1 }]]), 38);
    sky.update([session], true);
    sky.update([{ ...session, status: "finished", note: "Done." }], true);
    Object.assign(window, { finishedTest: { sky } });
  });
  const owns = () => page.evaluate(() => (window as any).finishedTest.sky.owns("claude:a"));
  // The done clip and the signature play on the perch first (the black vulture's: 5.64 s), then
  // a second's rest: no take-off before 6.6 s.
  await page.clock.runFor(6000);
  expect(await owns()).toBe(false);
  await page.clock.runFor(2000);
  expect(await owns()).toBe(true);
});

test("each running subagent sends out a scout, up to three a session and six in all", async ({ page }) => {
  await page.clock.install();
  await page.goto("/lab/flight/");
  await page.evaluate(async () => {
    const path = "/src/island/sky.ts";
    const { Sky } = await import(path);
    const sky = new Sky(() => {});
    document.body.append(sky.canvas);
    const session = (id: string, subagents: number) => ({ id, agent: "claude", project: id, cwd: null, status: "working",
      activity: "subagent", step: null, steps: [], step_count: 0, subagents, note: null, editor: null, species: "atratus" });
    sky.place(new Map([["claude:a", { x: 200, y: 24, scale: 1 }], ["claude:b", { x: 300, y: 24, scale: 1 }]]), { left: 0, top: 0, width: 720, height: 200, radius: 14 });
    Object.assign(window, { scoutTest: { sky, session } });
  });
  const set = (a: number, b: number) => page.evaluate(([a, b]) => {
    const { sky, session } = (window as any).scoutTest;
    sky.update([session("a", a), session("b", b)], true);
    return sky.scouting().live;
  }, [a, b]);
  const all = () => page.evaluate(() => (window as any).scoutTest.sky.scouting().all);
  expect(await set(2, 0)).toBe(2);
  expect(await set(5, 0)).toBe(3);
  expect(await set(5, 5)).toBe(6);
  await page.clock.runFor(2000);
  // Subagents end: their scouts fly off, and are gone once out of sight.
  expect(await set(1, 0)).toBe(1);
  expect(await all()).toBe(6);
  await page.clock.runFor(8000);
  expect(await all()).toBe(1);
  // The island folds into the thin pill: the last scout flies off too.
  await page.evaluate(() => (window as any).scoutTest.sky.place(new Map([["claude:a", { x: 200, y: 24, scale: 1 }]]),
    { left: 0, top: 0, width: 360, height: 38, radius: 14 }));
  expect(await set(1, 0)).toBe(0);
});

test("a rare visitor rides the thermal once and goes, never landing", async ({ page }) => {
  await page.clock.install();
  await page.goto("/lab/flight/");
  await page.evaluate(async () => {
    const path = "/src/island/sky.ts", clock = "/src/clock.ts";
    const { Sky } = await import(path), { Clock } = await import(clock);
    const sky = new Sky(() => {});
    document.body.append(sky.canvas);
    const session = { id: "a", agent: "claude", project: "a", cwd: null, status: "working", activity: "edit",
      step: null, steps: [], step_count: 0, subagents: 0, note: null, editor: null, species: "atratus" };
    sky.place(new Map([["claude:a", { x: 200, y: 24, scale: 1 }]]), { left: 0, top: 0, width: 720, height: 200, radius: 14 });
    sky.update([session], true);
    sky.visit(Clock.now());
    Object.assign(window, { visitTest: { sky, Clock } });
  });
  const visiting = () => page.evaluate(() => (window as any).visitTest.sky.visiting());
  expect(await visiting()).toBe(true);
  await page.clock.runFor(8000);
  expect(await visiting()).toBe(true);
  // One lap, then off past the edge.
  await page.clock.runFor(10000);
  expect(await visiting()).toBe(false);
  // Turned off in the settings: a visitor in the sky heads off at once.
  await page.evaluate(() => { const { sky, Clock } = (window as any).visitTest; sky.visit(Clock.now()); sky.setVisitors(false); });
  await page.clock.runFor(4000);
  expect(await visiting()).toBe(false);
});
