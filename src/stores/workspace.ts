import { create } from "zustand";
import { persist } from "zustand/middleware";
import type {
	BuiltinTabId,
	DirectoryViewMode,
	OpenedItem,
	PinnedDirTab,
} from "@/lib/types";

export type TabId = BuiltinTabId | string; // string = PinnedDirTab.id

interface WorkspaceState {
	activeTabId: TabId;
	pinnedDirTabs: PinnedDirTab[];
	/** 按 tabId 记录视图模式，持久化 */
	dirViewMode: Record<string, DirectoryViewMode>;
	openedItem: OpenedItem | null;
	pinnedNote: { path: string; title: string } | null;
	chatOpen: boolean;
}

interface WorkspaceActions {
	setActiveTab: (tabId: TabId) => void;
	pinDirTab: (tab: PinnedDirTab) => void;
	unpinDirTab: (id: string) => void;
	reorderPinnedTabs: (tabs: PinnedDirTab[]) => void;
	setDirViewMode: (tabId: string, mode: DirectoryViewMode) => void;
	openItem: (item: OpenedItem) => void;
	closeItem: () => void;
	setPinnedNote: (note: { path: string; title: string } | null) => void;
	toggleChat: () => void;
	openChat: () => void;
	closeChat: () => void;
}

const MAX_PINNED_TABS = 6;

export const useWorkspaceStore = create<WorkspaceState & WorkspaceActions>()(
	persist(
		(set) => ({
			activeTabId: "daily",
			pinnedDirTabs: [],
			dirViewMode: {},
			openedItem: null,
			pinnedNote: null,
			chatOpen: false,

			setActiveTab: (tabId) => set({ activeTabId: tabId }),

			pinDirTab: (tab) =>
				set((s) => {
					if (s.pinnedDirTabs.length >= MAX_PINNED_TABS) return s;
					if (s.pinnedDirTabs.some((t) => t.id === tab.id)) return s;
					return { pinnedDirTabs: [...s.pinnedDirTabs, tab] };
				}),

			unpinDirTab: (id) =>
				set((s) => ({
					pinnedDirTabs: s.pinnedDirTabs.filter((t) => t.id !== id),
					activeTabId: s.activeTabId === id ? "daily" : s.activeTabId,
				})),

			reorderPinnedTabs: (tabs) => set({ pinnedDirTabs: tabs }),

			setDirViewMode: (tabId, mode) =>
				set((s) => ({
					dirViewMode: { ...s.dirViewMode, [tabId]: mode },
				})),

			openItem: (item) => set({ openedItem: item }),
			closeItem: () => set({ openedItem: null }),

			setPinnedNote: (note) => set({ pinnedNote: note }),

			toggleChat: () => set((s) => ({ chatOpen: !s.chatOpen })),
			openChat: () => set({ chatOpen: true }),
			closeChat: () => set({ chatOpen: false }),
		}),
		{
			name: "mindclaw-workspace",
			// 只持久化需要跨会话保留的状态
			partialize: (s) => ({
				activeTabId: s.activeTabId,
				pinnedDirTabs: s.pinnedDirTabs,
				dirViewMode: s.dirViewMode,
				pinnedNote: s.pinnedNote,
			}),
		},
	),
);

// 当前 tab 的视图模式，默认 'flat'（Daily 适合时间倒序浏览）
export function useTabViewMode(tabId: string): DirectoryViewMode {
	const dirViewMode = useWorkspaceStore((s) => s.dirViewMode);
	return dirViewMode[tabId] ?? (tabId === "vault" ? "tree" : "flat");
}

// 判断 tabId 是否为内置 tab
export function isBuiltinTab(tabId: string): tabId is BuiltinTabId {
	return ["daily", "private", "vault", "source"].includes(tabId);
}

// 根据 tabId 获取 vault 子目录路径
export function tabToVaultPath(
	tabId: TabId,
	pinnedDirTabs: PinnedDirTab[],
): string {
	if (tabId === "daily") return "daily";
	if (tabId === "private") return "private";
	if (tabId === "source") return "source";
	if (tabId === "vault") return "";
	const pinned = pinnedDirTabs.find((t) => t.id === tabId);
	return pinned?.dirPath ?? "";
}
