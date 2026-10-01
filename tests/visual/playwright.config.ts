// Visual tests: the lab's island states and clips, with time frozen, against stored screenshots.
// npm run test:visual            compare
// npm run test:visual -- -u      accept the new look
import { defineConfig } from "@playwright/test";

const PORT = 1430;

export default defineConfig({
  testDir: ".",
  snapshotPathTemplate: "{testDir}/__screenshots__/{arg}{ext}",
  fullyParallel: true,
  reporter: [["list"]],
  use: { baseURL: `http://127.0.0.1:${PORT}`, viewport: { width: 1300, height: 1100 } },
  // Strict on purpose: the screenshots are stable, a corner radius changes only a few hundred
  // pixels, and on a dark UI most changes are subtle shades (a #141518 card on a #000 island is far
  // below the default color threshold of 0.2).
  expect: { toHaveScreenshot: { maxDiffPixels: 20, threshold: 0.02, animations: "disabled" } },
  webServer: {
    command: `npx vite ui --port ${PORT} --strictPort`,
    cwd: "../..",
    url: `http://127.0.0.1:${PORT}/lab/`,
    reuseExistingServer: true,
  },
  projects: [{ name: "chromium", use: { browserName: "chromium" } }],
});
