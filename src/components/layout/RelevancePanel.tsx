import { useQuery } from "@tanstack/react-query";
import { ipc } from "@/lib/ipc";
import type { KnowledgeEntry, NoteItem } from "@/lib/types";
import { useWorkspaceStore } from "@/stores/workspace";

export function RelevancePanel() {
	const openedItem = useWorkspaceStore((s) => s.openedItem);
	const openItem = useWorkspaceStore((s) => s.openItem);

	// 用当前打开文件的文件名（去掉扩展名）作为搜索词
	const keyword =
		openedItem !== null && openedItem.type !== "source"
			? (openedItem.path.split("/").pop()?.replace(/\.md$/i, "") ?? "")
			: "";

	const { data: results = [] } = useQuery({
		queryKey: ["relevance", keyword],
		queryFn: () => ipc.searchKnowledge(keyword),
		enabled: keyword.length > 0,
	});

	// 排除当前打开的文件本身
	const filtered = results.filter(
		(r) =>
			openedItem === null ||
			openedItem.type === "source" ||
			r.topic !== openedItem.path,
	);

	function handleOpen(entry: KnowledgeEntry) {
		const note: NoteItem = {
			type: "note",
			path: entry.topic,
			title: entry.title,
		};
		openItem(note);
	}

	return (
		<div className="flex h-full flex-col">
			<div className="flex items-center border-b border-border px-3 py-2">
				<span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
					关联
				</span>
			</div>

			<div className="flex-1 overflow-y-auto px-3 py-2">
				{openedItem === null ? (
					<p className="text-xs text-muted-foreground">
						打开笔记后查看关联内容
					</p>
				) : filtered.length === 0 ? (
					<p className="text-xs text-muted-foreground">暂无相关笔记</p>
				) : (
					<ul className="space-y-1">
						{filtered.map((entry) => (
							<li key={entry.id}>
								<button
									type="button"
									onClick={() => handleOpen(entry)}
									className="w-full text-left rounded px-1.5 py-1 text-xs hover:bg-accent transition-colors"
								>
									<span className="font-medium truncate block">
										{entry.title}
									</span>
									{entry.tags.length > 0 && (
										<span className="text-muted-foreground">
											{entry.tags.slice(0, 3).join(" · ")}
										</span>
									)}
								</button>
							</li>
						))}
					</ul>
				)}
			</div>
		</div>
	);
}
