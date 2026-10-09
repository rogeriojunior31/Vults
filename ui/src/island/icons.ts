// Line icons for the island, drawn for this project on a 24-unit grid (stroke, round caps).

const PATHS = {
  chevron: "M9 6l6 6-6 6",
  check: "M5 12.5l4.5 4.5L19 7.5",
  openOut: "M14 5h5v5M19 5l-8 8M17 14v4a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V8a1 1 0 0 1 1-1h4",
  // A cog of six teeth around its hub (the old one, rays round a dot, read as the sun).
  gear: "M10.21 2.77L13.79 2.77L14.21 5.57L16.46 6.87L19.09 5.83L20.89 8.94L18.68 10.70L18.68 13.30L20.89 15.06L19.09 18.17L16.46 17.13L14.21 18.43L13.79 21.23L10.21 21.23L9.79 18.43L7.54 17.13L4.91 18.17L3.11 15.06L5.32 13.30L5.32 10.70L3.11 8.94L4.91 5.83L7.54 6.87L9.79 5.57ZM15.00 12a3.0 3.0 0 1 0-6.0 0a3.0 3.0 0 1 0 6.0 0z",
  sound: "M5 9.5h3l4-3.5v12l-4-3.5H5zM15.5 9a4 4 0 0 1 0 6M18 6.5a7.5 7.5 0 0 1 0 11",
  mute: "M5 9.5h3l4-3.5v12l-4-3.5H5zM16 9.5l5 5M21 9.5l-5 5",
  // A vulture perched on the wire, hunched, its bare head forward (the old two dots read as eyes).
  flock: "M3 20.5h18M6.5 20.5C4.6 16.4 5 10 10.4 7.8c1.4-.5 2.7-.4 3.6.3M19.2 10.6a2.3 2.3 0 1 1-4.6 0 2.3 2.3 0 0 1 4.6 0zM19 11.8l1.6 1.5M14.4 12.4c.8 2.8.4 5.6-1.8 8.1M9 12c-.2 3.2.6 6 2.2 8.5",
  chat: "M5 6.5h14a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1h-7l-4 3v-3H5a1 1 0 0 1-1-1v-8a1 1 0 0 1 1-1z",
  close: "M7 7l10 10M17 7L7 17",
  fold: "M6 15l6-6 6 6",
  plus: "M12 5v14M5 12h14",
  chevronDown: "M6 9l6 6 6-6",
  stop: "M7 7h10v10H7z",
  file: "M7 3h7l5 5v13H7zM14 3v5h5",
  folder: "M4 6.5a1 1 0 0 1 1-1h4.5l2 2H19a1 1 0 0 1 1 1V17a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1z",
  list: "M9 7h10M9 12h10M9 17h10M5 7h.01M5 12h.01M5 17h.01",
  pin: "M9 4h6l-1 6 3 3H7l3-3zM12 13v7",
  moon: "M19 14.5A7.5 7.5 0 0 1 9.5 5a7.5 7.5 0 1 0 9.5 9.5z",
  hide: "M4 12s3-6 8-6 8 6 8 6-3 6-8 6-8-6-8-6zM12 14.5a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5zM5 19L19 5",
  play: "M8 5.5v13l10-6.5z",
  pause: "M8.5 6v12M15.5 6v12",
  previous: "M7 6v12M18 6.5v11L10 12z",
  next: "M17 6v12M6 6.5v11L14 12z",
  note: "M9 17.5a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0zM9 17.5V5l10-2v12M19 15a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0z",
  pull: "M6 3.5a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM6 7.5v9M6 16.5a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM18 16.5a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM18 16.5V9a3 3 0 0 0-3-3h-4.5M13 3.5L10.5 6 13 8.5",
  mic: "M12 3.5a2.8 2.8 0 0 0-2.8 2.8v5.4a2.8 2.8 0 0 0 5.6 0V6.3A2.8 2.8 0 0 0 12 3.5zM6.5 11.5a5.5 5.5 0 0 0 11 0M12 17v3.5",
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
