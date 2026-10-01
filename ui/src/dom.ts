/** Builds an element; text goes through textContent, never innerHTML. */
export function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  props: { class?: string; text?: string; onclick?: (() => void) | undefined } = {},
  ...children: (Node | null)[]
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (props.class) node.className = props.class;
  if (props.text !== undefined) node.textContent = props.text;
  if (props.onclick) node.addEventListener("click", props.onclick);
  for (const child of children) if (child) node.append(child);
  return node;
}
