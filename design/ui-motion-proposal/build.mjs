import { readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { build } from 'vite';
import { Script } from 'node:vm';

const here = new URL('./', import.meta.url);
const root = fileURLToPath(new URL('../../ui/', here));
const result = await build({
  configFile: false, root, logLevel: 'silent',
  plugins: [{
    name: 'proposal-assets',
    resolveId: id => id.endsWith('proposal-assets') ? '\0proposal-assets' : null,
    load: id => id === '\0proposal-assets' ? `
      export { Scene, FOCUS_SCENE, COMPACT_SCENE, HABITAT_SCENE, NEST_HABITAT_SCENE } from ${JSON.stringify(root + 'src/island/scene.ts')};
      export { assignSpecies, setZeca, setZecaLook, setZecaSpecies } from ${JSON.stringify(root + 'src/island/flock.ts')};
      export { SPECIES } from ${JSON.stringify(root + 'src/character/flock/index.ts')};
      export { Sky } from ${JSON.stringify(root + 'src/island/sky.ts')};
      export { setDayPart } from ${JSON.stringify(root + 'src/island/daypart.ts')};
      export { LOOK_GROUPS } from ${JSON.stringify(root + 'src/character/looks.ts')};
      export { AGENTS } from ${JSON.stringify(root + 'src/agents.ts')};
      export { icon } from ${JSON.stringify(root + 'src/island/icons.ts')};
      export { t, setLang } from ${JSON.stringify(root + 'src/i18n/index.ts')};
      export { SURFACE_SIZES, fitSurface, placePopover, surfaceFrames } from ${JSON.stringify(fileURLToPath(new URL('layout.mjs',here)))};
    ` : null,
  }],
  build: { write: false, minify: true, lib: { entry: 'proposal-assets', formats: ['iife'], name: 'VultsProposalAssets' } },
});
const assets = result[0].output.find(part => part.type === 'chunk').code
  .replace(/[^\x00-\x7f]/g, char => '\\u' + char.charCodeAt(0).toString(16).padStart(4, '0'));
const theme = (await readFile(new URL('../../ui/src/themes.css', here), 'utf8'))
  .replaceAll(':root', '#vults-ui-proposal').replaceAll('[data-theme="black"]', '#vults-ui-proposal[data-theme="black"]')
  .replaceAll('[data-theme="light"]', '#vults-ui-proposal[data-theme="light"]');
const template = await readFile(new URL('template.html', here), 'utf8');
const logic = await readFile(new URL('preview.js', here), 'utf8');
// Modules share the workspace's scope; their order is the order they are read in.
const MODULES = ['cards.js','island.js','compact.js','lab.js','connections.js','records.js','settings.js','memory.js','dictation.js'];
const modules = (await Promise.all(MODULES.map(name => readFile(new URL(name, here), 'utf8')))).join('\n');
const workspace = (await readFile(new URL('workspace.js', here), 'utf8')).replace('/* MODULES */', () => modules);
const extraCopy = await readFile(new URL('copy.json', here), 'utf8');
const spNight = await readFile(new URL('sp-night.json', here), 'utf8');
const fragment = template.replace('/* PRODUCTION_THEME */', () => theme)
  .replace('/* PRODUCTION_ASSETS */', () => assets.replaceAll('</script', '<\\/script'))
  .replace('/* PROPOSAL_LOGIC */', () => logic.replace('/* WORKSPACE_LOGIC */', () => workspace)
    .replace('/* SP_NIGHT */', () => spNight)
    // The page declares UTF-8 (below), so the translations go in unescaped and without the file's indentation.
    .replace('/* EXTRA_COPY */', () => JSON.stringify(JSON.parse(extraCopy)).replaceAll('</', '<\\/')));
if (Buffer.byteLength(fragment) > 1_000_000) throw new Error('Prototype exceeds inline size limit.');
for (const match of fragment.matchAll(/<script>([\s\S]*?)<\/script>/g)) new Script(match[1]);
// Opened straight from disk the page has no head: declare the encoding, and a doctype so tables inherit their text color.
await writeFile(new URL('vults-ui-proposal.html', here), '<!doctype html>\n<meta charset="utf-8">\n' + fragment);
console.log(`Prototype built: ${Buffer.byteLength(fragment)} bytes; production sprites, icons and theme embedded.`);
