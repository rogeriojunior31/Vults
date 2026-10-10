// Fails when a sentence the UI translates (`t("…")` in ui/src) is missing from a catalog in
// ui/src/i18n/, or a catalog keeps a sentence no code asks for any more. Run by check-i18n.sh.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const root = new URL("..", import.meta.url).pathname;
const src = join(root, "ui/src");
const catalogs = ["pt-BR", "es", "zh"];

const unquote = (s) => JSON.parse(`"${s}"`);
function files(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === "i18n" ? [] : files(path);
    return path.endsWith(".ts") ? [path] : [];
  });
}

const used = new Map();
for (const file of [...files(src), ...files(join(root, "ui/lab"))]) {
  const text = readFileSync(file, "utf8");
  // t("…") translates now; tk("…") marks a sentence kept in a table and translated later.
  for (const m of text.matchAll(/\btk?\(\s*"((?:[^"\\]|\\.)*)"/g)) used.set(unquote(m[1]), file);
  for (const m of text.matchAll(/\btk?\(\s*`([^`$]*)`/g)) used.set(m[1], file);
}

let failed = false;
for (const name of catalogs) {
  const text = readFileSync(join(src, "i18n", `${name}.ts`), "utf8");
  const keys = new Set([...text.matchAll(/^\s*"((?:[^"\\]|\\.)*)":/gm)].map((m) => unquote(m[1])));
  for (const [sentence, file] of used) {
    if (!keys.has(sentence)) {
      console.log(`${name}: missing ${JSON.stringify(sentence)} (${file.slice(root.length)})`);
      failed = true;
    }
  }
  for (const key of keys) {
    if (!used.has(key)) {
      console.log(`${name}: no code asks for ${JSON.stringify(key)}`);
      failed = true;
    }
  }
}
if (failed) {
  console.log("\ncheck-i18n: every t() sentence needs its translation in each catalog (ui/src/i18n/)");
  process.exit(1);
}
console.log(`check-i18n: ok (${used.size} sentences, ${catalogs.length} languages)`);
