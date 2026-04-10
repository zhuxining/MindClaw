import { create } from "zustand";
import type { AgentPhase, ChatMessage, ConversationMode } from "@/lib/types";

interface ChatState {
	currentSessionId: string | null;
	mode: ConversationMode;
	messages: ChatMessage[];
	streamingRequestId: string | null;
}

interface ChatActions {
	setMode: (mode: ConversationMode) => void;
	setSessionId: (id: string | null) => void;
	addUserMessage: (content: string, id: string) => void;
	startStreaming: (requestId: string) => void;
	appendChunk: (requestId: string, chunk: string) => void;
	setPhase: (requestId: string, phase: AgentPhase) => void;
	completeStreaming: (requestId: string) => void;
	setError: (requestId: string, message: string) => void;
	clearMessages: () => void;
}

export const useChatStore = create<ChatState & ChatActions>()((set) => ({
	currentSessionId: null,
	mode: "companion",
	messages: [],
	streamingRequestId: null,

	setMode: (mode) => set({ mode }),
	setSessionId: (id) => set({ currentSessionId: id }),

	addUserMessage: (content, id) =>
		set((s) => ({
			messages: [
				...s.messages,
				{
					type: "user",
					id,
					content,
					timestamp: Date.now(),
				},
			],
		})),

	startStreaming: (requestId) =>
		set((s) => ({
			streamingRequestId: requestId,
			messages: [
				...s.messages,
				{
					type: "agent",
					id: `agent-${requestId}`,
					requestId,
					content: "",
					phase: "thinking" as AgentPhase,
					isStreaming: true,
					timestamp: Date.now(),
				},
			],
		})),

	appendChunk: (requestId, chunk) =>
		set((s) => ({
			messages: s.messages.map((m) =>
				m.type === "agent" && m.requestId === requestId
					? {
							...m,
							content: m.content + chunk,
							phase: "streaming" as AgentPhase,
						}
					: m,
			),
		})),

	setPhase: (requestId, phase) =>
		set((s) => ({
			messages: s.messages.map((m) =>
				m.type === "agent" && m.requestId === requestId ? { ...m, phase } : m,
			),
		})),

	completeStreaming: (requestId) =>
		set((s) => ({
			streamingRequestId: null,
			messages: s.messages.map((m) =>
				m.type === "agent" && m.requestId === requestId
					? { ...m, phase: "completed" as AgentPhase, isStreaming: false }
					: m,
			),
		})),

	setError: (requestId, message) =>
		set((s) => ({
			streamingRequestId: null,
			messages: s.messages.map((m) =>
				m.type === "agent" && m.requestId === requestId
					? {
							...m,
							content: message,
							phase: "error" as AgentPhase,
							isStreaming: false,
						}
					: m,
			),
		})),

	clearMessages: () => set({ messages: [], streamingRequestId: null }),
}));
