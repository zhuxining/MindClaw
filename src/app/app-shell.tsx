import { FileText, Link2, ListTree } from "lucide-react";
import { useMemo } from "react";
import {
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup,
} from "@/components/ui/resizable";
import { CenterContent } from "@/shell/content-host/center-content";
import { ContentHost } from "@/shell/content-host/content-host";
import { useInspectorContext } from "@/shell/content-host/inspector-context";
import { LeftPanel } from "@/shell/panels/left-panel/left-panel";
import { PaneFilterToolbar } from "@/shell/panels/pane-filter-toolbar/pane-filter-toolbar";
import { PaneHost } from "@/shell/panels/pane-host/pane-host";
import { RightPanel } from "@/shell/panels/right-panel/right-panel";
import { NoteFrontmatterPane } from "@/shell/panes/note-frontmatter-pane/note-frontmatter-pane";
import { NoteOutlinePane } from "@/shell/panes/note-outline-pane/note-outline-pane";
import { RelatedFilesPane } from "@/shell/panes/related-files-pane/related-files-pane";
import { Ribbon } from "@/shell/ribbon/ribbon";
import { StatusBar } from "@/shell/status-bar/status-bar";
import { usePaneStore } from "@/stores/pane";
import { useShellStore } from "@/stores/shell";
import { getWorkspace } from "@/workspaces/workspace-registry";

export function AppShell() {
	const panelSizes = useShellStore((s) => s.panelSizes);
	const setPanelSizes = useShellStore((s) => s.setPanelSizes);
	const isHydrated = useShellStore((s) => s.isHydrated);
	const activeWorkspaceId = useShellStore((s) => s.activeWorkspaceId);

	const rightFilter = usePaneStore((s) => s.activeRightFilter);
	const setRightFilter = usePaneStore((s) => s.setRightFilter);

	const inspectorContext = useInspectorContext();

	const workspace = getWorkspace(activeWorkspaceId);
	const sizes = isHydrated ? panelSizes : { left: 22, center: 52, right: 26 };

	// Derive right panel panes from inspector context
	const rightPanes = useMemo(() => {
		const panes = [];
		if (inspectorContext.hasOutline) {
			panes.push({
				id: "note-outline",
				label: "大纲",
				render: () => <NoteOutlinePane />,
			});
		}
		if (inspectorContext.hasFrontmatter) {
			panes.push({
				id: "note-frontmatter",
				label: "元数据",
				render: () => <NoteFrontmatterPane />,
			});
		}
		if (inspectorContext.hasRelatedContent) {
			panes.push({
				id: "related-files",
				label: "关联",
				render: () => <RelatedFilesPane />,
			});
		}
		// If no panes available, show empty state message
		if (panes.length === 0) {
			panes.push({
				id: "empty",
				label: "无上下文",
				render: () => (
					<div className="flex h-full items-center justify-center p-4 text-sm text-muted-foreground">
						{inspectorContext.contentType
							? "当前内容不支持上下文面板"
							: "打开文件后可查看上下文信息"}
					</div>
				),
			});
		}
		return panes;
	}, [inspectorContext]);

	const rightFilters = useMemo(() => {
		const filters = [];
		if (inspectorContext.hasOutline) {
			filters.push({ id: "note-outline", icon: ListTree, label: "大纲" });
		}
		if (inspectorContext.hasFrontmatter) {
			filters.push({ id: "note-frontmatter", icon: FileText, label: "元数据" });
		}
		if (inspectorContext.hasRelatedContent) {
			filters.push({ id: "related-files", icon: Link2, label: "关联" });
		}
		return filters;
	}, [inspectorContext]);

	// Determine active pane - prefer the first available pane from context
	const effectiveRightFilter = useMemo(() => {
		if (rightFilters.length === 0) return "empty";
		// If current filter is valid for context, keep it
		if (rightFilters.some((f) => f.id === rightFilter)) return rightFilter;
		// Otherwise use first available
		return rightFilters[0].id;
	}, [rightFilter, rightFilters]);

	return (
		<div className="fixed inset-0 flex min-h-150 min-w-225 bg-background text-foreground">
			<Ribbon />

			<div className="flex min-h-0 flex-1 flex-col">
				<ResizablePanelGroup
					key={isHydrated ? "hydrated" : "loading"}
					orientation="horizontal"
					className="min-h-0 flex-1"
					onLayoutChanged={(layout: { [id: string]: number }) => {
						setPanelSizes({
							left: layout.left ?? sizes.left,
							center: layout.center ?? sizes.center,
							right: layout.right ?? sizes.right,
						});
					}}
				>
					<ResizablePanel
						id="left"
						defaultSize={`${sizes.left}`}
						minSize="16"
						maxSize="34"
						className="min-w-55"
					>
						<LeftPanel>
							{workspace && (
								<>
									<PaneHost
										panes={workspace.leftPanel.panes}
										activePaneId={workspace.leftPanel.defaultPane}
									/>
									<PaneFilterToolbar
										filters={workspace.leftPanel.filterToolbar}
										activeFilter={workspace.leftPanel.defaultPane}
										onFilterChange={() => {}}
									/>
								</>
							)}
						</LeftPanel>
					</ResizablePanel>

					<ResizableHandle withHandle className="bg-transparent" />

					<ResizablePanel
						id="center"
						defaultSize={`${sizes.center}`}
						minSize="34"
						className="min-w-105"
					>
						<ContentHost>
							<CenterContent />
						</ContentHost>
					</ResizablePanel>

					<ResizableHandle withHandle className="bg-transparent" />

					<ResizablePanel
						id="right"
						defaultSize={`${sizes.right}`}
						minSize="18"
						maxSize="36"
						className="min-w-65"
					>
						<RightPanel>
							<PaneHost
								panes={rightPanes}
								activePaneId={effectiveRightFilter}
							/>
							{rightFilters.length > 0 && (
								<PaneFilterToolbar
									filters={rightFilters}
									activeFilter={effectiveRightFilter}
									onFilterChange={setRightFilter}
								/>
							)}
						</RightPanel>
					</ResizablePanel>
				</ResizablePanelGroup>

				<StatusBar />
			</div>
		</div>
	);
}
