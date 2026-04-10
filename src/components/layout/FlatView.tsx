import { FileText, Pin } from "lucide-react";
import type { VaultEntry } from "@/lib/types";
import { cn } from "@/lib/utils";
import { useVaultDirQuery } from "@/queries/vault";
import { useWorkspaceStore } from "@/stores/workspace";

interface FlatViewProps {
	path: string;
	tabId: string;
}

export function FlatView({ path, tabId }: FlatViewProps) {
	const { data: entries = [], isLoading } = useVaultDirQuery(path || undefined);

	const openItem = useWorkspaceStore((s) => s.openItem);
	const openedItem = useWorkspaceStore((s) => s.openedItem);
	const pinnedNote = useWorkspaceStore((s) => s.pinnedNote);
	const setPinnedNote = useWorkspaceStore((s) => s.setPinnedNote);

	if (isLoading) {
		return <div className="p-3 text-xs text-muted-foreground">加载中…</div>;
	}

	const files = entries
		.filter((e) => !e.is_dir)
		.sort((a, b) => b.modified_ms - a.modified_ms);

	if (files.length === 0) {
		return (
			<div className="p-3 text-xs text-muted-foreground">
				{tabId === "daily" ? "暂无日记" : "暂无文件"}
			</div>
		);
	}

	function handleClick(entry: VaultEntry) {
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

	function handlePin(e: React.MouseEvent, entry: VaultEntry) {
		e.stopPropagation();
		const title = entry.name.endsWith(".md")
			? entry.name.slice(0, -3)
			: entry.name;
		if (pinnedNote?.path === entry.path) {
			setPinnedNote(null);
		} else {
			setPinnedNote({ path: entry.path, title });
		}
	}

	return (
		<div className="py-1">
			{files.map((entry) => {
				const isActive =
					openedItem !== null &&
					"path" in openedItem &&
					openedItem.path === entry.path;
				const isPinned = pinnedNote?.path === entry.path;
				const displayName = entry.name.endsWith(".md")
					? entry.name.slice(0, -3)
					: entry.name;
				const date = new Date(entry.modified_ms);
				const dateStr = `${date.getMonth() + 1}/${date.getDate()}`;

				return (
					<div
						key={entry.path}
						className={cn(
							"group flex w-full items-center gap-2 px-3 py-1.5 text-xs transition-colors hover:bg-accent/50",
							isActive && "bg-accent text-accent-foreground",
						)}
					>
						<button
							type="button"
							onClick={() => handleClick(entry)}
							title={entry.name}
							className="flex flex-1 min-w-0 items-center gap-2"
						>
							<FileText className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
							<span className="flex-1 truncate text-left">{displayName}</span>
							<span className="shrink-0 text-muted-foreground">{dateStr}</span>
						</button>
						{entry.name.endsWith(".md") && (
							<button
								type="button"
								onClick={(e) => handlePin(e, entry)}
								title={isPinned ? "取消固定" : "固定笔记"}
								className={cn(
									"shrink-0 rounded p-0.5 transition-colors",
									isPinned
										? "text-primary"
										: "text-transparent group-hover:text-muted-foreground hover:text-foreground",
								)}
							>
								<Pin className="h-3 w-3" />
							</button>
						)}
					</div>
				);
			})}
		</div>
	);
}
