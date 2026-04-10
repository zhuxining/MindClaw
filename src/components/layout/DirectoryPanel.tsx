import { LayoutList, TreePine } from "lucide-react";
import type { DirectoryViewMode } from "@/lib/types";
import { cn } from "@/lib/utils";
import {
	tabToVaultPath,
	useTabViewMode,
	useWorkspaceStore,
} from "@/stores/workspace";
import { FlatView } from "./FlatView";
import { TreeView } from "./TreeView";

export function DirectoryPanel() {
	const activeTabId = useWorkspaceStore((s) => s.activeTabId);
	const pinnedDirTabs = useWorkspaceStore((s) => s.pinnedDirTabs);
	const setDirViewMode = useWorkspaceStore((s) => s.setDirViewMode);
	const viewMode = useTabViewMode(activeTabId);

	const vaultPath = tabToVaultPath(activeTabId, pinnedDirTabs);

	return (
		<div className="flex h-full flex-col">
			{/* 视图切换工具栏 */}
			<div className="flex items-center justify-end px-2 py-1 border-b border-border">
				<ViewToggle
					mode={viewMode}
					onChange={(m) => setDirViewMode(activeTabId, m)}
				/>
			</div>

			{/* 文件列表 */}
			<div className="flex-1 overflow-y-auto">
				{viewMode === "tree" ? (
					<TreeView path={vaultPath} />
				) : (
					<FlatView path={vaultPath} tabId={activeTabId} />
				)}
			</div>
		</div>
	);
}

function ViewToggle({
	mode,
	onChange,
}: {
	mode: DirectoryViewMode;
	onChange: (m: DirectoryViewMode) => void;
}) {
	return (
		<div className="flex items-center gap-0.5 rounded border border-border p-0.5">
			<button
				type="button"
				title="平铺"
				onClick={() => onChange("flat")}
				className={cn(
					"rounded p-1 transition-colors",
					mode === "flat"
						? "bg-accent text-accent-foreground"
						: "text-muted-foreground hover:text-foreground",
				)}
			>
				<LayoutList className="h-3.5 w-3.5" />
			</button>
			<button
				type="button"
				title="树状"
				onClick={() => onChange("tree")}
				className={cn(
					"rounded p-1 transition-colors",
					mode === "tree"
						? "bg-accent text-accent-foreground"
						: "text-muted-foreground hover:text-foreground",
				)}
			>
				<TreePine className="h-3.5 w-3.5" />
			</button>
		</div>
	);
}
