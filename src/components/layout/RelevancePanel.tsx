import { useWorkspaceStore } from "@/stores/workspace";

export function RelevancePanel() {
	const openedItem = useWorkspaceStore((s) => s.openedItem);

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
				) : (
					<p className="text-xs text-muted-foreground">暂无相关笔记</p>
				)}
			</div>
		</div>
	);
}
