import { create } from "zustand";
import type {
	AcpServer,
	Agent,
	AgentResponse,
	ChannelDescriptor,
	ChannelMessage,
	ChannelStatus,
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
	channelDescriptors: ChannelDescriptor[];
	channelStatuses: Record<string, boolean>;
	runtimeStatuses: Record<string, ChannelStatus>;
	feishuConnected: boolean;

	setMessages: (messages: ChannelMessage[]) => void;
	addMessages: (messages: ChannelMessage[]) => void;
	setProcessingResult: (messageId: string, result: AgentResponse) => void;
	setAcpServers: (servers: AcpServer[]) => void;
	setAgents: (agents: Agent[]) => void;
	setSkills: (skills: Skill[]) => void;
	setSlashCommands: (commands: SlashCommand[]) => void;
	setConversationState: (state: ConversationExecutionState) => void;
	setChannelDescriptors: (descriptors: ChannelDescriptor[]) => void;
	setChannelConnected: (channel: string, connected: boolean) => void;
	setRuntimeStatuses: (statuses: ChannelStatus[]) => void;
	setFeishuConnected: (connected: boolean) => void;
	clearMessages: () => void;
	connected: (channel: string) => boolean;
}

function conversationKey(state: ConversationExecutionState): string {
	return `${state.key.channel}:${state.key.conversation_id}`;
}

export const useMessageStore = create<MessageState>((set, get) => ({
	messages: [],
	processingResults: {},
	acpServers: [],
	agents: [],
	skills: [],
	slashCommands: [],
	conversationStates: {},
	channelDescriptors: [],
	channelStatuses: {},
	runtimeStatuses: {},
	feishuConnected: false,

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
	setChannelDescriptors: (channelDescriptors) => set({ channelDescriptors }),
	setRuntimeStatuses: (runtimeStatuses) =>
		set({
			runtimeStatuses: Object.fromEntries(
				runtimeStatuses.map((s) => [s.channel, s]),
			),
		}),
	connected: (channel) =>
		get().channelStatuses[channel] ?? get().feishuConnected,
	clearMessages: () => set({ messages: [], processingResults: {} }),
}));
