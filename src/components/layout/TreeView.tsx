import { ChevronDown, ChevronRight, FileText, Folder, Pin } from "lucide-react";
import { useState } from "react";
import {
	ContextMenu,
	ContextMenuContent,
	ContextMenuItem,
	ContextMenuTrigger,
} from "@/components/ui/context-menu";
import type { VaultEntry } from "@/lib/types";
import { cn } from "@/lib/utils";
import { useVaultDirQuery } from "@/queries/vault";
import { useWorkspaceStore } from "@/stores/workspace";
import { buildOpenedItemFromEntry, isPinnableEntry } from "./opened-item";

interface TreeViewProps {
	path: string;
	query?: string;
}

export function TreeView({ path, query = "" }: TreeViewProps) {
	const { data: entries = [], isLoading } = useVaultDirQuery(path || undefined);

	if (isLoading) {
		return <div className="p-4 text-sm text-muted-foreground">加载目录…</div>;
	}

	return (
		<div className="h-full overflow-y-auto px-2 py-3">
			{entries.map((entry) => (
				<TreeNode key={entry.path} entry={entry} depth={0} query={query} />
			))}
		</div>
	);
}

function TreeNode({
	entry,
	depth,
	query,
}: {
	entry: VaultEntry;
	depth: number;
	query: string;
}) {
	const shouldInspectChildren =
		entry.is_dir && (depth === 0 || query.trim().length > 0);
	const [expanded, setExpanded] = useState(depth === 0 && entry.is_dir);
	const { data: children = [] } = useVaultDirQuery(
		shouldInspectChildren || expanded ? entry.path : undefined,
	);
	const openItem = useWorkspaceStore((state) => state.openItem);
	const openedItem = useWorkspaceStore((state) => state.openedItem);
	const pinnedNote = useWorkspaceStore((state) => state.pinnedNote);
	const setPinnedNote = useWorkspaceStore((state) => state.setPinnedNote);
	const pinDirTab = useWorkspaceStore((state) => state.pinDirTab);
	const normalizedQuery = query.trim().toLowerCase();

	const childMatches =
		normalizedQuery.length > 0 &&
		children.some((child) =>
			child.name.toLowerCase().includes(normalizedQuery),
		);
	const selfMatches =
		normalizedQuery.length === 0 ||
		entry.name.toLowerCase().includes(normalizedQuery) ||
		childMatches;

	if (!selfMatches) return null;

	const isActive =
		openedItem !== null &&
		"path" in openedItem &&
		openedItem.path === entry.path;
	const isPinned = pinnedNote?.path === entry.path;

	async function handleClick() {
		if (entry.is_dir) {
			setExpanded((value) => !value);
			return;
		}

		try {
			const item = await buildOpenedItemFromEntry(entry);
			openItem(item);
		} catch (error) {
			console.error("[TreeView] open failed", error);
		}
	}

	function handlePin(event: React.MouseEvent) {
		event.stopPropagation();
		const title = entry.name.endsWith(".md")
			? entry.name.slice(0, -3)
			: entry.name;
		if (isPinned) {
			setPinnedNote(null);
		} else {
			setPinnedNote({ path: entry.path, title });
		}
	}

	const row = (
		<div
			className={cn(
				"group flex items-center rounded-xl px-1.5 py-0.5",
				isActive && "bg-accent/70 text-accent-foreground",
			)}
		>
			<button
				type="button"
				onClick={handleClick}
				title={entry.name}
				className="flex min-w-0 flex-1 items-center gap-2 rounded-lg px-2 py-2 text-left text-sm transition-colors hover:bg-muted/70"
				style={{ paddingLeft: `${10 + depth * 16}px` } as React.CSSProperties}
			>
				{entry.is_dir ? (
					<>
						{expanded || normalizedQuery.length > 0 ? (
							<ChevronDown className="h-4 w-4 shrink-0 text-muted-foreground" />
						) : (
							<ChevronRight className="h-4 w-4 shrink-0 text-muted-foreground" />
						)}
						<Folder className="h-4 w-4 shrink-0 text-muted-foreground" />
					</>
				) : (
					<>
						<span className="w-4 shrink-0" />
						<FileText className="h-4 w-4 shrink-0 text-muted-foreground" />
					</>
				)}
				<span className="truncate">{entry.name}</span>
			</button>

			{isPinnableEntry(entry) ? (
				<button
					type="button"
					onClick={handlePin}
					title={isPinned ? "取消固定" : "固定笔记"}
					className={cn(
						"mr-1 shrink-0 rounded-lg p-1.5 transition-colors",
						isPinned
							? "text-primary"
							: "text-transparent group-hover:text-muted-foreground hover:text-foreground",
					)}
				>
					<Pin className="h-3.5 w-3.5" />
				</button>
			) : null}
		</div>
	);

	return (
		<>
			{entry.is_dir ? (
				<ContextMenu>
					<ContextMenuTrigger>{row}</ContextMenuTrigger>
					<ContextMenuContent>
						<ContextMenuItem
							onClick={() =>
								pinDirTab({
									id: `dir:${entry.path}`,
									dirPath: entry.path,
									label: entry.name,
								})
							}
						>
							固定为 Tab
						</ContextMenuItem>
					</ContextMenuContent>
				</ContextMenu>
			) : (
				row
			)}

			{entry.is_dir && (expanded || normalizedQuery.length > 0)
				? children.map((child) => (
						<TreeNode
							key={child.path}
							entry={child}
							depth={depth + 1}
							query={query}
						/>
					))
				: null}
		</>
	);
}
