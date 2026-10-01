// Line icons for the island, drawn for this project on a 24-unit grid (stroke, round caps).

const PATHS = {
  chevron: "M9 6l6 6-6 6",
  check: "M5 12.5l4.5 4.5L19 7.5",
  openOut: "M14 5h5v5M19 5l-8 8M17 14v4a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V8a1 1 0 0 1 1-1h4",
  gear: "M12 9.2a2.8 2.8 0 1 0 0 5.6 2.8 2.8 0 0 0 0-5.6zM12 3v2.2M12 18.8V21M4.2 7.5l1.9 1.1M17.9 15.4l1.9 1.1M4.2 16.5l1.9-1.1M17.9 8.6l1.9-1.1",
  sound: "M5 9.5h3l4-3.5v12l-4-3.5H5zM15.5 9a4 4 0 0 1 0 6M18 6.5a7.5 7.5 0 0 1 0 11",
  mute: "M5 9.5h3l4-3.5v12l-4-3.5H5zM16 9.5l5 5M21 9.5l-5 5",
  flock: "M4 18h16M7 14.5a2.2 2.2 0 1 0 0-4.4 2.2 2.2 0 0 0 0 4.4zM16.5 14.5a2.2 2.2 0 1 0 0-4.4 2.2 2.2 0 0 0 0 4.4z",
  chat: "M5 6.5h14a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1h-7l-4 3v-3H5a1 1 0 0 1-1-1v-8a1 1 0 0 1 1-1z",
  close: "M7 7l10 10M17 7L7 17",
} as const;

export type IconName = keyof typeof PATHS;

export function icon(name: IconName, size = 14, stroke = 1.9): SVGSVGElement {
  const ns = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(ns, "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("width", String(size));
  svg.setAttribute("height", String(size));
  svg.setAttribute("fill", "none");
  svg.setAttribute("stroke", "currentColor");
  svg.setAttribute("stroke-width", String(stroke));
  svg.setAttribute("stroke-linecap", "round");
  svg.setAttribute("stroke-linejoin", "round");
  svg.setAttribute("aria-hidden", "true");
  const path = document.createElementNS(ns, "path");
  path.setAttribute("d", PATHS[name]);
  svg.append(path);
  return svg;
}
