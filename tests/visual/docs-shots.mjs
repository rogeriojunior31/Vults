// The images in README.md and docs/, taken from the lab with time frozen. Run before each release:
//   node tests/visual/docs-shots.mjs [outdir]      (default docs/assets)
// It starts its own Vite server on a free port, so other dev servers can keep running, and shrinks
// the PNGs to 256 colors when ImageMagick is installed.
import { execFileSync } from "node:child_process";
import { mkdirSync } from "node:fs";
import { chromium } from "@playwright/test";
import { createServer } from "vite";

const out = process.argv[2] ?? "docs/assets";
mkdirSync(out, { recursive: true });

/** States the island shows only once they hold (render.ts SETTLE_MS). */
const SETTLE = 1700;

const SHOTS = [
  // The README's hero: vults of the world's species, and Zeca dressed for the season.
  { file: "island-flock.png", query: "state=busy-flock&open=1&flock=world&look=witch-hat" },
  { file: "island-compact-flock.png", query: "state=busy-flock" },
  { file: "island-busy-flock.png", query: "state=busy-flock&open=1" },
  { file: "island-approval.png", query: "state=approval&open=1", wait: SETTLE },
  { file: "island-question.png", query: "state=question-card&open=1", wait: SETTLE },
  { file: "island-chat-permission.png", query: "state=chat-permission&open=1" },
  { file: "island-usage-music.png", query: "state=idle-flock&open=1&usage=1&music=1" },
  {
    file: "island-live-diff.png",
    query: "state=live-diff&open=1",
    act: async (page) => {
      await page.locator(".tick-diff").click();
      await page.locator(".diff-view .diff-line").first().waitFor();
    },
  },
  {
    file: "island-github-card.png",
    query: "state=github-card&open=1",
    act: async (page) => {
      await page.getByRole("button", { name: "GitHub" }).click();
      await page.locator(".board-row").first().waitFor();
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
  const open = (query) => page.goto(`${base}/lab/?still=1&t=1500&${query}`);

  for (const shot of SHOTS) {
    await open(shot.query);
    if (shot.wait) await page.waitForTimeout(shot.wait);
    if (shot.act) await shot.act(page);
    await page.locator("#island").screenshot({ path: `${out}/${shot.file}`, animations: "disabled" });
    console.log(shot.file);
  }

  // Zeca's looks side by side, five to a row: his perch on the focus card, once per look.
  const tiles = [];
  for (const look of LOOKS) {
    await open(`state=editing&open=1&look=${look}`);
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
const files = [...SHOTS.map((s) => s.file), "zeca-looks.png"].map((f) => `${out}/${f}`);
try {
  execFileSync("magick", ["mogrify", "-dither", "None", "-colors", "256", "-define", "png:compression-level=9", "-define", "png:format=png8", ...files]);
} catch {
  console.warn("ImageMagick not found: the PNGs are left as taken");
}
