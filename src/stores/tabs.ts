import { create } from "zustand";
import type { ContentDescriptor, OpenTab } from "@/lib/types";

interface TabState {
	openTabs: OpenTab[];
	activeTabId: string | null;
}

interface TabActions {
	openTab: (descriptor: ContentDescriptor) => void;
	closeTab: (tabId: string) => void;
	setActiveTab: (tabId: string) => void;
	markDirty: (tabId: string) => void;
	markClean: (tabId: string) => void;
}

function tabId(descriptor: ContentDescriptor): string {
	// 使用 :: 作为分隔符，避免与 Windows 脚本路径 (C:) 或 URL (http:) 冲突
	return `${descriptor.type}::${descriptor.path}`;
}

export const useTabStore = create<TabState & TabActions>()((set) => ({
	openTabs: [],
	activeTabId: null,

	openTab: (descriptor) =>
		set((s) => {
			const id = tabId(descriptor);
			const existing = s.openTabs.find((t) => t.id === id);
			if (existing) return { activeTabId: id };
			return {
				openTabs: [...s.openTabs, { id, descriptor, dirty: false }],
				activeTabId: id,
			};
		}),

	closeTab: (id) =>
		set((s) => {
			const idx = s.openTabs.findIndex((t) => t.id === id);
			const next = s.openTabs.filter((t) => t.id !== id);
			let nextActive = s.activeTabId;
			if (s.activeTabId === id) {
				if (next.length === 0) {
					nextActive = null;
				} else {
					const fallback = next[Math.min(idx, next.length - 1)];
					nextActive = fallback.id;
				}
			}
			return { openTabs: next, activeTabId: nextActive };
		}),

	setActiveTab: (id) => set({ activeTabId: id }),

	markDirty: (id) =>
		set((s) => ({
			openTabs: s.openTabs.map((t) =>
				t.id === id ? { ...t, dirty: true } : t,
			),
		})),

	markClean: (id) =>
		set((s) => ({
			openTabs: s.openTabs.map((t) =>
				t.id === id ? { ...t, dirty: false } : t,
			),
		})),
}));
