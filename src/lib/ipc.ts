import { invoke } from "@tauri-apps/api/core";
import type {
	AppSettings,
	CreateTaskParams,
	KnowledgeEntry,
	Task,
	VaultEntry,
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

	updateTaskStatus: (id: string, status: string) =>
		call<void>("update_task_status", { id, status }),

	// ─── Daily & Notes ─────────────────────────────────────────────────────────
	getDaily: (date: string) => call<string>("get_daily", { date }),

	saveDaily: (date: string, content: string) =>
		call<void>("save_daily", { date, content }),

	readNote: (path: string) => call<string>("read_note", { path }),

	saveNote: (path: string, content: string) =>
		call<void>("save_note", { path, content }),

	// ─── Knowledge ─────────────────────────────────────────────────────────────
	searchKnowledge: (query: string) =>
		call<KnowledgeEntry[]>("search_knowledge", { query }),

	// ─── Vault ─────────────────────────────────────────────────────────────────
	listVaultDir: (path?: string) =>
		call<VaultEntry[]>("list_vault_dir", { path }),

	// ─── Settings ──────────────────────────────────────────────────────────────
	getSettings: () => call<AppSettings>("get_settings"),

	setVault: (path: string) => call<void>("set_vault", { path }),

	setApiKey: (key: string) => call<void>("set_api_key", { key }),
};
