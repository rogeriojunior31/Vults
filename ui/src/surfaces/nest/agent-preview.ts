// The proposal shares the production workspace and never receives native write callbacks.
import type { AgentProfile } from "../../bridge";
import { createAgentWorkspace } from "./agent-workspace";
export function createAgentPreview(presets:AgentProfile[],back:()=>void,start:"catalogue"|"editor"|"chat"="catalogue") {
  return createAgentWorkspace(presets,back,undefined,start);
}
