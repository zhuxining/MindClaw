import { create } from "zustand";
import type { AgentResponse, ChannelMessage, RouteRule } from "../lib/types";

interface MessageState {
	/// 消息列表
	messages: ChannelMessage[];
	/// 消息处理状态映射 (message_id → AgentResponse)
	processingResults: Record<string, AgentResponse>;
	/// 路由规则
	routeRules: RouteRule[];
	/// 飞书连接状态（deprecated，v2 迁移到 channelStatuses）
	feishuConnected: boolean;
	/// 渠道连接状态映射 (channel_name → connected)
	channelStatuses: Record<string, boolean>;
	/// 是否正在轮询
	isPolling: boolean;
	/// 轮询间隔（秒）
	pollInterval: number;
	/// 是否自动回复飞书
	autoReply: boolean;

	// Actions
	setMessages: (messages: ChannelMessage[]) => void;
	addMessages: (messages: ChannelMessage[]) => void;
	setProcessingResult: (messageId: string, result: AgentResponse) => void;
	setRouteRules: (rules: RouteRule[]) => void;
	addRouteRule: (rule: RouteRule) => void;
	removeRouteRule: (ruleId: string) => void;
	setFeishuConnected: (connected: boolean) => void;
	setChannelConnected: (channel: string, connected: boolean) => void;
	setPolling: (polling: boolean) => void;
	setPollInterval: (interval: number) => void;
	setAutoReply: (autoReply: boolean) => void;
	clearMessages: () => void;
}

export const useMessageStore = create<MessageState>((set) => ({
	messages: [],
	processingResults: {},
	routeRules: [],
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
	setRouteRules: (routeRules) => set({ routeRules }),
	addRouteRule: (rule) =>
		set((state) => {
			const filtered = state.routeRules.filter(
				(r) => r.rule_id !== rule.rule_id,
			);
			return { routeRules: [...filtered, rule] };
		}),
	removeRouteRule: (ruleId) =>
		set((state) => ({
			routeRules: state.routeRules.filter((r) => r.rule_id !== ruleId),
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
