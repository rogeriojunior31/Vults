// Inspect the packaged payload, then probe its backend without starting the application UI.
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { access, mkdtemp, readFile, readdir } from 'node:fs/promises';
import { basename, dirname, join } from 'node:path';

const digest = bytes => createHash('sha256').update(bytes).digest('hex');
export function nsisBinary(bytes) {
  // Tauri restores its source binary after replacing this bundle marker in the NSIS payload.
  const copy = Buffer.from(bytes);
  const marker = Buffer.from('__TAURI_BUNDLE_TYPE_VAR_UNK');
  const bundled = Buffer.from('__TAURI_BUNDLE_TYPE_VAR_NSS');
  for (let offset = copy.indexOf(marker); offset !== -1; offset = copy.indexOf(marker, offset + marker.length)) bundled.copy(copy, offset);
  return copy;
}
export async function verifyInstaller(installer, expected, base) {
  let zip;
  try { zip = execFileSync('where.exe', ['7z.exe'], { encoding: 'utf8', windowsHide: true }).trim().split(/\r?\n/)[0]; }
  catch { zip = join(process.env.ProgramFiles ?? 'C:/Program Files', '7-Zip/7z.exe'); }
  const extracted = await mkdtemp(join(base, 'verify-'));
  execFileSync(zip, ['x', installer, `-o${extracted}`, '-y'], { windowsHide: true, stdio: 'pipe' });
  const files = [];
  async function walk(path, depth = 0) {
    if (depth > 20 || files.length > 10_000) throw new Error('Unexpected Windows installer payload.');
    for (const entry of await readdir(path, { withFileTypes: true })) {
      if (entry.isSymbolicLink()) throw new Error('Unexpected link in Windows installer payload.');
      const file = join(path, entry.name);
      if (entry.isDirectory()) await walk(file, depth + 1);
      else files.push(file);
    }
  }
  await walk(extracted);
  const matched = {};
  for (const source of expected) {
    const name = basename(source).toLowerCase();
    const candidates = files.filter(file => basename(file).toLowerCase() === name);
    const sourceBytes = await readFile(source);
    const expectedBytes = name === 'vults.exe' ? nsisBinary(sourceBytes) : sourceBytes;
    if (candidates.length !== 1 || digest(await readFile(candidates[0])) !== digest(expectedBytes)) {
      throw new Error(`Windows installer payload mismatch: ${name}`);
    }
    matched[name] = candidates[0];
  }
  const binary = matched['vults.exe'], dll = matched['vulkan-1.dll'];
  if (!binary || !dll || dirname(binary).toLowerCase() !== dirname(dll).toLowerCase()) {
    throw new Error('Vulkan loader must be installed beside vults.exe.');
  }
  const probe = env => {
    const stdout = execFileSync(binary, ['--voice-backend-check'], { env, encoding: 'utf8', windowsHide: true, timeout: 30_000, stdio: ['ignore', 'pipe', 'pipe'] });
    const report = JSON.parse(stdout);
    if (report.vulkan_compiled !== true || typeof report.gpu !== 'boolean') throw new Error('Packaged app did not confirm Vulkan support.');
    return report;
  };
  const backend = probe(process.env);
  const missingDriver = join(extracted, 'no-vulkan-driver.json');
  await access(dll);
  const cpuFallback = probe({ ...process.env, VK_DRIVER_FILES: missingDriver, VK_ICD_FILENAMES: missingDriver });
  if (cpuFallback.gpu !== false) throw new Error('Packaged app did not fall back when no Vulkan driver was available.');
  return { extracted, payloadVerified: true, backend, cpuFallback };
}
