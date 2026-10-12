// The prototype's line icons (design exploration 08), drawn at 24 x 24 with a 1.6 stroke in the
// text's color. One table, so every surface draws the same mark for the same thing.
const ICONS = {
  "expand": "<path d='M14 4h6v6M20 4l-7 7M10 20H4v-6M4 20l7-7'/>",
  "collapse": "<path d='m6 15 6-6 6 6'/>",
  "arrow": "<path d='M5 12h14m-6-6 6 6-6 6'/>",
  "copy": "<rect x='8' y='8' width='12' height='12' rx='2'/><path d='M14 8V4H4v10h4'/>",
  "terminal": "<path d='m4 6 5 6-5 6m8 0h8'/>",
  "fullscreen": "<path d='M8 3H3v5m13-5h5v5M3 16v5h5m13-5v5h-5'/>",
  "flock": "<path d='M12 9.2c-1 0-1.2.7-1.4 1.4L4 8.6 2 8M4 8.6l-1.6 1.8M4.4 10.4l-1.4 2c2.4 1 5 1.8 7.5 2.2l-.6 2.8h4.2l-.6-2.8c2.5-.4 5.1-1.2 7.5-2.2l-1.4-2M20 8.6l1.6 1.8M20 8.6L22 8l-8.6 2.6c-.2-.7-.4-1.4-1.4-1.4M13.3 7.2a1.3 1.3 0 1 1-2.6 0 1.3 1.3 0 0 1 2.6 0z'/>",
  "chat": "<path d='M5 6.5h14a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1h-7l-4 3v-3H5a1 1 0 0 1-1-1v-8a1 1 0 0 1 1-1z'/>",
  "folder": "<path d='M4 6.5a1 1 0 0 1 1-1h4.5l2 2H19a1 1 0 0 1 1 1V17a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1z'/>",
  "activity": "<path d='M9 7h10M9 12h10M9 17h10M5 7h.01M5 12h.01M5 17h.01'/>",
  "pull": "<path d='M6 3.5a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM6 7.5v9M6 16.5a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM18 16.5a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM18 16.5V9a3 3 0 0 0-3-3h-4.5M13 3.5L10.5 6 13 8.5'/>",
  "gear": "<path d='M10.21 2.77L13.79 2.77L14.21 5.57L16.46 6.87L19.09 5.83L20.89 8.94L18.68 10.70L18.68 13.30L20.89 15.06L19.09 18.17L16.46 17.13L14.21 18.43L13.79 21.23L10.21 21.23L9.79 18.43L7.54 17.13L4.91 18.17L3.11 15.06L5.32 13.30L5.32 10.70L3.11 8.94L4.91 5.83L7.54 6.87L9.79 5.57ZM15.00 12a3.0 3.0 0 1 0-6.0 0a3.0 3.0 0 1 0 6.0 0z'/>",
  "plug": "<path d='M7 8h10v3a5 5 0 0 1-10 0V8M9 3v5m6-5v5M12 16v4H8'/>",
  "note": "<path d='M9 18V5l11-2v13M9 18a3 3 0 1 1-6 0 3 3 0 0 1 6 0zm11-2a3 3 0 1 1-6 0 3 3 0 0 1 6 0z'/>",
  "moon": "<path d='M20 14.5A8 8 0 0 1 9.5 4 8 8 0 1 0 20 14.5z'/>",
  "sound": "<path d='M4 9h4l5-4v14l-5-4H4zM16 9a4 4 0 0 1 0 6M18.5 6.5a7.5 7.5 0 0 1 0 11'/>",
  "mute": "<path d='M4 9h4l5-4v14l-5-4H4zM17 9l5 6m0-6-5 6'/>",
  "person": "<path d='M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM4 21a8 8 0 0 1 16 0'/>",
  "drop": "<path d='M12 5v14M5 12h14'/>",
  "phone": "<rect x='7' y='3' width='10' height='18' rx='2'/><path d='M11 18h2'/>",
  // Tasks, which the prototype does not draw: its list, each line ticked.
  "tasks": "<path d='M10 7h9M10 12h9M10 17h9'/><path d='m4 7 1.2 1.2L7.5 6M4 12l1.2 1.2L7.5 11M4 17l1.2 1.2L7.5 16'/>",
  // The header's search, from the prototype's inline 16 x 16 drawing, at this table's size.
  "search": "<circle cx='10.5' cy='10.5' r='6.75'/><path d='M15.75 15.75 21 21'/>",
} as const;

export type IconName = keyof typeof ICONS;

const NS = "http://www.w3.org/2000/svg";

/** An inline icon, hidden from screen readers: the button around it carries the words. */
export function icon(name: IconName, size = 14): SVGSVGElement {
  const svg = document.createElementNS(NS, "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("width", String(size));
  svg.setAttribute("height", String(size));
  svg.setAttribute("aria-hidden", "true");
  svg.setAttribute("class", "icon");
  svg.setAttribute("fill", "none");
  svg.setAttribute("stroke", "currentColor");
  svg.setAttribute("stroke-width", "1.6");
  svg.setAttribute("stroke-linecap", "round");
  svg.setAttribute("stroke-linejoin", "round");
  // The paths come from this file only, never from a page's data.
  svg.innerHTML = ICONS[name];
  return svg;
}
