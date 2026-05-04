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

export type DirectoryViewMode = "tree" | "flat";

export interface WorkspacePanelSizes {
	left: number;
	center: number;
	right: number;
}

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

export interface SourceWebItem {
	type: "source-web";
	path: string;
	title: string;
	url: string;
}

export interface SourcePdfItem {
	type: "source-pdf";
	path: string;
	title: string;
}

export interface SourceImageItem {
	type: "source-image";
	path: string;
	title: string;
}

export type OpenedItem =
	| DailyItem
	| NoteItem
	| SourceWebItem
	| SourcePdfItem
	| SourceImageItem;

export interface WorkspacePrefs {
	active_workspace_id: WorkspaceId;
	open_tabs: OpenTab[];
	active_tab_id: string | null;
	panel_sizes: WorkspacePanelSizes;
	last_opened_item: OpenedItem | null;
}

export type EditorSaveState = "idle" | "saving" | "saved" | "error";

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
	| "vault"
	| "private";

export interface SessionListItem {
	id: string;
	sender: string;
	mode: ConversationMode;
	created: string;
	updated: string;
}

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

// ─── Vault Notes ──────────────────────────────────────────────────────────────

export interface VaultNote {
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

// ─── Memory ────────────────────────────────────────────────────────────────────

export interface MemoryListItem {
	id: string;
	key: string;
	category: string;
	importance: number;
	file_path: string;
	updated: string;
}

// ─── Skills ────────────────────────────────────────────────────────────────────

export interface SkillMetadata {
	name: string;
	description: string;
	always_load: boolean;
	path: string;
}

export interface SkillManifest extends SkillMetadata {
	license?: string;
	compatibility?: string;
	metadata: Record<string, string>;
	allowed_tools: string[];
}

// ─── Workspace Shell ──────────────────────────────────────────────────────────

export type WorkspaceId =
	| "daily"
	| "inbox"
	| "private"
	| "vault"
	| "agent"
	| "skills"
	| "memory"
	| "mcp"
	| "session"
	| "cron"
	| "checklist"
	| "graph"
	| "tasks"
	| "settings";

export interface StatusBarState {
	saveState: EditorSaveState;
	lineCol: string;
	encoding: string;
}

// ─── Pane System ──────────────────────────────────────────────────────────────

export type LeftPaneId =
	| "calendar-filter"
	| "tags-filter"
	| "type-filter"
	| "saved-filter"
	| "file-explorer"
	| "agent-list"
	| "skill-list"
	| "memory-list"
	| "mcp-server-list"
	| "session-list"
	| "cron-job-list";

export type RightPaneId = "note-outline" | "note-frontmatter" | "related-files";

export interface PaneState {
	activePaneId: string;
	scrollPosition: number;
	filterParams: Record<string, string>;
}

// ─── Content Host / Tabs ──────────────────────────────────────────────────────

export type ContentType =
	| "daily-note"
	| "markdown"
	| "web"
	| "pdf"
	| "image"
	| "agent-session"
	| "agent-detail"
	| "skill-detail"
	| "memory-detail"
	| "mcp-detail"
	| "session-detail"
	| "cron-detail"
	| "checklist"
	| "graph"
	| "settings";

export interface ContentDescriptor {
	type: ContentType;
	path: string;
	title: string;
	/** Additional metadata depending on content type */
	meta?: Record<string, unknown>;
}

export interface OpenTab {
	id: string;
	descriptor: ContentDescriptor;
	dirty: boolean;
}

// ─── OpenedItem ↔ ContentDescriptor Conversion ────────────────────────────────

/** Convert legacy OpenedItem to ContentDescriptor */
export function openedItemToDescriptor(item: OpenedItem): ContentDescriptor {
	switch (item.type) {
		case "daily":
			return {
				type: "daily-note",
				path: item.path,
				title: item.date,
				meta: { date: item.date },
			};
		case "note":
			return {
				type: "markdown",
				path: item.path,
				title: item.title,
			};
		case "source-web":
			return {
				type: "web",
				path: item.path,
				title: item.title,
				meta: { url: item.url },
			};
		case "source-pdf":
			return {
				type: "pdf",
				path: item.path,
				title: item.title,
			};
		case "source-image":
			return {
				type: "image",
				path: item.path,
				title: item.title,
			};
	}
}

/** Convert ContentDescriptor to legacy OpenedItem (for backward compatibility) */
export function descriptorToOpenedItem(
	descriptor: ContentDescriptor,
): OpenedItem | null {
	switch (descriptor.type) {
		case "daily-note":
			return {
				type: "daily",
				date: (descriptor.meta?.date as string) ?? descriptor.title,
				path: descriptor.path,
			};
		case "markdown":
			return {
				type: "note",
				path: descriptor.path,
				title: descriptor.title,
			};
		case "web":
			return {
				type: "source-web",
				path: descriptor.path,
				title: descriptor.title,
				url: (descriptor.meta?.url as string) ?? "",
			};
		case "pdf":
			return {
				type: "source-pdf",
				path: descriptor.path,
				title: descriptor.title,
			};
		case "image":
			return {
				type: "source-image",
				path: descriptor.path,
				title: descriptor.title,
			};
		default:
			// agent-session, agent-detail, etc. not supported as OpenedItem
			return null;
	}
}
