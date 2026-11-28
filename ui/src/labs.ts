// Labs: beta features, each off until the user turns it on in Settings → Labs. The table is
// generated from `crates/core/src/labs.rs`; the app's own checks are in Rust.
import { LABS, type LabEntry } from "./labs.gen";

export { LABS, type LabEntry };

/** Whether `id` is on in `chosen`, the settings' Labs: its own key and the one it requires. */
export function labOn(chosen: Record<string, boolean>, id: string): boolean {
  const entry = LABS.find(l => l.id === id);
  if (!entry || !chosen[id]) return false;
  return entry.requires === null || !!chosen[entry.requires];
}
