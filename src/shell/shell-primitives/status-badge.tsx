import type { EditorSaveState } from "@/lib/types";
import { cn } from "@/lib/utils";

export function StatusBadge({
	state,
}: {
	state: EditorSaveState | "todo" | "in_progress" | "done" | "cancelled";
}) {
	const copy: Record<string, string> = {
		idle: "就绪",
		saving: "保存中",
		saved: "已保存",
		error: "保存失败",
		todo: "待办",
		in_progress: "进行中",
		done: "已完成",
		cancelled: "已取消",
	};

	return (
		<span
			className={cn(
				"inline-flex h-6 items-center rounded-full border px-2.5 text-[11px] font-medium",
				state === "saved" &&
					"border-emerald-200 bg-emerald-50 text-emerald-700",
				state === "saving" && "border-sky-200 bg-sky-50 text-sky-700",
				state === "error" && "border-red-200 bg-red-50 text-red-700",
				state === "idle" && "border-border bg-muted/70 text-muted-foreground",
				state === "todo" && "border-border bg-muted/70 text-muted-foreground",
				state === "in_progress" &&
					"border-amber-200 bg-amber-50 text-amber-700",
				state === "done" && "border-emerald-200 bg-emerald-50 text-emerald-700",
				state === "cancelled" &&
					"border-border bg-muted/70 text-muted-foreground",
			)}
		>
			{copy[state]}
		</span>
	);
}
