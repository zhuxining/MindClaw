import type { LucideIcon } from "lucide-react";
import type {
	ContentType,
	LeftPaneId,
	RightPaneId,
	WorkspaceId,
} from "@/lib/types";
import type { PaneFilterEntry } from "@/shell/panels/pane-filter-toolbar/pane-filter-toolbar";
import type { PaneDefinition } from "@/shell/panels/pane-host/pane-host";

export interface WorkspaceDefinition {
	id: WorkspaceId;
	ribbonItem: {
		id: string;
		icon: LucideIcon;
		label: string;
	};
	leftPanel: {
		panes: PaneDefinition[];
		filterToolbar: PaneFilterEntry[];
		defaultPane: LeftPaneId;
	};
	defaultContent: {
		type: ContentType;
		path: string;
		title: string;
	};
	rightPanel: {
		panes: PaneDefinition[];
		filterToolbar: PaneFilterEntry[];
		defaultPane: RightPaneId;
	};
	openBehavior: "new-tab" | "replace-current" | "standalone";
}
