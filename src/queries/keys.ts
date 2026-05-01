export const queryKeys = {
	tasks: {
		all: ["tasks"] as const,
		list: (status?: string) => ["tasks", "list", status ?? "all"] as const,
	},
	daily: {
		byDate: (date: string) => ["daily", date] as const,
	},
	settings: {
		all: ["settings"] as const,
		workspace: ["settings", "workspace"] as const,
	},
	vault: {
		dir: (path?: string) => ["vault", "dir", path ?? ""] as const,
		flat: (path?: string) => ["vault", "flat", path ?? ""] as const,
		search: (query: string) => ["vault", "search", query] as const,
		relevant: (path: string) => ["vault", "relevant", path] as const,
	},
} as const;
