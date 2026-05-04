import { useShellStore } from "@/stores/shell";

export function NoteFrontmatterPane() {
	const openedItem = useShellStore((s) => s.openedItem);

	if (
		!openedItem ||
		openedItem.type === "source-web" ||
		openedItem.type === "source-pdf" ||
		openedItem.type === "source-image"
	) {
		return (
			<div className="flex h-full items-center justify-center p-4 text-sm text-muted-foreground">
				打开 Markdown 文件后可查看元数据
			</div>
		);
	}

	const path = "path" in openedItem ? openedItem.path : "";
	const title = "title" in openedItem ? openedItem.title : path;

	return (
		<div className="p-3 text-sm">
			<div className="mb-2">
				<span className="text-muted-foreground text-xs">元数据</span>
			</div>
			<div className="space-y-1.5">
				<div>
					<span className="text-xs" style={{ color: "var(--flexoki-tx-3)" }}>
						标题
					</span>
					<p className="truncate">{title}</p>
				</div>
				<div>
					<span className="text-xs" style={{ color: "var(--flexoki-tx-3)" }}>
						路径
					</span>
					<p className="truncate">{path}</p>
				</div>
			</div>
		</div>
	);
}
