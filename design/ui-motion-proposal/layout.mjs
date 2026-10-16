export const SURFACE_SIZES = Object.freeze({
  mini: { width:520, height:38, radius:20 },
  quick: { width:520, height:440, radius:22 },
  room: { width:1120, height:760, radius:16 },
  full: { width:1440, height:900, radius:12 },
});

export function fitSurface(mode, availableWidth, availableHeight) {
  const size=SURFACE_SIZES[mode]??SURFACE_SIZES.quick;
  return {...size,width:Math.max(0,Math.min(size.width,Math.floor(availableWidth))),height:Math.max(0,Math.min(size.height,Math.floor(availableHeight)))};
}

export function placePopover(surface, anchor, contentHeight, headerBottom, room=false) {
  const margin=12,gap=8,top=Math.min(surface.height-margin,Math.max(margin,headerBottom-surface.top+gap));
  const bottom=Math.max(top,surface.height-margin),heightLimit=Math.max(0,bottom-top);
  const width=Math.max(0,Math.min(room?400:surface.width-2*margin,surface.width-2*margin));
  const above=Math.max(0,anchor.top-surface.top-gap-top),below=Math.max(0,bottom-(anchor.bottom-surface.top+gap));
  const height=Math.min(contentHeight,312,heightLimit);
  const opensAbove=above>=height||above>=below;
  const available=Math.max(above,below);
  const maxHeight=Math.min(heightLimit,available<120?heightLimit:Math.min(312,opensAbove?above:below));
  const actual=Math.min(height,maxHeight);
  const preferred=opensAbove?anchor.top-surface.top-gap-actual:anchor.bottom-surface.top+gap;
  return {left:Math.max(margin,Math.min(anchor.left-surface.left,surface.width-margin-width)),top:Math.max(top,Math.min(preferred,bottom-actual)),width,maxHeight,origin:opensAbove?'bottom left':'top left'};
}

export function surfaceFrames(from,to) {
  return [from,to].map(size=>({width:size.width+'px',height:size.height+'px',borderRadius:size.radius+'px'}));
}
