/** Subscribe before reading the native request; a newer event or user navigation always wins. */
export async function restoreNavigation(
  subscribe: (open: (tab: string) => void) => Promise<unknown>,
  pending: () => Promise<string | null>,
  open: (tab: string) => void,
  revision: () => number,
  initial: string,
): Promise<void> {
  const started = revision();
  try {
    await subscribe(open);
    const tab = await pending();
    if (started === revision()) open(tab ?? initial);
  } catch {
    if (started === revision()) open(initial);
  }
}
