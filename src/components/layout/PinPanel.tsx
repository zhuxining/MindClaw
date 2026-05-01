import { Pin, PinOff } from "lucide-react";
import { useWorkspaceStore } from "@/stores/workspace";
import {
	EmptyState,
	PanelAction,
	PanelFrame,
	SectionHeader,
} from "./shell-primitives";

export function PinPanel() {
	const pinnedNote = useWorkspaceStore((state) => state.pinnedNote);
	const setPinnedNote = useWorkspaceStore((state) => state.setPinnedNote);
	const openItem = useWorkspaceStore((state) => state.openItem);

	return (
		<PanelFrame className="overflow-hidden">
			<SectionHeader
				title="Pin"
				description={pinnedNote ? "当前固定参考笔记" : "固定一条长期参考的笔记"}
				actions={
					pinnedNote ? (
						<PanelAction title="取消固定" onClick={() => setPinnedNote(null)}>
							<PinOff className="h-4 w-4" />
						</PanelAction>
					) : null
				}
			/>

			<div className="min-h-0 flex-1 overflow-y-auto p-3">
				{pinnedNote ? (
					<button
						type="button"
						onClick={() =>
							openItem({
								type: "note",
								path: pinnedNote.path,
								title: pinnedNote.title,
							})
						}
						className="flex w-full items-start gap-3 rounded-2xl border border-border/70 bg-muted/40 px-3 py-3 text-left transition-colors hover:bg-muted/70"
					>
						<div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-border/70 bg-background">
							<Pin className="h-4 w-4 text-muted-foreground" />
						</div>
						<div className="min-w-0">
							<p className="truncate text-sm font-medium text-foreground">
								{pinnedNote.title}
							</p>
							<p className="truncate text-xs text-muted-foreground">
								{pinnedNote.path}
							</p>
						</div>
					</button>
				) : (
					<EmptyState
						title="还没有固定内容"
						description="在目录树或正文头部点击 Pin，可以把一条笔记挂到这里做长期参照。"
					/>
				)}
			</div>
		</PanelFrame>
	);
}
