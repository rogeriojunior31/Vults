// A failure the user reads: what failed, in their language, then the backend's own words when the
// UI does not already know them as a sentence.
import { known, t } from "./i18n";

export function failure(sentence: string, error: unknown): string {
  const raw = (error instanceof Error ? error.message : String(error)).trim();
  if (!raw) return sentence;
  if (known(raw)) return t(raw);
  return t("{what} Details: {error}", { what: sentence, error: raw });
}
