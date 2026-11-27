// Keep Tauri resources and published artifacts correct even with a short CARGO_TARGET_DIR.
import { execFileSync, spawnSync } from 'node:child_process';
import { copyFile, mkdir, readdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyInstaller } from './verify-windows-installer.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
if (process.platform !== 'win32' || process.arch !== 'x64' || process.env.CARGO_BUILD_TARGET || args.some(arg => arg === '--target' || arg.startsWith('--target='))) {
  throw new Error('This Windows installer packages the native x64 MSVC target only.');
}
const release = join(resolve(process.env.CARGO_TARGET_DIR ?? join(root, 'target')), 'release');
const staged = join(root, 'target/release');
await mkdir(staged, { recursive: true });
const hook = join(release, 'vults-hook.exe'), stagedHook = join(staged, 'vults-hook.exe');
if (hook.toLowerCase() !== stagedHook.toLowerCase()) await copyFile(hook, stagedHook);
// ggml's shader generator runs many glslc at once and says nothing when one writes an empty .spv;
// the link then fails on a missing symbol. Such a build of whisper-rs-sys is thrown away, so it is
// generated again rather than failing every later build.
const whisperBuilds = join(release, 'build/whisper-rs-sys');
for (const hash of await readdir(whisperBuilds).catch(() => [])) {
  const spv = join(whisperBuilds, hash, 'out/build/ggml/src/ggml-vulkan/vulkan-shaders.spv');
  const empty = [];
  for (const name of await readdir(spv).catch(() => [])) {
    if (name.endsWith('.spv') && (await stat(join(spv, name))).size === 0) empty.push(name);
  }
  if (empty.length) {
    console.warn(`Rebuilding whisper's Vulkan shaders: ${empty.join(', ')} came out empty.`);
    await rm(join(whisperBuilds, hash), { recursive: true, force: true });
  }
}
const result = spawnSync(process.execPath, [join(root, 'node_modules/@tauri-apps/cli/tauri.js'), 'build', '--config', 'app/tauri.bundle.windows.json', ...args], { cwd: root, stdio: 'inherit', windowsHide: true });
if (result.error) throw result.error;
if (result.status !== 0) process.exit(result.status ?? 1);
const binary = join(release, 'vults.exe');
const dependencies = execFileSync('dumpbin.exe', ['/DEPENDENTS', binary], { encoding: 'utf8', windowsHide: true });
if (!/^\s+vulkan-1\.dll\s*$/mi.test(dependencies)) throw new Error('Refusing a Windows installer whose app lacks Vulkan support.');
const source = join(release, 'bundle/nsis'), destination = join(staged, 'bundle/nsis');
await mkdir(destination, { recursive: true });
const { version } = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'));
const installers = (await readdir(source)).filter(name => name.endsWith('.exe') && name.includes(`_${version}_`));
if (!installers.length) throw new Error('The Windows build did not produce an NSIS installer.');
const artifacts = [];
for (const name of installers) {
  const from = join(source, name), to = join(destination, name);
  const verified = await verifyInstaller(from, [binary, stagedHook, join(root, 'target/windows-vulkan/runtime/vulkan-1.dll'), join(root, 'target/windows-vulkan/runtime/VulkanRT-License.txt')], join(root, 'target/windows-vulkan'));
  if (from.toLowerCase() !== to.toLowerCase()) await copyFile(from, to);
  const bytes = await readFile(to);
  artifacts.push({ path: to, sha256: createHash('sha256').update(bytes).digest('hex'), ...verified });
}
const report = { voiceFeature: 'voice-vulkan', vulkanImport: true, binary, hook, artifacts };
await writeFile(join(root, 'target/windows-vulkan/bundle-report.json'), JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
