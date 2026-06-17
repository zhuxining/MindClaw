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

export interface AgentResponse {
	request_id: string;
	status: "success" | "error" | "timeout";
	output: string;
	error_message: string | null;
}

export interface EnvVar {
	name: string;
	value: string;
}

export interface AcpServer {
	id: string;
	name: string;
	description: string;
	command: string;
	args: string[];
	env_vars: EnvVar[];
	timeout_secs: number;
	enabled: boolean;
}

export interface Identity {
	system_prompt: string;
	style: string | null;
	safety_policy: string | null;
}

export interface Agent {
	id: string;
	name: string;
	description: string;
	identity: Identity;
	default_acp_server_id: string;
	default_skill_id: string | null;
	enabled: boolean;
}

export interface Skill {
	id: string;
	name: string;
	description: string;
	instruction: string;
	enabled: boolean;
}

export interface SlashCommand {
	command: string;
	description: string;
	agent_id: string;
	skill_id: string | null;
	scope: "one_shot" | "sticky_conversation";
	enabled: boolean;
}

export interface ConversationKey {
	channel: string;
	conversation_id: string;
}

export interface ConversationExecutionState {
	key: ConversationKey;
	agent_id: string;
	skill_id: string | null;
}

export interface RegistryAgent {
	id: string;
	name: string;
	version: string;
	description: string;
	repository?: string;
	website?: string;
	authors: string[];
	license: string;
	icon?: string;
	distribution: {
		npx?: {
			package: string;
			args?: string[];
			env?: Record<string, string>;
		};
		binary?: {
			[key: string]: {
				archive: string;
				cmd: string;
			};
		};
	};
}

export interface AcpRegistry {
	version: string;
	agents: RegistryAgent[];
}

export interface ChannelConfig {
	enabled: boolean;
	poll_interval_secs: number;
	page_size: number;
	auto_reply: boolean;
	extra: unknown;
}

export interface AppConfig {
	channels: Record<string, ChannelConfig>;
	acp_servers: AcpServer[];
	agents: Agent[];
	skills: Skill[];
	slash_commands: SlashCommand[];
	default_agent_id: string;
}
