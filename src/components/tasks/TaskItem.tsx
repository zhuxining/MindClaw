import { Check } from "lucide-react";
import type { Task, TaskPriority } from "@/lib/types";
import { cn } from "@/lib/utils";

const PRIORITY_DOT: Record<TaskPriority, string> = {
	high: "bg-red-500",
	medium: "bg-orange-400",
	low: "bg-muted-foreground",
};

interface TaskItemProps {
	task: Task;
	onToggle: (task: Task) => void;
}

export function TaskItem({ task, onToggle }: TaskItemProps) {
	const isDone = task.status === "done";

	return (
		<div className="group flex items-center gap-2 px-3 py-1.5 hover:bg-accent/30 transition-colors">
			<button
				type="button"
				onClick={() => onToggle(task)}
				className={cn(
					"flex h-4 w-4 shrink-0 items-center justify-center rounded border transition-colors",
					isDone
						? "border-primary bg-primary text-primary-foreground"
						: "border-border hover:border-foreground",
				)}
				title={isDone ? "标记为待办" : "标记完成"}
			>
				{isDone && <Check className="h-2.5 w-2.5" />}
			</button>

			<span
				className={cn(
					"h-1.5 w-1.5 shrink-0 rounded-full",
					PRIORITY_DOT[task.priority],
				)}
			/>

			<span
				className={cn(
					"flex-1 truncate text-xs",
					isDone && "line-through text-muted-foreground",
				)}
			>
				{task.title}
			</span>

			{task.due_date && (
				<span className="shrink-0 text-xs text-muted-foreground">
					{task.due_date.slice(5)}
				</span>
			)}
		</div>
	);
}
