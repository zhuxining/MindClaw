import { listen } from "@tauri-apps/api/event";
import type { AgentOutboundEvent } from "./types";

export const EVENT_AGENT = "mindclaw://agent-event";

export async function listenAgentEvents(
	callback: (event: AgentOutboundEvent) => void,
) {
	return listen<AgentOutboundEvent>(EVENT_AGENT, (e) => callback(e.payload));
}
