export default function auditHitTargets() {
  const scroll = Array.from(document.querySelectorAll('*'))
    .filter(e => e.scrollHeight > e.clientHeight || e.scrollWidth > e.clientWidth)
    .map(e => [e, e.scrollLeft, e.scrollTop]);
  const failures = [], clipped = [];
  let checked = 0;
  for (const element of document.querySelectorAll('button,input,select,textarea,a[href],summary')) {
    const style = getComputedStyle(element);
    if (element.checkVisibility && !element.checkVisibility({opacityProperty:true,visibilityProperty:true,contentVisibilityAuto:true})) continue;
    let inClosedContent = false;
    for (let parent = element.parentElement; parent; parent = parent.parentElement) {
      if (parent.tagName === 'DETAILS' && !parent.open && !parent.querySelector(':scope > summary')?.contains(element)) inClosedContent = true;
    }
    if (inClosedContent) continue;
    if (element.disabled || style.visibility === 'hidden' || style.pointerEvents === 'none' || Number(style.opacity) === 0 || element.closest('[hidden],[inert]')) continue;
    let rect = element.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) continue;
    const name = (element.getAttribute('aria-label') || element.title || element.innerText || element.getAttribute('name') || element.tagName).slice(0,70);
    if (element.closest('.app-header') && (rect.left < 0 || rect.right > innerWidth || rect.top < 0 || rect.bottom > innerHeight)) clipped.push(name);
    element.scrollIntoView({block:'nearest',inline:'nearest'});
    rect = element.getBoundingClientRect();
    const box = {left:Math.max(0,rect.left),top:Math.max(0,rect.top),right:Math.min(innerWidth,rect.right),bottom:Math.min(innerHeight,rect.bottom)};
    for (let parent = element.parentElement; parent; parent = parent.parentElement) {
      const parentStyle = getComputedStyle(parent), bounds = parent.getBoundingClientRect();
      if (/auto|scroll|hidden|clip/.test(parentStyle.overflowX)) {
        box.left = Math.max(box.left,bounds.left+parent.clientLeft);
        box.right = Math.min(box.right,bounds.left+parent.clientLeft+parent.clientWidth);
      }
      if (/auto|scroll|hidden|clip/.test(parentStyle.overflowY)) {
        box.top = Math.max(box.top,bounds.top+parent.clientTop);
        box.bottom = Math.min(box.bottom,bounds.top+parent.clientTop+parent.clientHeight);
      }
    }
    if (box.right-box.left<8 || box.bottom-box.top<8) continue;
    checked++;
    const hit = document.elementFromPoint((box.left+box.right)/2,(box.top+box.bottom)/2);
    if (hit && !element.contains(hit)) failures.push({name,blocker:hit.tagName+'.'+hit.className});
  }
  for (const [element, left, top] of scroll) { element.scrollLeft = left; element.scrollTop = top; }
  return {checked,failed:failures,headerClipped:clipped,overflow:document.documentElement.scrollWidth>innerWidth+1,h1:document.querySelector('h1')?.textContent};
}
