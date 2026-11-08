// Native File objects preserve Explorer paths without trusting renderer path strings.
export interface FileDropTransport {
  postMessageWithAdditionalObjects(message: string, objects: File[]): void;
}

export function installWindowsDrop(
  target: Window,
  transport: FileDropTransport,
  ready: () => boolean,
  hover: (over: boolean) => void,
  failed: (files: File[]) => void,
): void {
  let depth = 0;
  const reset = () => {
    if (depth > 0) hover(false);
    depth = 0;
  };
  const files = (event: DragEvent) => Array.from(event.dataTransfer?.types ?? []).includes("Files");
  target.addEventListener("dragenter", event => {
    if (!files(event)) return;
    event.preventDefault();
    if (depth++ === 0 && ready()) hover(true);
  });
  target.addEventListener("dragover", event => {
    if (!files(event)) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = ready() ? "copy" : "none";
  });
  target.addEventListener("dragleave", () => {
    if (depth > 0 && --depth === 0) hover(false);
  });
  target.addEventListener("drop", event => {
    if (!files(event)) return;
    event.preventDefault();
    reset();
    const dropped = Array.from(event.dataTransfer?.files ?? []);
    if (!ready() || dropped.length === 0) return;
    if (dropped.length > 256) { failed(dropped); return; }
    try {
      transport.postMessageWithAdditionalObjects("vults-file-drop-v1", dropped);
    } catch {
      failed(dropped);
    }
  });
  target.addEventListener("blur", reset);
  target.addEventListener("dragend", reset);
}
