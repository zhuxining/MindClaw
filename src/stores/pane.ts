import { create } from "zustand";
import type {
	LeftPaneId,
	PaneState,
	RightPaneId,
	WorkspaceId,
} from "@/lib/types";

interface PaneStoreState {
	leftPaneByWorkspace: Record<string, PaneState>;
	rightPaneByContent: Record<string, PaneState>;
	activeLeftFilter: string;
	activeRightFilter: string;
}

interface PaneStoreActions {
	setLeftPane: (workspaceId: WorkspaceId, paneId: LeftPaneId) => void;
	setRightPane: (contentType: string, paneId: RightPaneId) => void;
	setLeftFilter: (filter: string) => void;
	setRightFilter: (filter: string) => void;
	updateLeftPaneScroll: (
		workspaceId: WorkspaceId,
		scrollPosition: number,
	) => void;
}

const DEFAULT_LEFT_PANE: LeftPaneId = "file-explorer";
const DEFAULT_RIGHT_PANE: RightPaneId = "note-outline";

function defaultLeftState(paneId: LeftPaneId = DEFAULT_LEFT_PANE): PaneState {
	return { activePaneId: paneId, scrollPosition: 0, filterParams: {} };
}

function defaultRightState(
	paneId: RightPaneId = DEFAULT_RIGHT_PANE,
): PaneState {
	return { activePaneId: paneId, scrollPosition: 0, filterParams: {} };
}

export const usePaneStore = create<PaneStoreState & PaneStoreActions>()(
	(set) => ({
		leftPaneByWorkspace: {},
		rightPaneByContent: {},
		activeLeftFilter: DEFAULT_LEFT_PANE,
		activeRightFilter: DEFAULT_RIGHT_PANE,

		setLeftPane: (workspaceId, paneId) =>
			set((s) => ({
				leftPaneByWorkspace: {
					...s.leftPaneByWorkspace,
					[workspaceId]: s.leftPaneByWorkspace[workspaceId]
						? { ...s.leftPaneByWorkspace[workspaceId], activePaneId: paneId }
						: defaultLeftState(paneId),
				},
			})),

		setRightPane: (contentType, paneId) =>
			set((s) => ({
				rightPaneByContent: {
					...s.rightPaneByContent,
					[contentType]: s.rightPaneByContent[contentType]
						? { ...s.rightPaneByContent[contentType], activePaneId: paneId }
						: defaultRightState(paneId),
				},
			})),

		setLeftFilter: (filter) => set({ activeLeftFilter: filter }),
		setRightFilter: (filter) => set({ activeRightFilter: filter }),

		updateLeftPaneScroll: (workspaceId, scrollPosition) =>
			set((s) => ({
				leftPaneByWorkspace: {
					...s.leftPaneByWorkspace,
					[workspaceId]: {
						...(s.leftPaneByWorkspace[workspaceId] ?? defaultLeftState()),
						scrollPosition,
					},
				},
			})),
	}),
);
