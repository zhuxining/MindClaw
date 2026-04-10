import { NoteEditor } from "@/components/editor/NoteEditor";
import { useWorkspaceStore } from "@/stores/workspace";

export function CenterContent() {
	const openedItem = useWorkspaceStore((s) => s.openedItem);

	if (!openedItem) {
		return (
			<div className="flex h-full items-center justify-center text-sm text-muted-foreground">
				<p>从左侧选择文件以查看内容</p>
			</div>
		);
	}

	if (openedItem.type === "daily") {
		return (
			<NoteEditor
				key={openedItem.date}
				path={openedItem.path}
				date={openedItem.date}
			/>
		);
	}

	if (openedItem.type === "note") {
		return <NoteEditor key={openedItem.path} path={openedItem.path} />;
	}

	if (openedItem.type === "source") {
		if (openedItem.sourceType === "pdf") {
			return (
				<div className="flex h-full items-center justify-center text-sm text-muted-foreground">
					PDF 查看器（Phase 2）
				</div>
			);
		}
		if (openedItem.url) {
			return (
				<iframe
					src={openedItem.url}
					className="h-full w-full border-0"
					title="资源预览"
				/>
			);
		}
	}

	return null;
}
