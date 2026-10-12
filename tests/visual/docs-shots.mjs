// The images in README.md and docs/, taken from the lab with time frozen. Run before each release:
//   node tests/visual/docs-shots.mjs [outdir]      (default docs/assets)
// It starts its own Vite server on a free port, so other dev servers can keep running, and shrinks
// the PNGs to 256 colors when ImageMagick is installed.
// Run it on a desktop, not in the visual tests' container: the pictures show the app as users see
// it, and the container's system-ui font has no bold.
import { execFileSync } from "node:child_process";
import { mkdirSync } from "node:fs";
import { chromium } from "@playwright/test";
import { createServer } from "vite";

const out = process.argv[2] ?? "docs/assets";
mkdirSync(out, { recursive: true });

/** States the island shows only once they hold (render.ts SETTLE_MS). */
const SETTLE = 1700;

const SHOTS = [
  // The README's hero: vults of the world's species, one in front with its session.
  { file: "island-flock.png", query: "state=busy-flock&open=1&flock=world" },
  { file: "island-compact-flock.png", query: "state=busy-flock" },
  { file: "island-busy-flock.png", query: "state=busy-flock&open=1" },
  // A vult opens its session: the view the prototype draws, back to the flock with ‹ Flock.
  {
    file: "island-session.png",
    query: "state=editing&open=1",
    act: async (page) => {
      await page.locator("#island .habitat-bird.chosen").click();
      await page.locator("#island .card.focus").waitFor();
    },
  },
  { file: "island-approval.png", query: "state=approval&open=1", wait: SETTLE },
  { file: "island-question.png", query: "state=question-card&open=1", wait: SETTLE },
  { file: "island-chat-permission.png", query: "state=chat-permission&open=1" },
  { file: "island-usage-music.png", query: "state=idle-flock&open=1&usage=1&music=1" },
  {
    file: "island-live-diff.png",
    query: "state=live-diff&open=1",
    act: async (page) => {
      await page.locator("#island .habitat-bird.chosen").click();
      await page.locator(".tick-diff").click();
      await page.locator(".diff-view .diff-line").first().waitFor();
    },
  },
  {
    file: "island-github-card.png",
    query: "state=github-card&open=1",
    act: async (page) => {
      await page.getByRole("button", { name: /^Connections/ }).click();
      await page.locator(".qconn-row").first().waitFor();
    },
  },
];
const LOOKS = [
  "witch-hat", "santa-hat", "party-hat", "bunny-ears", "sunglasses",
  "west-coast", "fitted-cap", "mountain-hat", "headband", "dreads",
  "front-knot", "durag", "crown", "bucket-hat", "clock-chain", "headphones", "shutter-shades", "chrome-chain", "eye-patch",
];
/** Looks per row of the picture: the seasonal ones and the sunglasses fill the first. */
const PER_ROW = 5;

const server = await createServer({ root: "ui", configFile: "ui/vite.config.ts", server: { port: 0, strictPort: false }, logLevel: "error" });
await server.listen();
const base = server.resolvedUrls.local[0].replace(/\/$/, "");
const browser = await chromium.launch();
try {
  const context = await browser.newContext({ viewport: { width: 1300, height: 1100 }, deviceScaleFactor: 2 });
  const page = await context.newPage();
  page.on("pageerror", (e) => console.error(e.message));
  // On black, as on a dark desktop: the lab's grey would show in the island's rounded corners.
  const open = async (query) => {
    await page.goto(`${base}/lab/?still=1&t=1500&${query}`);
    await page.addStyleTag({ content: "html, body, .island-frame { background: #000 !important; }" });
  };

  for (const shot of SHOTS) {
    await open(shot.query);
    if (shot.wait) await page.waitForTimeout(shot.wait);
    if (shot.act) await shot.act(page);
    await page.locator("#island").screenshot({ path: `${out}/${shot.file}`, animations: "disabled" });
    console.log(shot.file);
  }

  // Pictures made of several island shots, side by side: Quick at rest and in a session, and the
  // other four tabs. Each part is taken like the shots above, then laid out on a black page.
  const shoot = async (query, act) => {
    await open(query);
    if (act) await act(page);
    return (await page.locator("#island").screenshot({ animations: "disabled" })).toString("base64");
  };
  const tab = (name) => async (p) => {
    await p.getByRole("button", { name }).first().click();
    await p.waitForTimeout(300);
  };
  const COMPOSED = [
    {
      file: "island-quick.png",
      columns: 2,
      parts: [
        ["state=busy-flock&open=1"],
        ["state=busy-flock&open=1", async (p) => {
          await p.locator("#island .habitat-bird", { hasText: "site" }).click();
          await p.locator("#island .card.focus").waitFor();
        }],
      ],
    },
    {
      file: "island-tabs.png",
      columns: 2,
      parts: [
        ["state=busy-flock&open=1", tab("Chat")],
        ["state=busy-flock&open=1", tab("Activity")],
        ["state=github-card&open=1", tab(/^Connections/)],
        ["state=busy-flock&open=1", tab("Settings")],
      ],
    },
  ];
  for (const c of COMPOSED) {
    const parts = [];
    for (const [query, act] of c.parts) parts.push(await shoot(query, act));
    await page.setContent(
      `<body style="margin:0;background:#000"><div id="row" style="display:inline-grid;grid-template-columns:repeat(${c.columns},auto);gap:24px;align-items:start;padding:12px">${parts
        .map((t) => `<img src="data:image/png;base64,${t}" style="zoom:0.5">`)
        .join("")}</div></body>`,
    );
    await page.locator("#row img").last().evaluate((img) => img.decode());
    await page.locator("#row").screenshot({ path: `${out}/${c.file}` });
    console.log(c.file);
  }

  // Zeca's looks side by side, five to a row: his perch on the empty wire's card, once per look.
  const tiles = [];
  for (const look of LOOKS) {
    await open(`empty=1&open=1&look=${look}`);
    tiles.push((await page.locator("#island .perch").first().screenshot({ animations: "disabled" })).toString("base64"));
  }
  await page.setContent(
    `<body style="margin:0;background:#000"><div id="row" style="display:inline-grid;grid-template-columns:repeat(${PER_ROW},auto);gap:8px">${tiles
      .map((t) => `<img src="data:image/png;base64,${t}" style="zoom:0.5">`)
      .join("")}</div></body>`,
  );
  await page.locator("#row img").last().evaluate((img) => img.decode());
  await page.locator("#row").screenshot({ path: `${out}/zeca-looks.png` });
  console.log("zeca-looks.png");
} finally {
  await browser.close();
  await server.close();
}
const files = [...SHOTS.map((s) => s.file), "island-quick.png", "island-tabs.png", "zeca-looks.png"].map((f) => `${out}/${f}`);
try {
  // One file at a time with the PNG8: prefix: recent ImageMagick 7 builds refuse mogrify's
  // png:format define ("no encode delegate").
  for (const f of files) execFileSync("magick", [f, "-dither", "None", "-colors", "256", "-define", "png:compression-level=9", `PNG8:${f}`]);
} catch {
  console.warn("ImageMagick not found: the PNGs are left as taken");
}
