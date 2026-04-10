import { useEffect } from "react";
import { listenAgentEvents } from "@/lib/events";
import type { AgentOutboundEvent } from "@/lib/types";
import { useChatStore } from "@/stores/chat";

export function useAgentEvents() {
	const appendChunk = useChatStore((s) => s.appendChunk);
	const setPhase = useChatStore((s) => s.setPhase);
	const completeStreaming = useChatStore((s) => s.completeStreaming);
	const setError = useChatStore((s) => s.setError);
	const setSessionId = useChatStore((s) => s.setSessionId);

	useEffect(() => {
		let cancelled = false;
		let unlisten: (() => void) | undefined;

		listenAgentEvents((event: AgentOutboundEvent) => {
			const { request_id, session_id, payload } = event;

			setSessionId(session_id);

			switch (payload.type) {
				case "Chunk":
					appendChunk(request_id, payload.data.content);
					break;
				case "Status":
					setPhase(request_id, payload.data.status);
					break;
				case "Done":
					completeStreaming(request_id);
					break;
				case "Error":
					setError(request_id, payload.data.message);
					break;
			}
		}).then((fn) => {
			if (cancelled) {
				// Component already unmounted — clean up immediately
				fn();
			} else {
				unlisten = fn;
			}
		});

		return () => {
			cancelled = true;
			unlisten?.();
		};
	}, [appendChunk, setPhase, completeStreaming, setError, setSessionId]);
}
