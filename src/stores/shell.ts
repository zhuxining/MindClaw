import { create } from "zustand";
import type {
	EditorSaveState,
	OpenedItem,
	StatusBarState,
	WorkspaceId,
	WorkspacePanelSizes,
} from "@/lib/types";

const WORKSPACE_IDS: readonly WorkspaceId[] = [
	"daily",
	"inbox",
	"private",
	"vault",
	"agent",
	"skills",
	"memory",
	"mcp",
	"session",
	"cron",
	"checklist",
	"graph",
	"tasks",
	"settings",
];

function isValidWorkspaceId(id: string): id is WorkspaceId {
	return WORKSPACE_IDS.includes(id as WorkspaceId);
}

interface ShellState {
	activeWorkspaceId: WorkspaceId;
	leftPanelCollapsed: boolean;
	rightPanelCollapsed: boolean;
	panelSizes: WorkspacePanelSizes;
	openedItem: OpenedItem | null;
	statusBar: StatusBarState;
	isHydrated: boolean;
}

interface ShellActions {
	setActiveWorkspace: (id: WorkspaceId) => void;
	toggleLeftPanel: () => void;
	toggleRightPanel: () => void;
	setPanelSizes: (sizes: WorkspacePanelSizes) => void;
	openItem: (item: OpenedItem) => void;
	closeItem: () => void;
	setSaveState: (state: EditorSaveState) => void;
	setCursorPosition: (line: number, col: number) => void;
	hydrate: (prefs: {
		activeWorkspaceId?: string;
		panelSizes?: WorkspacePanelSizes;
		lastOpenedItem?: OpenedItem | null;
	}) => void;
}

const DEFAULT_PANEL_SIZES: WorkspacePanelSizes = {
	left: 22,
	center: 52,
	right: 26,
};

export const useShellStore = create<ShellState & ShellActions>()((set) => ({
	activeWorkspaceId: "daily",
	leftPanelCollapsed: false,
	rightPanelCollapsed: false,
	panelSizes: DEFAULT_PANEL_SIZES,
	openedItem: null,
	statusBar: { saveState: "idle", lineCol: "行 1, 列 1", encoding: "UTF-8" },
	isHydrated: false,

	setActiveWorkspace: (id) => set({ activeWorkspaceId: id }),

	toggleLeftPanel: () =>
		set((s) => ({ leftPanelCollapsed: !s.leftPanelCollapsed })),
	toggleRightPanel: () =>
		set((s) => ({ rightPanelCollapsed: !s.rightPanelCollapsed })),

	setPanelSizes: (sizes) => set({ panelSizes: sizes }),

	openItem: (item) => set({ openedItem: item }),
	closeItem: () => set({ openedItem: null }),

	setSaveState: (saveState) =>
		set((s) => ({ statusBar: { ...s.statusBar, saveState } })),
	setCursorPosition: (line, col) =>
		set((s) => ({
			statusBar: {
				...s.statusBar,
				lineCol: `行 ${line}, 列 ${col}`,
			},
		})),

	hydrate: (prefs) =>
		set((s) => {
			if (s.isHydrated) return s;
			const workspaceId = prefs.activeWorkspaceId;
			return {
				activeWorkspaceId:
					workspaceId && isValidWorkspaceId(workspaceId)
						? workspaceId
						: "daily",
				panelSizes: prefs.panelSizes ?? DEFAULT_PANEL_SIZES,
				openedItem: prefs.lastOpenedItem ?? null,
				isHydrated: true,
			};
		}),
}));

export function workspaceIdToScope(id: WorkspaceId): string {
	switch (id) {
		case "daily":
			return "daily";
		case "inbox":
			return "inbox";
		case "private":
			return "private";
		case "vault":
			return "";
		default:
			return "";
	}
}

export function workspaceIdToViewMode(id: WorkspaceId): "tree" | "flat" {
	return id === "vault" ? "tree" : "flat";
}
