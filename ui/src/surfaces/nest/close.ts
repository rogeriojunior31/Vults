/** A system close must wait for navigation and drafts, including repeated close requests. */
export function savedClose(
  flush: () => Promise<void>,
  close: () => Promise<void>,
  failed: () => void,
) {
  let pending = false;
  return async (event: { preventDefault(): void }) => {
    event.preventDefault();
    if (pending) return;
    pending = true;
    try {
      await flush();
      await close();
    } catch {
      failed();
    } finally {
      pending = false;
    }
  };
}
