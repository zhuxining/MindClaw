import { ChevronDown, ChevronRight, FileText, Folder, Pin } from "lucide-react";
import { useState } from "react";
import type { VaultEntry } from "@/lib/types";
import { cn } from "@/lib/utils";
import { useVaultDirQuery } from "@/queries/vault";
import { useWorkspaceStore } from "@/stores/workspace";

interface TreeViewProps {
	path: string;
}

export function TreeView({ path }: TreeViewProps) {
	const { data: entries = [], isLoading } = useVaultDirQuery(path || undefined);

	if (isLoading) {
		return <div className="p-3 text-xs text-muted-foreground">加载中…</div>;
	}

	return (
		<div className="py-1">
			{entries.map((entry) => (
				<TreeNode key={entry.path} entry={entry} depth={0} />
			))}
		</div>
	);
}

function TreeNode({ entry, depth }: { entry: VaultEntry; depth: number }) {
	const [expanded, setExpanded] = useState(depth === 0 && entry.is_dir);
	const { data: children = [] } = useVaultDirQuery(
		expanded ? entry.path : undefined,
	);
	const openItem = useWorkspaceStore((s) => s.openItem);
	const openedItem = useWorkspaceStore((s) => s.openedItem);
	const pinnedNote = useWorkspaceStore((s) => s.pinnedNote);
	const setPinnedNote = useWorkspaceStore((s) => s.setPinnedNote);

	const isActive =
		openedItem !== null &&
		"path" in openedItem &&
		openedItem.path === entry.path;
	const isPinned = pinnedNote?.path === entry.path;

	function handleClick() {
		if (entry.is_dir) {
			setExpanded((v) => !v);
		} else {
			const isDaily =
				entry.path.startsWith("daily/") && entry.name.endsWith(".md");
			if (isDaily) {
				const date = entry.name.replace(".md", "");
				openItem({ type: "daily", date, path: entry.path });
			} else if (entry.name.endsWith(".md")) {
				openItem({
					type: "note",
					path: entry.path,
					title: entry.name.replace(".md", ""),
				});
			} else if (entry.name.endsWith(".pdf")) {
				openItem({ type: "source", path: entry.path, sourceType: "pdf" });
			} else {
				openItem({ type: "note", path: entry.path, title: entry.name });
			}
		}
	}

	function handlePin(e: React.MouseEvent) {
		e.stopPropagation();
		const title = entry.name.endsWith(".md")
			? entry.name.slice(0, -3)
			: entry.name;
		if (isPinned) {
			setPinnedNote(null);
		} else {
			setPinnedNote({ path: entry.path, title });
		}
	}

	return (
		<>
			<div
				className={cn(
					"group flex items-center rounded text-xs transition-colors hover:bg-accent/50",
					isActive && "bg-accent text-accent-foreground",
				)}
			>
				<button
					type="button"
					onClick={handleClick}
					title={entry.name}
					className="flex flex-1 min-w-0 items-center gap-1.5 py-1 pr-1 truncate"
					style={{ paddingLeft: `${8 + depth * 16}px` } as React.CSSProperties}
				>
					{entry.is_dir ? (
						<>
							{expanded ? (
								<ChevronDown className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
							) : (
								<ChevronRight className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
							)}
							<Folder className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
						</>
					) : (
						<FileText className="ml-5 h-3.5 w-3.5 shrink-0 text-muted-foreground" />
					)}
					<span className="truncate">{entry.name}</span>
				</button>

				{!entry.is_dir && entry.name.endsWith(".md") && (
					<button
						type="button"
						onClick={handlePin}
						title={isPinned ? "取消固定" : "固定笔记"}
						className={cn(
							"shrink-0 rounded p-0.5 mr-1 transition-colors",
							isPinned
								? "text-primary"
								: "text-transparent group-hover:text-muted-foreground hover:text-foreground",
						)}
					>
						<Pin className="h-3 w-3" />
					</button>
				)}
			</div>

			{entry.is_dir && expanded && (
				<div>
					{children.map((child) => (
						<TreeNode key={child.path} entry={child} depth={depth + 1} />
					))}
				</div>
			)}
		</>
	);
}
