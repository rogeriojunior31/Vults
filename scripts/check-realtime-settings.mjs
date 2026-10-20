// Exercise the production bridge without a desktop, microphone, keyring or provider.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { build, normalizePath } from "vite";

const root = normalizePath(fileURLToPath(new URL("../ui/", import.meta.url)));
const calls = [];
const saved = { on: false, model: "gpt-realtime-2.1-mini", limitMicrousd: "9223372036854775807" };
globalThis.realtimeSettingsInvoke = async (command, args) => {
  calls.push({ command, args });
  if (command === "voice_realtime_settings") return saved;
};
const bundles = await build({ configFile: false, root, logLevel: "silent", plugins: [{
  name: "realtime-settings-boundary",
  enforce: "pre",
  resolveId(id) { if (id.startsWith("@tauri-apps/api/")) return `\0${id}`; },
  load(id) {
    if (id === "\0@tauri-apps/api/core") return "export const invoke = globalThis.realtimeSettingsInvoke;";
    if (id === "\0@tauri-apps/api/event") return "export const emitTo = () => {}; export const listen = () => {};";
    if (id === "\0@tauri-apps/api/window") return "export const getCurrentWindow = () => ({});";
  },
}], build: { write: false, minify: false, lib: { entry: `${root}src/bridge.ts`, formats: ["es"] } } });
try {
  const chunk = bundles[0].output.find(part => part.type === "chunk");
  const { Bridge } = await import(`data:text/javascript;base64,${Buffer.from(chunk.code).toString("base64")}`);
  assert.equal(calls.length, 0, "Importing the bridge cannot read settings or start voice");
  assert.deepEqual(await Bridge.voiceRealtimeSettings(), saved);
  assert.deepEqual(calls.pop(), { command: "voice_realtime_settings", args: undefined });
  for (const limit of [NaN, Infinity, -1, 0.5, Number.MAX_SAFE_INTEGER + 1, "1000000", null]) {
    await assert.rejects(Bridge.setVoiceRealtime(true, limit, saved.model));
    assert.equal(calls.length, 0, "Invalid or rounded budgets cannot reach native consent");
  }
  await Bridge.setVoiceRealtime(true, 1_000_000, saved.model);
  assert.deepEqual(calls.pop(), { command: "set_voice_realtime", args: { on: true, limitMicrousd: 1_000_000, model: saved.model } });
  await Bridge.setVoiceRealtime(false, 0, saved.model);
  assert.deepEqual(calls.pop(), { command: "set_voice_realtime", args: { on: false, limitMicrousd: 0, model: saved.model } });
  assert.equal(calls.length, 0);
  console.log("check-realtime-settings: exact readback, explicit writes and safe-integer budgets without capture");
} finally {
  delete globalThis.realtimeSettingsInvoke;
}
