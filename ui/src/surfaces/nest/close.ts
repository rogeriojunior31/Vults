/** Serialize a surface change after saving; a failed attempt leaves its state available to retry. */
export function savedTransition(
  flush: () => Promise<void>,
  failed: () => void,
) {
  let pending = false;
  return async (change: () => Promise<void>) => {
    if (pending) return;
    pending = true;
    try {
      await flush();
      await change();
    } catch {
      failed();
    } finally {
      pending = false;
    }
  };
}

/** System close has its own guard: a size change may request this close while still pending. */
export function savedClose(flush: () => Promise<void>, close: () => Promise<void>, failed: () => void) {
  const transition = savedTransition(flush, failed);
  return async (event: { preventDefault(): void }) => {
    event.preventDefault();
    await transition(close);
  };
}
