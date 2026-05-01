import { invoke } from "@tauri-apps/api/core";
import type {
	AppSettings,
	CreateTaskParams,
	OpenedItem,
	Task,
	VaultEntry,
	VaultNote,
	WorkspacePrefs,
} from "./types";

// biome-ignore lint/suspicious/noExplicitAny: invoke args can be any shape
async function call<T>(cmd: string, args?: any): Promise<T> {
	try {
		return await invoke<T>(cmd, args);
	} catch (err) {
		const message =
			typeof err === "string"
				? err
				: ((err as { message?: string })?.message ?? "Unknown error");
		throw new Error(message);
	}
}

function normalizePath(path: string): string {
	return path.replace(/\.\./g, "").replace(/\/+/g, "/");
}

export const ipc = {
	// ─── Conversation ──────────────────────────────────────────────────────────
	sendMessage: (content: string, sessionId?: string, mode?: string) =>
		call<string>("send_message", {
			content,
			session_id: sessionId,
			mode,
		}),

	getSessionHistory: (sessionId: string) =>
		call<string[]>("get_session_history", { session_id: sessionId }),

	// ─── Tasks ─────────────────────────────────────────────────────────────────
	listTasks: (status?: string) => call<Task[]>("list_tasks", { status }),

	createTask: (params: CreateTaskParams) => call<Task>("create_task", params),

	getTask: (id: string) => call<Task>("get_task", { id }),

	updateTask: (
		id: string,
		params: Partial<CreateTaskParams & { status: string }>,
	) => call<Task>("update_task", { id, ...params }),

	deleteTask: (id: string) => call<void>("delete_task", { id }),

	updateTaskStatus: (id: string, status: string) =>
		call<void>("update_task_status", { id, status }),

	// ─── Daily & Notes ─────────────────────────────────────────────────────────
	getDaily: (date: string) => call<string>("get_daily", { date }),

	saveDaily: (date: string, content: string) =>
		call<void>("save_daily", { date, content }),

	readNote: (path: string) => call<string>("read_note", { path }),

	saveNote: (path: string, content: string) =>
		call<void>("save_note", { path, content }),

	// ─── Vault Notes ──────────────────────────────────────────────────────────
	searchVault: (query: string) => call<VaultNote[]>("search_vault", { query }),

	getRelevantNotes: (path: string) =>
		call<VaultNote[]>("get_relevant_notes", { path }),

	// ─── Vault ─────────────────────────────────────────────────────────────────
	listVaultDir: (path?: string) =>
		call<VaultEntry[]>("list_vault_dir", { path }),

	listVaultFilesRecursive: (path?: string) =>
		call<VaultEntry[]>("list_vault_files_recursive", {
			path: path ? normalizePath(path) : "",
		}),

	resolveSourceItem: (path: string) =>
		call<OpenedItem>("resolve_source_item", { path: normalizePath(path) }),

	getVaultNote: (id: string) => call<VaultNote>("get_vault_note", { id }),

	// ─── Settings ──────────────────────────────────────────────────────────────
	getSettings: () => call<AppSettings>("get_settings"),

	getWorkspacePrefs: () => call<WorkspacePrefs>("get_workspace_prefs"),

	saveSettings: (settings: AppSettings) =>
		call<void>("save_settings", { settings }),

	saveWorkspacePrefs: (prefs: WorkspacePrefs) =>
		call<void>("save_workspace_prefs", { prefs }),

	setVault: (path: string) => call<void>("set_vault", { path }),

	setApiKey: (key: string) => call<void>("set_api_key", { key }),
};
