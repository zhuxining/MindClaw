import { useQuery } from "@tanstack/react-query";
import { ArrowUpRight, Sparkles } from "lucide-react";
import { ipc } from "@/lib/ipc";
import {
	EmptyState,
	PanelFrame,
	SectionHeader,
} from "@/shell/shell-primitives";
import { useShellStore } from "@/stores/shell";

export function RelatedContentPanel() {
	const openedItem = useShellStore((state) => state.openedItem);
	const openItem = useShellStore((state) => state.openItem);
	const supportedPath =
		openedItem && (openedItem.type === "daily" || openedItem.type === "note")
			? openedItem.path
			: null;

	const { data: results = [], isLoading } = useQuery({
		queryKey: ["related-files", supportedPath],
		queryFn: () => ipc.getRelevantNotes(supportedPath ?? ""),
		enabled: Boolean(supportedPath),
	});

	return (
		<PanelFrame className="overflow-hidden">
			<SectionHeader
				title="Related Content"
				description="围绕当前内容的关联笔记"
			/>

			<div className="min-h-0 flex-1 overflow-y-auto p-3">
				{!supportedPath ? (
					<EmptyState
						title="打开一条笔记后再看关联"
						description="右侧会根据当前内容的标题、标签和上下文给出更贴近的参考笔记。"
					/>
				) : isLoading ? (
					<div className="px-1 py-2 text-sm text-muted-foreground">
						正在整理关联笔记…
					</div>
				) : results.length === 0 ? (
					<EmptyState
						title="暂无关联内容"
						description="这条内容还没有足够明确的关联线索，继续积累后会在这里出现。"
					/>
				) : (
					<ul className="space-y-2">
						{results.map((entry) => (
							<li key={entry.id}>
								<button
									type="button"
									onClick={() =>
										openItem({
											type: "note",
											path: entry.topic,
											title: entry.title,
										})
									}
									className="flex w-full items-start gap-3 rounded-2xl border border-transparent px-3 py-3 text-left transition-colors hover:border-border hover:bg-muted/60"
								>
									<div className="mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-border/70 bg-muted/60">
										<Sparkles className="h-4 w-4 text-muted-foreground" />
									</div>
									<div className="min-w-0 flex-1">
										<p className="truncate text-sm font-medium text-foreground">
											{entry.title}
										</p>
										<p className="truncate text-xs text-muted-foreground">
											{entry.topic}
										</p>
										{entry.tags.length > 0 ? (
											<p className="mt-1 truncate text-[11px] text-muted-foreground">
												{entry.tags.slice(0, 3).join(" · ")}
											</p>
										) : null}
									</div>
									<ArrowUpRight className="mt-1 h-4 w-4 shrink-0 text-muted-foreground" />
								</button>
							</li>
						))}
					</ul>
				)}
			</div>
		</PanelFrame>
	);
}
