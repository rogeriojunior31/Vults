import { el } from "../../dom";
import { t } from "../../i18n";
import { icon } from "../../island/icons";

/** Only presentation is kept here; folding never changes the selected item or its owner. */
export function sidePanel(panel: HTMLElement, id: string, label: string, width: number, side: "left" | "right" = "left", min = 180, max = 400): { relabel: () => void } {
  const key = `vults.panel.${id}`;
  let size = width;
  let folded = false;
  try {
    const saved = JSON.parse(localStorage.getItem(key) ?? "null");
    if (Number.isFinite(saved?.width)) size = Math.max(min, Math.min(max, saved.width));
    folded = saved?.folded === true;
  } catch { /* The current panel remains usable without saved layout. */ }
  const remember = () => { try { localStorage.setItem(key, JSON.stringify({ width: size, folded })); } catch { /* Keep the in-memory layout. */ } };
  const fold = el("button", { class: "panel-fold" });
  const handle = el("div", { class: "panel-handle" });
  handle.tabIndex = 0;
  handle.setAttribute("role", "separator");
  handle.setAttribute("aria-orientation", "vertical");
  const tools = el("div", { class: "panel-tools" }, el("strong", { text: t(label) }), fold);
  panel.classList.add("flexible", `panel-${side}`);
  panel.prepend(tools);
  panel.append(handle);
  const apply = () => {
    tools.querySelector("strong")!.textContent = t(label);
    handle.setAttribute("aria-label", t("Resize {panel}", { panel: t(label) }));
    handle.title = t("Drag to resize · double-click to reset");
    panel.style.setProperty("--panel-width", `${folded ? 48 : size}px`);
    panel.classList.toggle("panel-folded", folded);
    fold.replaceChildren(icon(folded === (side === "left") ? "sideRight" : "sideLeft", 15));
    const name = folded ? t("Show {panel}", { panel: t(label) }) : t("Fold {panel}", { panel: t(label) });
    fold.title = name;
    fold.setAttribute("aria-label", name);
    fold.setAttribute("aria-expanded", String(!folded));
    handle.setAttribute("aria-valuemin", String(min));
    handle.setAttribute("aria-valuemax", String(max));
    handle.setAttribute("aria-valuenow", String(size));
  };
  const resize = (next: number) => {
    const cap = Math.max(min, Math.min(max, (panel.parentElement?.clientWidth || max * 2) * .48));
    size = Math.max(min, Math.min(cap, Math.round(next)));
    folded = false;
    apply();
  };
  const reset = () => { resize(width); remember(); };
  fold.addEventListener("click", () => { folded = !folded; apply(); remember(); });
  handle.addEventListener("dblclick", reset);
  handle.addEventListener("keydown", e => {
    if (!["ArrowLeft", "ArrowRight", "Home", "End", "Enter"].includes(e.key)) return;
    e.preventDefault();
    if (e.key === "Enter") { fold.click(); fold.focus(); return; }
    if (e.key === "Home") resize(min);
    else if (e.key === "End") resize(max);
    else resize(size + ((e.key === "ArrowRight") === (side === "left") ? 20 : -20));
    remember();
  });
  handle.addEventListener("pointerdown", e => {
    if (e.button !== 0) return;
    e.preventDefault();
    const start = e.clientX, from = size;
    handle.setPointerCapture?.(e.pointerId);
    handle.classList.add("dragging");
    const move = (event: PointerEvent) => resize(from + (event.clientX - start) * (side === "left" ? 1 : -1));
    const end = () => {
      handle.classList.remove("dragging");
      handle.removeEventListener("pointermove", move);
      handle.removeEventListener("pointerup", end);
      handle.removeEventListener("pointercancel", end);
      handle.removeEventListener("lostpointercapture", end);
      remember();
    };
    handle.addEventListener("pointermove", move);
    handle.addEventListener("pointerup", end);
    handle.addEventListener("pointercancel", end);
    handle.addEventListener("lostpointercapture", end);
  });
  apply();
  return { relabel: apply };
}
