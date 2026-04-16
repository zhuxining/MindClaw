import { FileText, Pin } from "lucide-react";
import { cn } from "@/lib/utils";
import { useVaultFlatQuery } from "@/queries/vault";
import { useWorkspaceStore } from "@/stores/workspace";
import { buildOpenedItemFromEntry, isPinnableEntry } from "./opened-item";

interface FlatViewProps {
	path: string;
	tabId: string;
	query?: string;
}

export function FlatView({ path, tabId, query = "" }: FlatViewProps) {
	const { data: entries = [], isLoading } = useVaultFlatQuery(
		path || undefined,
	);
	const openItem = useWorkspaceStore((state) => state.openItem);
	const openedItem = useWorkspaceStore((state) => state.openedItem);
	const pinnedNote = useWorkspaceStore((state) => state.pinnedNote);
	const setPinnedNote = useWorkspaceStore((state) => state.setPinnedNote);

	if (isLoading) {
		return <div className="p-4 text-sm text-muted-foreground">整理列表中…</div>;
	}

	const normalizedQuery = query.trim().toLowerCase();
	const files = entries.filter((entry) =>
		normalizedQuery.length === 0
			? true
			: entry.name.toLowerCase().includes(normalizedQuery) ||
				entry.path.toLowerCase().includes(normalizedQuery),
	);

	if (files.length === 0) {
		return (
			<div className="p-4 text-sm text-muted-foreground">
				{tabId === "daily" ? "暂无日记" : "暂无文件"}
			</div>
		);
	}

	async function handleOpen(pathIndex: number) {
		try {
			const item = await buildOpenedItemFromEntry(files[pathIndex]);
			openItem(item);
		} catch (error) {
			console.error("[FlatView] open failed", error);
		}
	}

	return (
		<div className="h-full overflow-y-auto px-2 py-3">
			<div className="space-y-1.5">
				{files.map((entry, index) => {
					const isActive =
						openedItem !== null &&
						"path" in openedItem &&
						openedItem.path === entry.path;
					const isPinned = pinnedNote?.path === entry.path;
					const displayName = entry.name.replace(/\.[^.]+$/, "");
					const date = new Date(entry.modified_ms);
					const dateStr = `${date.getMonth() + 1}/${date.getDate()}`;

					return (
						<div
							key={entry.path}
							className={cn(
								"group flex items-center gap-2 rounded-xl px-2 py-1.5 transition-colors",
								isActive && "bg-accent/70 text-accent-foreground",
							)}
						>
							<button
								type="button"
								onClick={() => void handleOpen(index)}
								title={entry.path}
								className="flex min-w-0 flex-1 items-center gap-3 rounded-lg px-2 py-2 text-left hover:bg-muted/70"
							>
								<div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-border/70 bg-muted/60">
									<FileText className="h-4 w-4 text-muted-foreground" />
								</div>
								<div className="min-w-0 flex-1">
									<p className="truncate text-sm font-medium">{displayName}</p>
									<p className="truncate text-xs text-muted-foreground">
										{entry.path}
									</p>
								</div>
								<div className="shrink-0 text-xs text-muted-foreground">
									{dateStr}
								</div>
							</button>

							{isPinnableEntry(entry) ? (
								<button
									type="button"
									onClick={(event) => {
										event.stopPropagation();
										if (isPinned) {
											setPinnedNote(null);
										} else {
											setPinnedNote({
												path: entry.path,
												title: displayName,
											});
										}
									}}
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
				})}
			</div>
		</div>
	);
}
