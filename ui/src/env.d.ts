// Build-time choices Vite passes to the renderer (only VITE_* variables reach it).
interface ImportMetaEnv {
  /** Zeca's species, by id (ui/src/character/flock/species.ts); the black vulture by default. */
  readonly VITE_ZECA_SPECIES?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
