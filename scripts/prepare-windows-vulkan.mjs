// Build-time tools only. Extract signed upstream archives without running an installer.
import { createHash } from 'node:crypto';
import { createReadStream, createWriteStream } from 'node:fs';
import { access, copyFile, mkdir, readFile, rename, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { Readable } from 'node:stream';
import { pipeline } from 'node:stream/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const base = join(root, 'target/windows-vulkan');
// Digests published at https://vulkan.lunarg.com/sdk/files.json; SDK and runtime release independently.
export const SDK = { version: '1.4.363.1', name: 'vulkansdk-windows-X64-1.4.363.1.exe', sha: 'c02932bd9a968b22d404536108adaacac908c081cd6b0113f5bfefc523dd0a6a' };
export const RUNTIME = { version: '1.4.363.0', name: 'VulkanRT-X64-1.4.363.0-Components.zip', sha: 'a25a927aa8b9f0371048f1861cf88ac3b9bc9b1fb332c42d897c8ab32695769a' };
async function exists(path) { try { await access(path); return true; } catch { return false; } }
async function hash(path) {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  return hash.digest('hex');
}
async function archive(spec, previous) {
  const path = join(base, 'downloads', spec.name);
  await mkdir(dirname(path), { recursive: true });
  if (!await exists(path)) {
    if (await exists(previous) && await hash(previous) === spec.sha) await copyFile(previous, path);
    else {
      const response = await fetch(`https://sdk.lunarg.com/sdk/download/${spec.version}/windows/${spec.name}`, { signal: AbortSignal.timeout(300_000) });
      if (!response.ok || !response.body) throw new Error(`Vulkan download failed: HTTP ${response.status}`);
      await pipeline(Readable.fromWeb(response.body), createWriteStream(`${path}.partial`));
      if (await hash(`${path}.partial`) !== spec.sha) throw new Error(`Vulkan checksum mismatch: ${spec.name}`);
      await rename(`${path}.partial`, path);
    }
  }
  if (await hash(path) !== spec.sha) throw new Error(`Vulkan checksum mismatch: ${spec.name}`);
  return path;
}
function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit++) crc = (crc >>> 1) ^ ((crc & 1) ? 0xedb88320 : 0);
  }
  return (crc ^ 0xffffffff) >>> 0;
}
// Qt's SDK contains several complete 7z archives; validate their headers and bounds before slicing.
export function embeddedArchives(bytes) {
  const signature = Buffer.from([0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c]);
  const archives = [];
  for (let offset = bytes.indexOf(signature); offset !== -1; offset = bytes.indexOf(signature, offset + 1)) {
    if (offset + 32 > bytes.length || crc32(bytes.subarray(offset + 12, offset + 32)) !== bytes.readUInt32LE(offset + 8)) continue;
    const size = 32n + bytes.readBigUInt64LE(offset + 12) + bytes.readBigUInt64LE(offset + 20);
    if (size > BigInt(bytes.length - offset)) continue;
    archives.push(bytes.subarray(offset, offset + Number(size)));
  }
  return archives;
}
function sevenZip() {
  try { return execFileSync('where.exe', ['7z.exe'], { encoding: 'utf8', windowsHide: true }).trim().split(/\r?\n/)[0]; }
  catch { return join(process.env.ProgramFiles ?? 'C:/Program Files', '7-Zip/7z.exe'); }
}
export async function prepare() {
  if (process.platform !== 'win32' || process.arch !== 'x64') throw new Error('Windows Vulkan packaging requires an x64 Windows host.');
  const zip = sevenZip();
  const run = args => execFileSync(zip, args, { encoding: 'utf8', windowsHide: true, maxBuffer: 16 * 1024 * 1024 });
  const sdk = join(base, 'sdk'), runtime = join(base, 'runtime');
  await mkdir(sdk, { recursive: true });
  await mkdir(runtime, { recursive: true });
  const sdkArchive = await archive(SDK, join(root, 'target/local-tools/vulkan-sdk-installer.exe'));
  const parts = embeddedArchives(await readFile(sdkArchive));
  let found = 0;
  for (const [index, part] of parts.entries()) {
    const path = join(base, 'downloads', `sdk-part-${index}.7z`);
    await writeFile(path, part);
    const files = run(['l', '-ba', path]);
    const patterns = [];
    if (files.includes('Include\\vulkan\\vulkan.h')) patterns.push('Include/*');
    if (files.includes('Bin\\glslc.exe')) patterns.push('Bin/glslc.exe');
    if (files.includes('Lib\\vulkan-1.lib')) patterns.push('Lib/vulkan-1.lib');
    if (patterns.length) { run(['x', path, `-o${sdk}`, '-y', ...patterns]); found += patterns.length; }
  }
  if (found !== 3) throw new Error('The pinned Vulkan SDK is missing required build components.');
  const runtimeArchive = await archive(RUNTIME, join(root, 'target/local-tools/vulkan-runtime.zip'));
  const prefix = `VulkanRT-X64-${RUNTIME.version}-Components`;
  run(['e', runtimeArchive, `-o${runtime}`, '-y', `${prefix}/x64/vulkan-1.dll`, `${prefix}/VulkanRT-License.txt`]);
  for (const path of ['Include/vulkan/vulkan.h', 'Bin/glslc.exe', 'Lib/vulkan-1.lib']) await access(join(sdk, path));
  const dll = await readFile(join(runtime, 'vulkan-1.dll'));
  if (dll.readUInt16LE(dll.readUInt32LE(0x3c) + 4) !== 0x8664) throw new Error('Vulkan runtime is not x64.');
  await access(join(runtime, 'VulkanRT-License.txt'));
  const report = { sdk, runtime, sdkVersion: SDK.version, sdkSha256: SDK.sha, runtimeVersion: RUNTIME.version, runtimeSha256: RUNTIME.sha };
  await writeFile(join(base, 'prepared.json'), JSON.stringify(report, null, 2));
  return report;
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  prepare().then(report => console.log(JSON.stringify(report, null, 2))).catch(error => { console.error(error.message); process.exitCode = 1; });
}
