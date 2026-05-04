import { ArrowUpRight, Check } from "lucide-react";
import type { Task, TaskPriority } from "@/lib/types";
import { cn } from "@/lib/utils";

const PRIORITY_DOT: Record<TaskPriority, string> = {
	high: "bg-red-500",
	medium: "bg-orange-400",
	low: "bg-zinc-400",
};

interface TaskItemProps {
	task: Task;
	onToggle: (task: Task) => void;
	onOpen: (task: Task) => void;
}

export function TaskItem({ task, onToggle, onOpen }: TaskItemProps) {
	const isDone = task.status === "done";

	return (
		<div className="group mx-2 flex items-start gap-2 rounded-2xl border border-transparent px-3 py-2 transition-colors hover:border-border hover:bg-muted/60">
			<button
				type="button"
				onClick={() => onToggle(task)}
				className={cn(
					"mt-1 flex h-5 w-5 shrink-0 items-center justify-center rounded-md border transition-colors",
					isDone
						? "border-primary bg-primary text-primary-foreground"
						: "border-border hover:border-foreground",
				)}
				title={isDone ? "标记为待办" : "标记完成"}
			>
				{isDone ? <Check className="h-3 w-3" /> : null}
			</button>

			<span
				className={cn(
					"mt-2 h-2 w-2 shrink-0 rounded-full",
					PRIORITY_DOT[task.priority],
				)}
			/>

			<button
				type="button"
				onClick={() => onOpen(task)}
				className="min-w-0 flex-1 text-left"
			>
				<p
					className={cn(
						"text-sm font-medium text-foreground",
						isDone && "text-muted-foreground line-through",
					)}
				>
					{task.title}
				</p>
				<div className="mt-1 flex items-center gap-2 text-[11px] text-muted-foreground">
					{task.due_date ? <span>{task.due_date}</span> : <span>无截止日</span>}
					<span>·</span>
					<span>
						{task.status === "in_progress"
							? "进行中"
							: task.status === "todo"
								? "待办"
								: task.status === "done"
									? "已完成"
									: "已取消"}
					</span>
				</div>
			</button>

			<button
				type="button"
				onClick={() => onOpen(task)}
				className="mt-1 shrink-0 rounded-lg p-1.5 text-muted-foreground transition-colors hover:text-foreground"
				title="查看详情"
			>
				<ArrowUpRight className="h-4 w-4" />
			</button>
		</div>
	);
}
