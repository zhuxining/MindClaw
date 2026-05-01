import { create } from "zustand";
import { todayLocalDate } from "@/lib/date";
import type {
	BuiltinTabId,
	DirectoryViewMode,
	OpenedItem,
	PinnedDirTab,
	WorkspacePanelSizes,
	WorkspacePrefs,
	WorkspaceRightPanelHeights,
} from "@/lib/types";

export type TabId = BuiltinTabId | string;

function todayDailyItem(): OpenedItem {
	const date = todayLocalDate();
	return {
		type: "daily",
		date,
		path: `daily/${date}.md`,
	};
}

// NOTE: These defaults are duplicated in Rust (src-tauri/src/models/settings.rs).
// When changing values, update BOTH locations to maintain consistency.
const DEFAULT_PANEL_SIZES: WorkspacePanelSizes = {
	left: 22,
	center: 52,
	right: 26,
};

// 面板大小约束（与 AppShell.tsx 中的 min/maxSize 对应）
const PANEL_CONSTRAINTS = {
	left: { min: 16, max: 34 },
	center: { min: 34, max: 100 }, // center 没有 maxSize 限制
	right: { min: 18, max: 36 },
};

/**
 * 验证并修正面板大小，确保在有效范围内且总和合理
 */
function validatePanelSizes(
	sizes: WorkspacePanelSizes | undefined,
): WorkspacePanelSizes {
	if (!sizes) return DEFAULT_PANEL_SIZES;

	// 如果任何面板为 0 或过小，使用默认值
	const left =
		sizes.left < PANEL_CONSTRAINTS.left.min
			? DEFAULT_PANEL_SIZES.left
			: sizes.left;
	const center =
		sizes.center < PANEL_CONSTRAINTS.center.min
			? DEFAULT_PANEL_SIZES.center
			: sizes.center;
	const right =
		sizes.right < PANEL_CONSTRAINTS.right.min
			? DEFAULT_PANEL_SIZES.right
			: sizes.right;

	// 确保三个面板总和约为 100%（允许小误差）
	const total = left + center + right;
	if (total < 90 || total > 110) {
		return DEFAULT_PANEL_SIZES;
	}

	return { left, center, right };
}

const DEFAULT_RIGHT_PANEL_HEIGHTS: WorkspaceRightPanelHeights = {
	pin: 20,
	tasks: 50,
	relatedContent: 30,
};

interface WorkspaceState {
	activeTabId: TabId;
	pinnedDirTabs: PinnedDirTab[];
	dirViewMode: Record<string, DirectoryViewMode>;
	openedItem: OpenedItem | null;
	pinnedNote: { path: string; title: string } | null;
	chatOpen: boolean;
	panelSizes: WorkspacePanelSizes;
	rightPanelHeights: WorkspaceRightPanelHeights;
	isHydrated: boolean;
}

interface WorkspaceActions {
	hydrateFromPrefs: (prefs: WorkspacePrefs) => void;
	setActiveTab: (tabId: TabId) => void;
	pinDirTab: (tab: PinnedDirTab) => void;
	unpinDirTab: (id: string) => void;
	reorderPinnedTabs: (tabs: PinnedDirTab[]) => void;
	setDirViewMode: (tabId: string, mode: DirectoryViewMode) => void;
	setPanelSizes: (sizes: WorkspacePanelSizes) => void;
	setRightPanelHeights: (sizes: WorkspaceRightPanelHeights) => void;
	openItem: (item: OpenedItem) => void;
	closeItem: () => void;
	setPinnedNote: (note: { path: string; title: string } | null) => void;
	toggleChat: () => void;
	openChat: () => void;
	closeChat: () => void;
}

const MAX_PINNED_TABS = 6;

export const useWorkspaceStore = create<WorkspaceState & WorkspaceActions>()(
	(set) => ({
		activeTabId: "daily",
		pinnedDirTabs: [],
		dirViewMode: {},
		openedItem: todayDailyItem(),
		pinnedNote: null,
		chatOpen: false,
		panelSizes: DEFAULT_PANEL_SIZES,
		rightPanelHeights: DEFAULT_RIGHT_PANEL_HEIGHTS,
		isHydrated: false,

		hydrateFromPrefs: (prefs) =>
			set((state) => {
				if (state.isHydrated) return state;
				return {
					activeTabId: prefs.active_tab_id || "daily",
					pinnedDirTabs: prefs.pinned_dir_tabs ?? [],
					dirViewMode: prefs.dir_view_mode ?? {},
					openedItem: prefs.last_opened_item ?? state.openedItem,
					pinnedNote: prefs.pinned_note ?? null,
					panelSizes: validatePanelSizes(prefs.panel_sizes),
					rightPanelHeights:
						prefs.right_panel_heights ?? DEFAULT_RIGHT_PANEL_HEIGHTS,
					isHydrated: true,
				};
			}),

		setActiveTab: (tabId) => set({ activeTabId: tabId }),

		pinDirTab: (tab) =>
			set((state) => {
				if (state.pinnedDirTabs.length >= MAX_PINNED_TABS) return state;
				if (state.pinnedDirTabs.some((item) => item.id === tab.id))
					return state;
				return { pinnedDirTabs: [...state.pinnedDirTabs, tab] };
			}),

		unpinDirTab: (id) =>
			set((state) => ({
				pinnedDirTabs: state.pinnedDirTabs.filter((tab) => tab.id !== id),
				activeTabId: state.activeTabId === id ? "daily" : state.activeTabId,
			})),

		reorderPinnedTabs: (tabs) => set({ pinnedDirTabs: tabs }),

		setDirViewMode: (tabId, mode) =>
			set((state) => ({
				dirViewMode: { ...state.dirViewMode, [tabId]: mode },
			})),

		setPanelSizes: (sizes) => set({ panelSizes: sizes }),
		setRightPanelHeights: (sizes) => set({ rightPanelHeights: sizes }),

		openItem: (item) => set({ openedItem: item }),
		closeItem: () => set({ openedItem: null }),

		setPinnedNote: (note) => set({ pinnedNote: note }),

		toggleChat: () => set((state) => ({ chatOpen: !state.chatOpen })),
		openChat: () => set({ chatOpen: true }),
		closeChat: () => set({ chatOpen: false }),
	}),
);

export function useTabViewMode(tabId: string): DirectoryViewMode {
	return useWorkspaceStore(
		(state) =>
			state.dirViewMode[tabId] ?? (tabId === "vault" ? "tree" : "flat"),
	);
}

export function isBuiltinTab(tabId: string): tabId is BuiltinTabId {
	return ["daily", "private", "vault", "source"].includes(tabId);
}

export function tabToVaultPath(
	tabId: TabId,
	pinnedDirTabs: PinnedDirTab[],
): string {
	if (tabId === "daily") return "daily";
	if (tabId === "private") return "private";
	if (tabId === "source") return "source";
	if (tabId === "vault") return "";
	const pinned = pinnedDirTabs.find((tab) => tab.id === tabId);
	return pinned?.dirPath ?? "";
}

export function workspacePrefsFromState(
	state: WorkspaceState,
	chatMode: WorkspacePrefs["chat_mode"],
): WorkspacePrefs {
	return {
		active_tab_id: state.activeTabId,
		pinned_dir_tabs: state.pinnedDirTabs,
		dir_view_mode: state.dirViewMode,
		panel_sizes: state.panelSizes,
		right_panel_heights: state.rightPanelHeights,
		last_opened_item: state.openedItem,
		pinned_note: state.pinnedNote,
		chat_mode: chatMode,
	};
}
