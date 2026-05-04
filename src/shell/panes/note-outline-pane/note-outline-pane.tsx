import { useShellStore } from "@/stores/shell";

export function NoteOutlinePane() {
	const openedItem = useShellStore((s) => s.openedItem);

	if (
		!openedItem ||
		openedItem.type === "source-web" ||
		openedItem.type === "source-pdf" ||
		openedItem.type === "source-image"
	) {
		return (
			<div className="flex h-full items-center justify-center p-4 text-sm text-muted-foreground">
				打开 Markdown 文件后可查看大纲
			</div>
		);
	}

	return (
		<div className="flex h-full items-center justify-center p-4 text-sm text-muted-foreground">
			大纲功能 — 编辑器中实现后可用
		</div>
	);
}
