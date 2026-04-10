import { Pin, PinOff } from "lucide-react";
import { useWorkspaceStore } from "@/stores/workspace";

export function PinPanel() {
	const pinnedNote = useWorkspaceStore((s) => s.pinnedNote);
	const setPinnedNote = useWorkspaceStore((s) => s.setPinnedNote);
	const openItem = useWorkspaceStore((s) => s.openItem);

	return (
		<div className="flex h-full flex-col border-b border-border">
			<div className="flex items-center justify-between border-b border-border px-3 py-2">
				<span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
					Pin
				</span>
				{pinnedNote && (
					<button
						type="button"
						onClick={() => setPinnedNote(null)}
						title="取消固定"
						className="rounded p-0.5 text-muted-foreground hover:text-foreground transition-colors"
					>
						<PinOff className="h-3.5 w-3.5" />
					</button>
				)}
			</div>

			<div className="flex-1 overflow-y-auto px-3 py-2">
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
						className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-sm hover:bg-accent/50 transition-colors"
					>
						<Pin className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
						<span className="truncate text-left">{pinnedNote.title}</span>
					</button>
				) : (
					<p className="text-xs text-muted-foreground">
						点击文件旁的 Pin 图标固定笔记
					</p>
				)}
			</div>
		</div>
	);
}
