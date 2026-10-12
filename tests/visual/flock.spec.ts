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
  await page.goto("/lab/?state=idle-flock");
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

test("work does not take the front, so birds keep their places; a manual selection holds", async ({ page }) => {
  await page.goto("/lab/?still=1&state=idle-flock");
  await page.evaluate(async () => {
    const path = "/src/island/render.ts";
    const { createIsland } = await import(path);
    const noop = () => {};
    document.body.replaceChildren();
    const root = document.createElement("main"); root.id = "island"; document.body.append(root);
    const app = createIsland(root, { decide: noop, decideAlways: noop, layout: noop, openAlert: noop,
      jump: noop, openSettings: noop, opened: noop, setSounds: noop, dismissAlert: noop,
      chat: { send: async () => {}, decide: noop, stop: noop, reset: noop, keyboard: noop, models: async () => [], pick: async () => ({}), setPick: async () => {}, conversations: async () => [], open: async () => { throw new Error(); }, rename: async () => {}, remove: async () => false } });
    const sessions = ["a", "b", "c"].map(id => ({ id, agent: "claude", project: `project-${id}`, cwd: null,
      status: "idle", activity: null, step: null, steps: [], step_count: 0, subagents: 0, note: null }));
    app.render({ sessions, approval: null, alerts: [] });
    Object.assign(window, { focusTest: { app, sessions } });
  });
  await expect(page.locator(".compact")).toHaveAttribute("aria-label", /^project-a/);
  await page.evaluate(() => {
    const { app, sessions } = (window as any).focusTest;
    sessions[2] = { ...sessions[2], status: "working", activity: "run" };
    app.render({ sessions, approval: null, alerts: [] });
  });
  await expect(page.locator(".compact")).toHaveAttribute("aria-label", /^project-a/);
  await page.locator(".compact").click();
  await page.locator(".habitat-bird").filter({ hasText: "project-b" }).click();
  await page.evaluate(() => {
    const { app, sessions } = (window as any).focusTest;
    sessions[0] = { ...sessions[0], status: "working", activity: "edit" };
    app.render({ sessions, approval: null, alerts: [] });
  });
  await expect(page.locator(".compact")).toHaveAttribute("aria-label", /^project-b/);
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
    sky.visit();
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
  await page.evaluate(() => { const { sky } = (window as any).visitTest; sky.visit(); sky.setVisitors(false); });
  await page.clock.runFor(4000);
  expect(await visiting()).toBe(false);
});

test("visitors come on their own every 10 to 20 minutes, and never when there is nothing to visit", async ({ page }) => {
  await page.clock.install();
  await page.goto("/lab/flight/");
  await page.evaluate(async () => {
    const path = "/src/island/sky.ts";
    const { Sky } = await import(path);
    const sky = new Sky(() => {});
    document.body.append(sky.canvas);
    const session = { id: "a", agent: "claude", project: "a", cwd: null, status: "working", activity: "edit",
      step: null, steps: [], step_count: 0, subagents: 0, note: null, editor: null, species: "atratus" };
    sky.place(new Map([["claude:a", { x: 200, y: 24, scale: 1 }]]), { left: 0, top: 0, width: 720, height: 200, radius: 14 });
    Object.assign(window, { schedTest: { sky, session } });
  });
  const visiting = () => page.evaluate(() => (window as any).schedTest.sky.visiting());
  // The sessions leave and come back with `Math.random` pinned, so the next visit is due at a known
  // instant. Waiting for a random one meant drawing up to ten minutes of frames: past the time limit.
  const arrive = async (setup: string, random: number) => {
    // The last scenario's visitor, if any, heads off and is gone first.
    await page.clock.fastForward(30_000);
    await page.clock.fastForward(30_000);
    await page.evaluate(([setup, random]) => {
      const { sky, session } = (window as any).schedTest;
      Math.random = () => random as number;
      document.body.classList.toggle("still", setup === "calm");
      sky.setVisitors(setup !== "off");
      sky.update([], true);
      sky.update(setup === "empty" ? [] : [session], true);
    }, [setup, random] as const);
    expect(await visiting()).toBe(false);
  };
  const update = () => page.evaluate(() => { const { sky, session } = (window as any).schedTest; sky.update([session], true); });
  // Both ends of the 10 to 20 minutes: nobody 1.5 s before the visit is due, a visitor 1.5 s after,
  // as a visible sky draws at least once a second. The 1.5 s also covers the real time the clock
  // runs between calls.
  for (const random of [0, 0.9999]) {
    await arrive("on", random);
    await page.clock.fastForward(10 * 60_000 + random * 10 * 60_000 - 1500);
    expect(await visiting(), `random ${random}`).toBe(false);
    await page.clock.runFor(3000);
    expect(await visiting(), `random ${random}`).toBe(true);
  }
  // Turned off: nobody when the visit is due.
  await arrive("off", 0);
  await page.clock.fastForward(10 * 60_000 - 1500);
  await page.clock.runFor(3000);
  expect(await visiting(), "off").toBe(false);
  // Nothing to visit: no visit is set, so a session that comes 5 minutes later waits its own 10.
  await arrive("empty", 0);
  await page.clock.fastForward(5 * 60_000);
  await update();
  await page.clock.fastForward(5 * 60_000 - 1500);
  await page.clock.runFor(3000);
  expect(await visiting(), "empty").toBe(false);
  // Less motion, then motion comes back (or the island shows again): an overdue visit is not made up.
  await arrive("calm", 0);
  await page.clock.fastForward(25 * 60_000);
  await page.evaluate(() => document.body.classList.remove("still"));
  await update();
  await page.clock.runFor(1500);
  expect(await visiting(), "calm").toBe(false);
});

/** A scene of the pill with one session (`claude:a`) and Zeca, driven by the page's clock. */
async function wire(page: import("@playwright/test").Page) {
  await page.clock.install();
  await page.goto("/lab/flight/");
  await page.evaluate(async () => {
    const scenePath = "/src/island/scene.ts", flockPath = "/src/character/flock/index.ts", islandFlock = "/src/island/flock.ts";
    const { Scene, COMPACT_SCENE } = await import(scenePath);
    const { jitter } = await import(islandFlock);
    const { speciesSet } = await import(flockPath);
    // No fly-in: the bird is on the wire from the start.
    const scene = new Scene({ ...COMPACT_SCENE, flights: false });
    document.body.append(scene.canvas);
    const session = (status: string, activity: string | null = null) => ({ id: "a", agent: "claude", project: "a", cwd: null, status,
      activity, step: null, steps: [], step_count: 0, subagents: 0, note: null, editor: null, species: "atratus" });
    const length = (clip: string) => speciesSet("atratus").clips[clip].frames.reduce((n: number, f: { ms: number }) => n + f.ms, 0);
    Object.assign(window, { wireTest: { scene, session, length, nap: 90_000 * jitter("claude:a") } });
  });
  return {
    update: (status: string, activity: string | null = null) =>
      page.evaluate(([status, activity]) => { const { scene, session } = (window as any).wireTest; scene.update([session(status, activity)], null); }, [status, activity]),
    playing: () => page.evaluate(() => (window as any).wireTest.scene.playing("claude:a")),
    length: (clip: string) => page.evaluate((clip) => (window as any).wireTest.length(clip), clip),
    /** Jumps past the bird's own nap time (90 s, ±12% by its key); due timers fire once. */
    nap: async () => page.clock.fastForward(Math.ceil(await page.evaluate(() => (window as any).wireTest.nap)) + 100),
  };
}

test("a resting bird dozes off before it sleeps, and work wakes it with a stretch", async ({ page }) => {
  const w = await wire(page);
  await w.update("idle");
  expect(await w.playing()).toBe("idle");
  await w.nap();
  expect(await w.playing()).toBe("doze");
  await page.clock.runFor(await w.length("doze"));
  expect(await w.playing()).toBe("sleep");
  await w.update("working", "edit");
  expect(await w.playing()).toBe("wake");
  await page.clock.runFor((await w.length("wake")) + 50);
  expect(await w.playing()).toBe("edit");
});

test("web work wakes a sleeping bird with a stretch, then it takes off", async ({ page }) => {
  const w = await wire(page);
  await w.update("idle");
  await w.nap();
  await page.clock.runFor(await w.length("doze"));
  await w.update("working", "web");
  expect(await w.playing()).toBe("wake");
  await page.clock.runFor((await w.length("wake")) + 50);
  expect(await w.playing()).toBe("fly");
});

test("a permission wakes a sleeping bird at once, with no stretch first", async ({ page }) => {
  const w = await wire(page);
  await w.update("idle");
  await w.nap();
  await page.clock.runFor(await w.length("doze"));
  expect(await w.playing()).toBe("sleep");
  await w.update("approval");
  expect(await w.playing()).toBe("approval");
});

test("with a song playing, idle birds court on the wire and stay out of the sky", async ({ page }) => {
  await page.clock.install();
  await page.goto("/lab/flight/");
  await page.evaluate(async () => {
    const skyPath = "/src/island/sky.ts", behaviorPath = "/src/island/behavior.ts";
    const { Sky } = await import(skyPath), behavior = await import(behaviorPath);
    const sky = new Sky(() => {});
    document.body.append(sky.canvas);
    const session = { id: "a", agent: "claude", project: "a", cwd: null, status: "idle", activity: null,
      step: null, steps: [], step_count: 0, subagents: 0, note: null, editor: null, species: "atratus" };
    sky.place(new Map([["claude:a", { x: 200, y: 24, scale: 1 }]]), { left: 0, top: 0, width: 720, height: 200, radius: 14 });
    Object.assign(window, { songTest: { sky, session, behavior } });
  });
  const play = (on: boolean) => page.evaluate((on) => {
    const { sky, session, behavior } = (window as any).songTest;
    behavior.setMusic(on);
    sky.update([session], true);
    return behavior.clipFor(session);
  }, on);
  const owns = () => page.evaluate(() => (window as any).songTest.sky.owns("claude:a"));
  expect(await play(true)).toBe("court");
  await page.clock.runFor(5000);
  expect(await owns()).toBe(false);
  // The song stops: the idle bird goes up again after its rest.
  expect(await play(false)).toBe("idle");
  await page.clock.runFor(5000);
  expect(await owns()).toBe(true);
  // A song starting again brings it down to court.
  await play(true);
  await page.clock.runFor(8000);
  expect(await owns()).toBe(false);
});

test("Zeca swallowing a file then shows his crop full, going down", async ({ page }) => {
  await wire(page);
  const talk = (clip: string) => page.evaluate((clip) => {
    const { scene } = (window as any).wireTest;
    scene.update([], null, { clip, agent: "claude" });
    return scene.playing("zeca");
  }, clip);
  expect(await talk("idle")).toBe("idle");
  await page.clock.runFor(5000);
  expect(await talk("swallow")).toBe("fed");
  // The chat goes back to idle when the swallow is over: the crop still plays out.
  expect(await talk("idle")).toBe("fed");
  // A message sent about the file: he thinks at once, the crop cut short.
  expect(await talk("think")).toBe("think");
});

test("in the summer molt a soaring bird misses a flight feather on each wing", async ({ page }) => {
  await page.goto("/lab/flight/");
  const parts = await page.evaluate(async () => {
    const directorPath = "/src/character/director.ts", flockPath = "/src/character/flock/index.ts";
    const { Bird, setMolt, moltSeason } = await import(directorPath);
    const { speciesSet } = await import(flockPath);
    const set = speciesSet("atratus");
    const soaring = () => {
      const bird = new Bird(set, { x: 100, wireY: 30, height: 20, skyRight: 720, skyTop: 0 });
      bird.soar({ cx: 300, cy: 80, rx: 200, ry: 40, lapMs: 9000, phase: 0 }, 0);
      const sortie = new Bird(set, { x: 100, wireY: 30, height: 20, skyRight: 720, skyTop: 0 });
      sortie.want("fly", 0);
      // Every flight frame of a web sortie (take-off, cruise, the way home), not only the soaring.
      const legs = [400, 1500, 3000, 5000, 7000].map((t) => sortie.shot(t).frame).filter((f: { layers: unknown[] }) => f.layers.length === 1);
      return [bird.shot(5000).frame, ...legs].map((f: { layers: [string][] }) => f.layers[0][0]);
    };
    setMolt(false);
    const summer = (m: number) => moltSeason(new Date(2026, m, 15));
    const off = soaring();
    setMolt(true);
    const on = soaring();
    setMolt(false);
    return { off, on, months: [0, 2, 3, 10, 11].map(summer) };
  });
  expect(parts.off.some((p: string) => p.endsWith(":molt"))).toBe(false);
  expect(parts.on.every((p: string) => p.endsWith(":molt"))).toBe(true);
  // December to March.
  expect(parts.months).toEqual([true, true, false, false, true]);
});

test("in the thermal each species beats its own wings, the wider ones more rarely", async ({ page }) => {
  await page.goto("/lab/flight/");
  const bursts = await page.evaluate(async () => {
    const directorPath = "/src/character/director.ts", flockPath = "/src/character/flock/index.ts";
    const { Bird } = await import(directorPath);
    const { speciesSet } = await import(flockPath);
    const count = (id: string) => {
      const bird = new Bird(speciesSet(id), { x: 100, wireY: 30, height: 20, skyRight: 720, skyTop: 0 });
      bird.soar({ cx: 300, cy: 80, rx: 200, ry: 40, lapMs: 9000, phase: 0 }, 0);
      // Past the climb, 40 s of circling, a frame every 20 ms; a beat more than a second after the
      // last one starts a burst (a burst's beats have short glides between them).
      let n = 0, last = -Infinity;
      for (let t = 2000; t < 42_000; t += 20) {
        if (!bird.shot(t).frame.layers[0][0].startsWith("fly_")) continue;
        if (t - last > 1000) n++;
        last = t;
      }
      return n;
    };
    return { black: count("atratus"), condor: count("vultur"), turkey: count("aura") };
  });
  // The black vulture every 3.6 s; the Andean condor about every 10 s.
  expect(bursts.black).toBeGreaterThanOrEqual(10);
  expect(bursts.black).toBeLessThanOrEqual(12);
  expect(bursts.condor).toBeGreaterThanOrEqual(3);
  expect(bursts.condor).toBeLessThanOrEqual(5);
  expect(bursts.turkey).toBeGreaterThan(0);
});

test("each species keeps its height in the sky, and the day has its parts", async ({ page }) => {
  await page.goto("/lab/flight/");
  const r = await page.evaluate(async () => {
    const skyPath = "/src/island/sky.ts", dayPath = "/src/island/daypart.ts";
    const { floorOf } = await import(skyPath), { partAt } = await import(dayPath);
    return {
      floors: ["aura", "melambrotus", "atratus", "vultur", "gyps-fulvus"].map(floorOf),
      parts: [5, 6, 8, 9, 16, 17, 18, 19, 23].map(partAt),
    };
  });
  expect(r.floors).toEqual([-1, -1, 0, 1, 1]);
  expect(r.parts).toEqual(["night", "dawn", "dawn", "day", "day", "dusk", "dusk", "night", "night"]);
});

test("only by day does the idle flock soar; at dawn it suns itself, with no mark", async ({ page }) => {
  await page.clock.install();
  await page.goto("/lab/flight/");
  await page.evaluate(async () => {
    const skyPath = "/src/island/sky.ts", dayPath = "/src/island/daypart.ts", behaviorPath = "/src/island/behavior.ts";
    const { Sky } = await import(skyPath), day = await import(dayPath), behavior = await import(behaviorPath);
    const sky = new Sky(() => {});
    document.body.append(sky.canvas);
    const session = { id: "a", agent: "claude", project: "a", cwd: null, status: "idle", activity: null,
      step: null, steps: [], step_count: 0, subagents: 0, note: null, editor: null, species: "atratus" };
    sky.place(new Map([["claude:a", { x: 200, y: 24, scale: 1 }]]), { left: 0, top: 0, width: 720, height: 200, radius: 14 });
    Object.assign(window, { dayTest: { sky, session, day, behavior } });
  });
  const at = (part: string) => page.evaluate((part) => {
    const { sky, session, day, behavior } = (window as any).dayTest;
    day.setDayPart(part);
    sky.update([session], true);
    return { clip: behavior.clipFor(session), emote: behavior.emoteFor(session) };
  }, part);
  const owns = () => page.evaluate(() => (window as any).dayTest.sky.owns("claude:a"));
  expect(await at("night")).toEqual({ clip: "idle", emote: null });
  await page.clock.runFor(5000);
  expect(await owns()).toBe(false);
  expect(await at("dawn")).toEqual({ clip: "sun", emote: null });
  await page.clock.runFor(5000);
  expect(await owns()).toBe(false);
  await at("day");
  await page.clock.runFor(5000);
  expect(await owns()).toBe(true);
  // Dusk brings it down.
  await at("dusk");
  await page.clock.runFor(8000);
  expect(await owns()).toBe(false);
});

test("at night a resting bird nods off within seconds", async ({ page }) => {
  const w = await wire(page);
  await page.evaluate(async () => {
    const dayPath = "/src/island/daypart.ts";
    (await import(dayPath)).setDayPart("night");
  });
  await w.update("idle");
  // 8 s, ±12% by the bird's key.
  await page.clock.runFor(9200);
  expect(["doze", "sleep"]).toContain(await w.playing());
});

test("a signature that happens in the air plays once a lap in the thermal, the skimmer dipping low", async ({ page }) => {
  await page.goto("/lab/flight/");
  const r = await page.evaluate(async () => {
    const directorPath = "/src/character/director.ts", flockPath = "/src/character/flock/index.ts";
    const { Bird } = await import(directorPath);
    const { speciesSet, perchedSignature } = await import(flockPath);
    // A lap of 9 s from t = 1.8 s (past the climb), the flourish at a third of it; the box is rows
    // 0 to 120, the orbit tall (ry 38), so the skim must stay inside it wherever on the lap.
    const lap = (id: string, floor: number | undefined) => {
      const set = speciesSet(id);
      const soaring = (flourish: boolean) => {
        const bird = new Bird(set, { x: 100, wireY: 30, height: 20, skyRight: 720, skyTop: 0 });
        bird.soar({ cx: 300, cy: 52, rx: 200, ry: 38, lapMs: 9000, phase: 1.2, flourish: flourish ? { at: 1 / 3, top: 9, floor } : undefined }, 0);
        return bird;
      };
      const bird = soaring(true), plain = soaring(false);
      let own = 0, lowest = -Infinity, highest = Infinity, jump = 0, last: number | null = null;
      for (let t = 1800; t < 10_800; t += 20) {
        const shot = bird.shot(t);
        // The frame is 17 rows tall, its centre 8 under its top.
        const centre = shot.y + 8;
        // Off the plain orbit: the flourish's own rises and falls.
        own = Math.max(own, Math.abs(shot.y - plain.shot(t).y));
        lowest = Math.max(lowest, centre);
        highest = Math.min(highest, centre);
        if (last !== null) jump = Math.max(jump, Math.abs(centre - last));
        last = centre;
      }
      return { perched: perchedSignature(set), own, lowest, highest, jump };
    };
    return { rueppell: lap("gyps-rueppelli", undefined), grass: lap("burrovianus", 110), flat: lap("burrovianus", undefined) };
  });
  expect(r.rueppell.perched).toBe(false);
  expect(r.rueppell.own).toBeGreaterThan(3);
  // Never above the top, never a jump: rises and falls grow and shrink with the flourish.
  expect(r.rueppell.highest).toBeGreaterThanOrEqual(9);
  expect(r.rueppell.jump).toBeLessThan(3);
  // The skim comes down to the floor and no further, from wherever on the lap it starts.
  expect(r.grass.lowest).toBeGreaterThan(r.flat.lowest + 10);
  expect(r.grass.lowest).toBeLessThanOrEqual(111);
  // A steep but continuous dive: from the top of the lap to the floor in 0.7 s, 20 ms apart.
  expect(r.grass.jump).toBeLessThan(5);
});
