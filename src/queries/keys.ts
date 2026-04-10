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
	},
	settings: {
		all: ["settings"] as const,
	},
	vault: {
		dir: (path?: string) => ["vault", "dir", path ?? ""] as const,
	},
} as const;
