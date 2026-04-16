export const queryKeys = {
	tasks: {
		all: ["tasks"] as const,
		list: (status?: string) => ["tasks", "list", status ?? "all"] as const,
	},
	daily: {
		byDate: (date: string) => ["daily", date] as const,
	},
	knowledge: {
		search: (query: string) => ["knowledge", "search", query] as const,
		relevant: (path: string) => ["knowledge", "relevant", path] as const,
	},
	settings: {
		all: ["settings"] as const,
		workspace: ["settings", "workspace"] as const,
	},
	vault: {
		dir: (path?: string) => ["vault", "dir", path ?? ""] as const,
		flat: (path?: string) => ["vault", "flat", path ?? ""] as const,
	},
} as const;
