import { create } from "zustand";
import type {
	AcpServer,
	Agent,
	AgentResponse,
	ChannelMessage,
	ConversationExecutionState,
	Skill,
	SlashCommand,
} from "../lib/types";

interface MessageState {
	messages: ChannelMessage[];
	processingResults: Record<string, AgentResponse>;
	acpServers: AcpServer[];
	agents: Agent[];
	skills: Skill[];
	slashCommands: SlashCommand[];
	conversationStates: Record<string, ConversationExecutionState>;
	feishuConnected: boolean;
	channelStatuses: Record<string, boolean>;
	isPolling: boolean;
	pollInterval: number;
	autoReply: boolean;

	setMessages: (messages: ChannelMessage[]) => void;
	addMessages: (messages: ChannelMessage[]) => void;
	setProcessingResult: (messageId: string, result: AgentResponse) => void;
	setAcpServers: (servers: AcpServer[]) => void;
	setAgents: (agents: Agent[]) => void;
	setSkills: (skills: Skill[]) => void;
	setSlashCommands: (commands: SlashCommand[]) => void;
	setConversationState: (state: ConversationExecutionState) => void;
	setFeishuConnected: (connected: boolean) => void;
	setChannelConnected: (channel: string, connected: boolean) => void;
	setPolling: (polling: boolean) => void;
	setPollInterval: (interval: number) => void;
	setAutoReply: (autoReply: boolean) => void;
	clearMessages: () => void;
}

function conversationKey(state: ConversationExecutionState): string {
	return `${state.key.channel}:${state.key.conversation_id}`;
}

export const useMessageStore = create<MessageState>((set) => ({
	messages: [],
	processingResults: {},
	acpServers: [],
	agents: [],
	skills: [],
	slashCommands: [],
	conversationStates: {},
	feishuConnected: false,
	channelStatuses: {},
	isPolling: false,
	pollInterval: 30,
	autoReply: true,

	setMessages: (messages) => set({ messages }),
	addMessages: (newMessages) =>
		set((state) => {
			const existingIds = new Set(state.messages.map((m) => m.message_id));
			const filtered = newMessages.filter(
				(m) => !existingIds.has(m.message_id),
			);
			return { messages: [...filtered, ...state.messages] };
		}),
	setProcessingResult: (messageId, result) =>
		set((state) => ({
			processingResults: {
				...state.processingResults,
				[messageId]: result,
			},
		})),
	setAcpServers: (acpServers) => set({ acpServers }),
	setAgents: (agents) => set({ agents }),
	setSkills: (skills) => set({ skills }),
	setSlashCommands: (slashCommands) => set({ slashCommands }),
	setConversationState: (conversationState) =>
		set((state) => ({
			conversationStates: {
				...state.conversationStates,
				[conversationKey(conversationState)]: conversationState,
			},
		})),
	setFeishuConnected: (connected) =>
		set((state) => ({
			feishuConnected: connected,
			channelStatuses: { ...state.channelStatuses, feishu: connected },
		})),
	setChannelConnected: (channel, connected) =>
		set((state) => ({
			channelStatuses: { ...state.channelStatuses, [channel]: connected },
		})),
	setPolling: (isPolling) => set({ isPolling }),
	setPollInterval: (pollInterval) => set({ pollInterval }),
	setAutoReply: (autoReply) => set({ autoReply }),
	clearMessages: () => set({ messages: [], processingResults: {} }),
}));
