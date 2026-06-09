/// 统一渠道消息（匹配 Rust ChannelMessage）
export interface ChannelMessage {
	message_id: string;
	channel: string;
	conversation_id: string;
	sender_id: string;
	sender_name: string;
	content: string;
	timestamp: number;
	is_reply: boolean;
	reply_to: string | null;
}

/// 路由匹配条件
export interface RouteCondition {
	channel: string | null;
	sender_id: string | null;
	keywords: string[] | null;
	keyword_mode: "contains" | "not_contains";
}

/// 消息路由规则
export interface RouteRule {
	rule_id: string;
	name: string;
	condition: RouteCondition;
	agent_id: string;
	priority: number;
	enabled: boolean;
}

/// Agent 处理结果
export interface AgentResponse {
	request_id: string;
	status: "success" | "error" | "timeout";
	output: string;
	error_message: string | null;
}

/// 飞书配置
export interface FeishuConfig {
	poll_interval_secs: number;
	page_size: number;
	auto_reply: boolean;
}

/// 渠道通用配置
export interface ChannelConfig {
	enabled: boolean;
	poll_interval_secs: number;
	page_size: number;
	auto_reply: boolean;
	extra: unknown;
}

/// 渠道连接状态
export interface ChannelStatus {
	name: string;
	display_name: string;
	icon: string;
	connected: boolean;
	enabled: boolean;
}

/// 应用配置
export interface AppConfig {
	feishu: FeishuConfig;
	channels: Record<string, ChannelConfig>;
}
