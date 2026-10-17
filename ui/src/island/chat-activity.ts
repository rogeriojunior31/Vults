import type { ChatActivity, ChatActivityEvent } from "../bridge";

export const emptyActivity = (): ChatActivity => ({ reasoning: "", tools: [], decisions: [], truncated: false });

/** Presentation only: this data never goes to decide(), send() or a tool runner. */
export function applyActivity(activity: ChatActivity, event: ChatActivityEvent): void {
  if (event.type === "reasoning") {
    const remaining = Math.max(0, 32768 - [...activity.reasoning].length);
    activity.reasoning += [...event.text].slice(0, remaining).join("");
    activity.truncated ||= [...event.text].length > remaining;
  } else if (event.type === "start") {
    if (activity.tools.some(t => t.id === event.id)) return;
    if (activity.tools.length >= 128) { activity.truncated = true; return; }
    activity.truncated ||= event.detail.length > 16384;
    activity.tools.push({ id: event.id, tool: event.tool, detail: event.detail.slice(0, 16384), ok: null });
  } else if (event.type === "end") {
    const tool = activity.tools.find(t => t.id === event.id);
    if (tool) tool.ok = event.ok;
  } else {
    if (activity.decisions.length >= 128) { activity.truncated = true; return; }
    activity.decisions.push({ tool: event.tool, target: event.target.slice(0, 16384), answer: event.answer, by: event.by });
  }
}

export function readActivity(text: string): ChatActivity {
  const result = emptyActivity();
  try {
    const data = JSON.parse(text);
    if (typeof data.reasoning === "string") applyActivity(result, { type: "reasoning", text: data.reasoning });
    if (Array.isArray(data.tools)) for (const tool of data.tools.slice(0, 128)) {
      if (typeof tool.id !== "string" || typeof tool.tool !== "string" || typeof tool.detail !== "string") continue;
      applyActivity(result, { type: "start", id: tool.id, tool: tool.tool, detail: tool.detail });
      if (typeof tool.ok === "boolean") applyActivity(result, { type: "end", id: tool.id, ok: tool.ok });
    }
    if (Array.isArray(data.decisions)) for (const d of data.decisions.slice(0, 128)) {
      if ([d.tool, d.target, d.answer, d.by].every(value => typeof value === "string") && ["allow", "deny", "always"].includes(d.answer) && ["human", "policy", "full-access", "conversation"].includes(d.by)) applyActivity(result, { type: "decision", tool: d.tool, target: d.target, answer: d.answer, by: d.by });
    }
    result.truncated ||= data.truncated === true;
  } catch { result.truncated = true; }
  return result;
}
