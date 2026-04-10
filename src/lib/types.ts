// ─── Task ────────────────────────────────────────────────────────────────────

export type TaskStatus = "todo" | "in_progress" | "done" | "cancelled";
export type TaskPriority = "low" | "medium" | "high";

export interface Task {
	id: string;
	title: string;
	status: TaskStatus;
	priority: TaskPriority;
	due_date: string | null;
	tags: string[];
	created: string;
	updated: string;
	body: string | null;
}

export interface CreateTaskParams {
	title: string;
	body?: string;
	priority?: TaskPriority;
	due_date?: string;
	tags?: string[];
}

// ─── Settings ────────────────────────────────────────────────────────────────

export type UserRole = "professional" | "student" | "researcher" | "creator";
export type ModelTierPreference = "economy" | "quality" | "auto";

export interface AgentPreference {
	provider: string;
	model_id: string | null;
	model_tier: ModelTierPreference;
	max_tokens_per_turn: number;
	enable_memory: boolean;
	enable_tools: boolean;
}

export interface AppSettings {
	vault_path: string;
	user_role: UserRole | null;
	agent: AgentPreference;
	language: string;
}

// ─── Vault / Directory ───────────────────────────────────────────────────────

export interface VaultEntry {
	name: string;
	/** 相对于 vault 根目录的路径 */
	path: string;
	is_dir: boolean;
	/** Unix 毫秒时间戳 */
	modified_ms: number;
}

// ─── Workspace ────────────────────────────────────────────────────────────────

export type BuiltinTabId = "daily" | "private" | "vault" | "source";

export interface PinnedDirTab {
	id: string;
	/** 相对于 vault 根目录的路径，或以 "/" 开头的绝对路径 */
	dirPath: string;
	label: string;
}

export type DirectoryViewMode = "tree" | "flat";

export interface DailyItem {
	type: "daily";
	date: string; // YYYY-MM-DD
	path: string;
}

export interface NoteItem {
	type: "note";
	path: string;
	title: string;
}

export interface SourceItem {
	type: "source";
	path: string;
	sourceType: "link" | "pdf";
	url?: string;
}

export type OpenedItem = DailyItem | NoteItem | SourceItem;

// ─── Chat ─────────────────────────────────────────────────────────────────────

export type AgentPhase =
	| "thinking"
	| "using_tools"
	| "streaming"
	| "completed"
	| "cancelled"
	| "error";

export type ConversationMode =
	| "companion"
	| "reflection"
	| "challenge"
	| "knowledge"
	| "vault";

export interface UserMessage {
	type: "user";
	id: string;
	content: string;
	timestamp: number;
}

export interface AgentMessage {
	type: "agent";
	id: string;
	requestId: string;
	content: string;
	phase: AgentPhase;
	isStreaming: boolean;
	timestamp: number;
}

export type ChatMessage = UserMessage | AgentMessage;

// ─── Agent Events ─────────────────────────────────────────────────────────────

export type AgentEventPayload =
	| { type: "Chunk"; data: { content: string } }
	| { type: "Done" }
	| { type: "Error"; data: { message: string; retryable: boolean } }
	| { type: "Status"; data: { status: AgentPhase } };

export interface AgentOutboundEvent {
	id: string;
	request_id: string;
	session_id: string;
	payload: AgentEventPayload;
}

// ─── Knowledge ────────────────────────────────────────────────────────────────

export interface KnowledgeEntry {
	id: string;
	title: string;
	topic: string;
	content: string;
	wikilinks: string[];
	tags: string[];
	source_url: string | null;
	created_at: number;
	updated_at: number;
}
